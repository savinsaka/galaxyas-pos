//! Remote GPOS (eksperimental) — seperti TeamViewer, tapi hanya isi jendela GPOS.
//!
//! Dua peran, keduanya lewat relay (`relay/app.py`, rute `/remote/*`):
//!
//! - **Host** (PC yang dibantu): switch dinyalakan → menyambung ke relay dengan
//!   ID 9 digit + OTP 6 digit baru, lalu menunggu. Begitu penyambung masuk, layar
//!   GPOS ditangkap lewat DevTools Protocol milik WebView2
//!   (`Page.captureScreenshot`) dan input dari penyambung dijalankan dengan
//!   `Input.dispatchMouseEvent` / `Input.dispatchKeyEvent`. Keduanya hanya
//!   menyentuh webview GPOS: kursor Windows kasir tidak bergerak dan aplikasi
//!   lain tidak ikut tertangkap. Switch **tidak disimpan** — setiap app dibuka,
//!   remote kembali mati.
//! - **Viewer** (PC yang membantu): isi ID + OTP → frame JPEG diteruskan ke
//!   frontend lewat IPC Channel biner, input dikirim balik sebagai JSON.
//!
//! OTP sekali pakai: selesai satu sesi, host membuat OTP baru (selama switch
//! masih menyala). Laju frame diatur dengan `ack` dari viewer, jadi koneksi
//! lambat tidak menumpuk frame di mana pun.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

use crate::commands::AppState;
use crate::error::{AppError, AppResult};

const DEFAULT_RELAY: &str = "relay.jjapps.net";
const HOST_EVENT: &str = "remote-gpos://host";
const VIEWER_EVENT: &str = "remote-gpos://viewer";

/// Selang tangkap layar. Frame yang sama persis dengan sebelumnya tidak
/// dikirim, jadi layar diam hampir tidak memakan kuota.
const CAPTURE_INTERVAL: Duration = Duration::from_millis(250);
/// Kalau ack viewer tidak datang selama ini, anggap hilang dan lanjut.
const ACK_TIMEOUT: Duration = Duration::from_secs(4);
const PING_INTERVAL: Duration = Duration::from_secs(15);
const JPEG_QUALITY: u32 = 60;
const BACKOFF_SECS: [u64; 4] = [2, 5, 15, 30];

// ---------- Status ----------

#[derive(Debug, Clone, Serialize)]
pub struct HostStatus {
    /// Switch Remote di PC ini.
    pub enabled: bool,
    /// `off` | `connecting` | `waiting` | `connected` | `error`
    pub phase: String,
    pub remote_id: String,
    pub otp: String,
    pub allow_control: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ViewerStatus {
    /// `off` | `connecting` | `connected` | `closed`
    pub phase: String,
    pub remote_id: String,
    pub error: Option<String>,
}

struct HostHandle {
    stop_tx: watch::Sender<bool>,
}

struct ViewerHandle {
    stop_tx: watch::Sender<bool>,
    input_tx: mpsc::Sender<String>,
}

pub struct RemoteGposState {
    host: Mutex<Option<HostHandle>>,
    host_status: Arc<Mutex<HostStatus>>,
    allow_control: Arc<AtomicBool>,
    viewer: Mutex<Option<ViewerHandle>>,
    viewer_status: Arc<Mutex<ViewerStatus>>,
}

impl Default for RemoteGposState {
    fn default() -> Self {
        Self {
            host: Mutex::new(None),
            host_status: Arc::new(Mutex::new(HostStatus {
                enabled: false,
                phase: "off".into(),
                remote_id: String::new(),
                otp: String::new(),
                allow_control: true,
                error: None,
            })),
            allow_control: Arc::new(AtomicBool::new(true)),
            viewer: Mutex::new(None),
            viewer_status: Arc::new(Mutex::new(ViewerStatus {
                phase: "off".into(),
                remote_id: String::new(),
                error: None,
            })),
        }
    }
}

fn lock_err<T>(_: T) -> AppError {
    AppError::Other("gagal mengunci status remote".into())
}

fn update_host(app: &AppHandle, status: &Arc<Mutex<HostStatus>>, f: impl FnOnce(&mut HostStatus)) {
    let snapshot = match status.lock() {
        Ok(mut s) => {
            f(&mut s);
            s.clone()
        }
        Err(_) => return,
    };
    let _ = app.emit(HOST_EVENT, snapshot);
}

fn update_viewer(app: &AppHandle, status: &Arc<Mutex<ViewerStatus>>, f: impl FnOnce(&mut ViewerStatus)) {
    let snapshot = match status.lock() {
        Ok(mut s) => {
            f(&mut s);
            s.clone()
        }
        Err(_) => return,
    };
    let _ = app.emit(VIEWER_EVENT, snapshot);
}

// ---------- Utilitas ----------

/// Angka acak dari UUID v4 (sumber acak OS) — cukup untuk OTP/ID tanpa
/// menambah dependensi `rand`.
fn random_u64() -> u64 {
    let b = uuid::Uuid::new_v4().into_bytes();
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

fn new_otp() -> String {
    format!("{:06}", random_u64() % 1_000_000)
}

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// ID 9 digit + rahasia per-PC, dibuat sekali lalu disimpan. Rahasia yang
/// mengunci ID itu di relay supaya PC lain tidak bisa mengaku ID yang sama.
fn load_identity(conn: &rusqlite::Connection) -> AppResult<(String, String)> {
    let mut id = crate::db::get_setting(conn, "remote_gpos_id")?.unwrap_or_default();
    let mut secret = crate::db::get_setting(conn, "remote_gpos_secret")?.unwrap_or_default();
    if id.len() != 9 || !id.chars().all(|c| c.is_ascii_digit()) {
        id = format!("{}", 100_000_000 + random_u64() % 900_000_000);
        crate::db::set_setting(conn, "remote_gpos_id", &id)?;
    }
    if secret.len() < 32 {
        secret = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
        crate::db::set_setting(conn, "remote_gpos_secret", &secret)?;
    }
    Ok((id, secret))
}

/// Alamat relay dari Pengaturan Akses Online; kosong → relay bawaan.
fn relay_ws_base(conn: &rusqlite::Connection) -> AppResult<String> {
    let raw = crate::db::get_setting(conn, "relay_url")?.unwrap_or_default();
    let raw = raw.trim().trim_end_matches('/');
    let raw = if raw.is_empty() { DEFAULT_RELAY } else { raw };
    let base = if raw.starts_with("ws://") || raw.starts_with("wss://") {
        raw.to_string()
    } else if let Some(rest) = raw.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = raw.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        format!("wss://{raw}")
    };
    // Isian lama Akses Online boleh berupa URL lengkap `/agent/ws`.
    Ok(base.trim_end_matches("/agent/ws").to_string())
}

fn header(value: &str, what: &str) -> AppResult<tokio_tungstenite::tungstenite::http::HeaderValue> {
    value.parse().map_err(|_| AppError::Other(format!("{what} tidak valid")))
}

/// Pesan `{"type":"error","message":...}` dari relay, bila ada.
fn relay_error(text: &str) -> Option<String> {
    let v: Value = serde_json::from_str(text).ok()?;
    if v.get("type")?.as_str()? == "error" {
        return Some(v.get("message")?.as_str()?.to_string());
    }
    None
}

// ---------- DevTools Protocol pada webview GPOS ----------

#[cfg(windows)]
async fn cdp(app: &AppHandle, method: &str, params: Value) -> AppResult<Value> {
    use webview2_com::CallDevToolsProtocolMethodCompletedHandler;
    use windows_core::HSTRING;

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Other("jendela utama tidak ditemukan".into()))?;
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let method = method.to_string();
    let params = params.to_string();

    // Closure ini berjalan di main thread; hasilnya datang lewat handler COM
    // (juga di main thread) — tidak ada yang memblok event loop.
    window
        .with_webview(move |pw| {
            let tx_err = tx.clone();
            let result = (|| -> windows_core::Result<()> {
                let core = unsafe { pw.controller().CoreWebView2()? };
                let handler = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(
                    move |hr: windows_core::Result<()>, json: String| {
                        if let Some(tx) = tx.lock().ok().and_then(|mut g| g.take()) {
                            let _ = tx.send(hr.map(|_| json).map_err(|e| e.message()));
                        }
                        Ok(())
                    },
                ));
                unsafe {
                    core.CallDevToolsProtocolMethod(
                        &HSTRING::from(method.as_str()),
                        &HSTRING::from(params.as_str()),
                        &handler,
                    )
                }
            })();
            if let Err(e) = result {
                if let Some(tx) = tx_err.lock().ok().and_then(|mut g| g.take()) {
                    let _ = tx.send(Err(e.message()));
                }
            }
        })
        .map_err(|e| AppError::Other(format!("webview tidak bisa diakses: {e}")))?;

    let json = tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .map_err(|_| AppError::Other("webview tidak menjawab".into()))?
        .map_err(|_| AppError::Other("webview tidak menjawab".into()))?
        .map_err(|e| AppError::Other(format!("DevTools: {e}")))?;
    serde_json::from_str(&json).map_err(|e| AppError::Other(format!("DevTools: {e}")))
}

#[cfg(not(windows))]
async fn cdp(_app: &AppHandle, _method: &str, _params: Value) -> AppResult<Value> {
    Err(AppError::Other("Remote GPOS hanya tersedia di Windows".into()))
}

/// Ukuran viewport dalam piksel CSS — koordinat input dari viewer memakai
/// satuan ini.
async fn viewport(app: &AppHandle) -> AppResult<(f64, f64)> {
    let m = cdp(app, "Page.getLayoutMetrics", json!({})).await?;
    let vp = m.get("cssVisualViewport").or_else(|| m.get("visualViewport"));
    let w = vp.and_then(|v| v.get("clientWidth")).and_then(Value::as_f64).unwrap_or(0.0);
    let h = vp.and_then(|v| v.get("clientHeight")).and_then(Value::as_f64).unwrap_or(0.0);
    if w <= 0.0 || h <= 0.0 {
        return Err(AppError::Other("ukuran layar GPOS tidak terbaca".into()));
    }
    Ok((w, h))
}

/// Tangkap layar GPOS sebagai JPEG seukuran piksel CSS (layar HiDPI tidak
/// dikirim dalam resolusi penuh — hemat kuota).
async fn capture(app: &AppHandle, w: f64, h: f64) -> AppResult<Vec<u8>> {
    use base64::Engine as _;
    let dpr = app
        .get_webview_window("main")
        .and_then(|win| win.scale_factor().ok())
        .unwrap_or(1.0)
        .max(1.0);
    let shot = cdp(
        app,
        "Page.captureScreenshot",
        json!({
            "format": "jpeg",
            "quality": JPEG_QUALITY,
            "optimizeForSpeed": true,
            "clip": { "x": 0, "y": 0, "width": w, "height": h, "scale": 1.0 / dpr },
        }),
    )
    .await?;
    let data = shot
        .get("data")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Other("tangkapan layar kosong".into()))?;
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| AppError::Other(format!("tangkapan layar rusak: {e}")))
}

/// Jalankan satu input dari viewer. Hanya dua metode DevTools yang boleh
/// dipanggil dari luar; parameternya diteruskan apa adanya dan divalidasi
/// oleh WebView2 sendiri.
async fn dispatch_input(app: &AppHandle, msg: &Value) {
    let method = match msg.get("type").and_then(Value::as_str) {
        Some("mouse") => "Input.dispatchMouseEvent",
        Some("key") => "Input.dispatchKeyEvent",
        _ => return,
    };
    let Some(params) = msg.get("params").filter(|p| p.is_object()) else { return };
    let _ = cdp(app, method, params.clone()).await;
}

// ---------- Host ----------

fn start_host(app: AppHandle, state: &RemoteGposState) -> AppResult<()> {
    let conn = app.state::<AppState>().conn.clone();
    let (remote_id, secret, base) = {
        let c = conn.lock().map_err(lock_err)?;
        let (id, secret) = load_identity(&c)?;
        (id, secret, relay_ws_base(&c)?)
    };

    let (stop_tx, mut stop_rx) = watch::channel(false);
    let status = state.host_status.clone();
    let allow = state.allow_control.clone();
    update_host(&app, &status, |s| {
        s.enabled = true;
        s.phase = "connecting".into();
        s.remote_id = remote_id.clone();
        s.otp.clear();
        s.error = None;
    });

    tauri::async_runtime::spawn(async move {
        let mut attempt = 0usize;
        while !*stop_rx.borrow() {
            let otp = new_otp();
            match host_session(&app, &base, &remote_id, &secret, &otp, &status, &allow, &mut stop_rx).await {
                Ok(()) => attempt = 0,
                Err(e) => {
                    update_host(&app, &status, |s| {
                        s.phase = "error".into();
                        s.otp.clear();
                        s.error = Some(e.to_string());
                    });
                    attempt = (attempt + 1).min(BACKOFF_SECS.len() - 1);
                }
            }
            if *stop_rx.borrow() {
                break;
            }
            let pause = if attempt == 0 { 1 } else { BACKOFF_SECS[attempt] };
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(pause)) => {}
                _ = stop_rx.changed() => {}
            }
        }
        update_host(&app, &status, |s| {
            s.enabled = false;
            s.phase = "off".into();
            s.otp.clear();
            s.error = None;
        });
    });

    *state.host.lock().map_err(lock_err)? = Some(HostHandle { stop_tx });
    Ok(())
}

/// Satu sesi host: tunggu penyambung, layani sampai putus. `Ok` = selesai wajar
/// (penyambung keluar / waktu tunggu habis) → OTP baru dibuat.
#[allow(clippy::too_many_arguments)]
async fn host_session(
    app: &AppHandle,
    base: &str,
    remote_id: &str,
    secret: &str,
    otp: &str,
    status: &Arc<Mutex<HostStatus>>,
    allow: &Arc<AtomicBool>,
    stop_rx: &mut watch::Receiver<bool>,
) -> AppResult<()> {
    let mut request = format!("{base}/remote/host")
        .into_client_request()
        .map_err(|e| AppError::Other(format!("URL relay tidak valid: {e}")))?;
    let h = request.headers_mut();
    h.insert("X-Remote-Id", header(remote_id, "ID remote")?);
    h.insert("X-Remote-Secret", header(secret, "rahasia remote")?);
    h.insert("X-Remote-Otp-Hash", header(&sha256_hex(otp), "OTP")?);

    let (ws, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| AppError::Other(format!("gagal menyambung ke relay: {e}")))?;
    let (mut writer, mut reader) = ws.split();

    let mut ping = tokio::time::interval(PING_INTERVAL);
    ping.tick().await;
    let mut tick = tokio::time::interval(CAPTURE_INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut viewing = false;
    let mut last_frame: Vec<u8> = Vec::new();
    let mut awaiting_ack: Option<Instant> = None;
    let mut last_meta: Option<(f64, f64, bool)> = None;

    let result = loop {
        tokio::select! {
            _ = stop_rx.changed() => break Ok(()),
            _ = ping.tick() => {
                if writer.send(Message::Text(r#"{"type":"ping"}"#.into())).await.is_err() {
                    break Ok(());
                }
            }
            _ = tick.tick(), if viewing => {
                if let Some(sent) = awaiting_ack {
                    if sent.elapsed() < ACK_TIMEOUT {
                        continue;
                    }
                }
                let Ok((w, h)) = viewport(app).await else { continue };
                let control = allow.load(Ordering::Relaxed);
                if last_meta != Some((w, h, control)) {
                    last_meta = Some((w, h, control));
                    let meta = json!({ "type": "meta", "width": w, "height": h, "control": control });
                    if writer.send(Message::Text(meta.to_string())).await.is_err() {
                        break Ok(());
                    }
                }
                let Ok(frame) = capture(app, w, h).await else { continue };
                if frame == last_frame {
                    continue;
                }
                if writer.send(Message::Binary(frame.clone())).await.is_err() {
                    break Ok(());
                }
                last_frame = frame;
                awaiting_ack = Some(Instant::now());
            }
            incoming = reader.next() => {
                let Some(Ok(message)) = incoming else { break Ok(()) };
                let text = match message {
                    Message::Text(t) => t,
                    Message::Close(_) => break Ok(()),
                    _ => continue,
                };
                if let Some(err) = relay_error(&text) {
                    break Err(AppError::Other(err));
                }
                let Ok(msg) = serde_json::from_str::<Value>(&text) else { continue };
                match msg.get("type").and_then(Value::as_str) {
                    Some("ready") => update_host(app, status, |s| {
                        s.phase = "waiting".into();
                        s.otp = otp.to_string();
                        s.error = None;
                    }),
                    Some("viewer_joined") => {
                        viewing = true;
                        update_host(app, status, |s| {
                            s.phase = "connected".into();
                            // OTP sudah terpakai; jangan dipajang lagi.
                            s.otp.clear();
                        });
                    }
                    Some("ack") => awaiting_ack = None,
                    // Viewer minta frame penuh (mis. baru selesai resize).
                    Some("refresh") => {
                        last_frame.clear();
                        last_meta = None;
                    }
                    Some("mouse") | Some("key") if viewing && allow.load(Ordering::Relaxed) => {
                        dispatch_input(app, &msg).await;
                    }
                    _ => {}
                }
            }
        }
    };
    let _ = writer.close().await;
    result
}

// ---------- Viewer ----------

async fn viewer_session(
    app: &AppHandle,
    base: &str,
    remote_id: &str,
    otp: &str,
    frames: &Channel<InvokeResponseBody>,
    status: &Arc<Mutex<ViewerStatus>>,
    input_rx: &mut mpsc::Receiver<String>,
    stop_rx: &mut watch::Receiver<bool>,
) -> AppResult<()> {
    let mut request = format!("{base}/remote/view")
        .into_client_request()
        .map_err(|e| AppError::Other(format!("URL relay tidak valid: {e}")))?;
    let h = request.headers_mut();
    h.insert("X-Remote-Id", header(remote_id, "ID remote")?);
    h.insert("X-Remote-Otp", header(otp, "OTP")?);

    let (ws, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| AppError::Other(format!("gagal menyambung ke relay: {e}")))?;
    let (mut writer, mut reader) = ws.split();
    let mut ping = tokio::time::interval(PING_INTERVAL);
    ping.tick().await;

    loop {
        tokio::select! {
            _ = stop_rx.changed() => break,
            _ = ping.tick() => {
                if writer.send(Message::Text(r#"{"type":"ping"}"#.into())).await.is_err() {
                    break;
                }
            }
            outgoing = input_rx.recv() => {
                let Some(text) = outgoing else { break };
                if writer.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
            incoming = reader.next() => {
                let Some(Ok(message)) = incoming else { break };
                match message {
                    Message::Binary(bytes) => {
                        let _ = frames.send(InvokeResponseBody::Raw(bytes));
                    }
                    Message::Text(text) => {
                        if let Some(err) = relay_error(&text) {
                            let _ = writer.close().await;
                            return Err(AppError::Other(err));
                        }
                        let Ok(msg) = serde_json::from_str::<Value>(&text) else { continue };
                        match msg.get("type").and_then(Value::as_str) {
                            Some("joined") => update_viewer(app, status, |s| {
                                s.phase = "connected".into();
                                s.error = None;
                            }),
                            // Metadata (ukuran layar, izin kontrol) diteruskan ke
                            // frontend sebagai event.
                            Some("meta") => {
                                let _ = app.emit("remote-gpos://meta", msg);
                            }
                            _ => {}
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        }
    }
    let _ = writer.close().await;
    Ok(())
}

// ---------- Command ----------

#[tauri::command]
pub async fn remote_gpos_status(
    state: State<'_, RemoteGposState>,
) -> AppResult<(HostStatus, ViewerStatus)> {
    let host = state.host_status.lock().map_err(lock_err)?.clone();
    let viewer = state.viewer_status.lock().map_err(lock_err)?.clone();
    Ok((host, viewer))
}

/// Switch utama: nyala = langsung dapat ID + OTP; mati = sesi diputus.
#[tauri::command]
pub async fn remote_gpos_set_host(
    app: AppHandle,
    state: State<'_, RemoteGposState>,
    enabled: bool,
) -> AppResult<HostStatus> {
    if let Some(old) = state.host.lock().map_err(lock_err)?.take() {
        let _ = old.stop_tx.send(true);
    }
    if enabled {
        start_host(app, &state)?;
    }
    let s = state.host_status.lock().map_err(lock_err)?.clone();
    Ok(s)
}

/// Kontrol penuh (true) atau hanya lihat (false). Berlaku seketika, juga di
/// tengah sesi. Pilihan terakhir diingat supaya tidak perlu dipilih ulang.
#[tauri::command]
pub async fn remote_gpos_set_control(
    app: AppHandle,
    state: State<'_, RemoteGposState>,
    app_state: State<'_, AppState>,
    allow: bool,
) -> AppResult<()> {
    state.allow_control.store(allow, Ordering::Relaxed);
    if let Ok(c) = app_state.conn.lock() {
        let _ = crate::db::set_setting(&c, "remote_gpos_allow_control", if allow { "1" } else { "0" });
    }
    update_host(&app, &state.host_status, |s| s.allow_control = allow);
    Ok(())
}

/// Dipanggil sekali saat startup: pulihkan pilihan kontrol/lihat.
pub fn load_preferences(state: &RemoteGposState, conn: &rusqlite::Connection) {
    let allow = crate::db::get_setting(conn, "remote_gpos_allow_control")
        .ok()
        .flatten()
        .as_deref()
        != Some("0");
    state.allow_control.store(allow, Ordering::Relaxed);
    if let Ok(mut s) = state.host_status.lock() {
        s.allow_control = allow;
        s.remote_id = load_identity(conn).map(|(id, _)| id).unwrap_or_default();
    }
}

#[tauri::command]
pub async fn remote_gpos_connect(
    app: AppHandle,
    state: State<'_, RemoteGposState>,
    app_state: State<'_, AppState>,
    remote_id: String,
    otp: String,
    frames: Channel<InvokeResponseBody>,
) -> AppResult<()> {
    let remote_id: String = remote_id.chars().filter(|c| c.is_ascii_digit()).collect();
    let otp: String = otp.chars().filter(|c| c.is_ascii_digit()).collect();
    if remote_id.len() != 9 {
        return Err(AppError::Other("ID remote harus 9 angka.".into()));
    }
    if otp.len() != 6 {
        return Err(AppError::Other("OTP harus 6 angka.".into()));
    }
    let base = {
        let c = app_state.conn.lock().map_err(lock_err)?;
        if load_identity(&c).map(|(id, _)| id == remote_id).unwrap_or(false) {
            return Err(AppError::Other("Itu ID PC ini sendiri.".into()));
        }
        relay_ws_base(&c)?
    };

    if let Some(old) = state.viewer.lock().map_err(lock_err)?.take() {
        let _ = old.stop_tx.send(true);
    }
    let (stop_tx, mut stop_rx) = watch::channel(false);
    let (input_tx, mut input_rx) = mpsc::channel::<String>(256);
    *state.viewer.lock().map_err(lock_err)? = Some(ViewerHandle { stop_tx, input_tx });

    let status = state.viewer_status.clone();
    update_viewer(&app, &status, |s| {
        s.phase = "connecting".into();
        s.remote_id = remote_id.clone();
        s.error = None;
    });

    tauri::async_runtime::spawn(async move {
        let result =
            viewer_session(&app, &base, &remote_id, &otp, &frames, &status, &mut input_rx, &mut stop_rx).await;
        update_viewer(&app, &status, |s| {
            s.phase = "closed".into();
            s.error = result.err().map(|e| e.to_string());
        });
    });
    Ok(())
}

/// Input/ack dari layar viewer. Sengaja tidak menunggu apa pun — kalau antrian
/// penuh (koneksi macet), gerakan mouse dibuang saja.
#[tauri::command]
pub async fn remote_gpos_send(state: State<'_, RemoteGposState>, message: Value) -> AppResult<()> {
    let tx = state.viewer.lock().map_err(lock_err)?.as_ref().map(|v| v.input_tx.clone());
    if let Some(tx) = tx {
        let _ = tx.try_send(message.to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn remote_gpos_disconnect(state: State<'_, RemoteGposState>) -> AppResult<()> {
    if let Some(old) = state.viewer.lock().map_err(lock_err)?.take() {
        let _ = old.stop_tx.send(true);
    }
    Ok(())
}
