//! Chat antar toko + Push Alert (server: `backend/app/routers/chat.py`).
//!
//! Identitas = kode toko (sama dengan `store_id` sync) + Kunci Chat dari
//! jjapps.net/admin. Setelah diatur, PC ini memegang satu WebSocket ke server
//! dan menyambung ulang sendiri. Pesan masuk diteruskan ke frontend lewat event
//! `chat://message`; yang memunculkan popup hanya pesan `push` (lihat
//! `ChatAlert.svelte`) — pesan biasa diam saja.
//!
//! File tidak disimpan di server: isinya datang sebagai frame biner tepat
//! setelah kepala pesannya, lalu ditulis ke `Documents\GPOS Chat\<kode toko>\`.
//! File tidak pernah dibuka otomatis.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

use crate::commands::AppState;
use crate::error::{AppError, AppResult};

const DEFAULT_SERVER: &str = "https://jjapps.net";
const PING_INTERVAL: Duration = Duration::from_secs(20);
const BACKOFF_SECS: [u64; 5] = [2, 5, 15, 30, 60];
pub const MAX_FILE: usize = 10 * 1024 * 1024;
/// Voice note (rekaman opus dari MediaRecorder), maks 2 menit — sama dengan server.
pub const MAX_VOICE: usize = 2 * 1024 * 1024;
pub const VOICE_EXT: &[&str] = &["webm", "ogg"];
const MAX_TEXT: usize = 2000;

/// Sama persis dengan `ALLOWED_EXT` di server. Sengaja TIDAK: svg, docm, xlsm.
pub const ALLOWED_EXT: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "heic", "heif", "tif", "tiff", "doc", "docx",
    "xls", "xlsx", "csv", "pdf",
];

#[derive(Debug, Clone, Serialize, Default)]
pub struct ChatStatus {
    pub configured: bool,
    pub connected: bool,
    pub code: String,
    pub name: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContact {
    pub code: String,
    pub name: String,
}

enum Outgoing {
    Text(String),
    File(String, Vec<u8>),
}

struct ChatHandle {
    stop_tx: watch::Sender<bool>,
    out_tx: mpsc::Sender<Outgoing>,
}

#[derive(Default)]
pub struct ChatState {
    handle: Mutex<Option<ChatHandle>>,
    status: Arc<Mutex<ChatStatus>>,
}

#[derive(Clone)]
struct ChatConfig {
    server: String,
    code: String,
    key: String,
}

fn lock_err<T>(_: T) -> AppError {
    AppError::Other("gagal mengunci status chat".into())
}

fn set_status(app: &AppHandle, status: &Arc<Mutex<ChatStatus>>, f: impl FnOnce(&mut ChatStatus)) {
    let snap = match status.lock() {
        Ok(mut s) => {
            f(&mut s);
            s.clone()
        }
        Err(_) => return,
    };
    let _ = app.emit("chat://status", snap);
}

fn load_config(conn: &rusqlite::Connection) -> AppResult<Option<ChatConfig>> {
    let get = |k: &str| -> AppResult<String> {
        Ok(crate::db::get_setting(conn, k)?.unwrap_or_default().trim().to_string())
    };
    let code = get("chat_store_code")?;
    let key = get("chat_key")?;
    if code.is_empty() || key.is_empty() {
        return Ok(None);
    }
    let mut server = get("server_url")?;
    if server.is_empty() {
        server = DEFAULT_SERVER.to_string();
    }
    Ok(Some(ChatConfig { server: server.trim_end_matches('/').to_string(), code, key }))
}

fn ws_url(server: &str) -> String {
    let base = if let Some(rest) = server.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = server.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        format!("wss://{server}")
    };
    format!("{base}/api/v1/chat/ws")
}

fn http_base(server: &str) -> String {
    if server.starts_with("http://") || server.starts_with("https://") {
        server.to_string()
    } else {
        format!("https://{server}")
    }
}

/// Ekstensi voice note — hanya sah untuk pesan `kind: "voice"`.
pub fn voice_ext_allowed(name: &str) -> bool {
    name.rsplit_once('.')
        .map(|(_, ext)| VOICE_EXT.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Pesan yang membawa isi (file / voice note).
fn has_payload(msg: &Value) -> bool {
    matches!(msg.get("kind").and_then(Value::as_str), Some("file") | Some("voice"))
}

pub fn ext_allowed(name: &str) -> bool {
    name.rsplit_once('.')
        .map(|(_, ext)| ALLOWED_EXT.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Nama aman untuk file/folder Windows.
fn safe_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let cleaned: String = base
        .chars()
        .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') || c.is_control() { '_' } else { c })
        .collect();
    let cleaned = cleaned.trim_matches(|c| c == ' ' || c == '.').to_string();
    if cleaned.is_empty() {
        "file".into()
    } else {
        cleaned.chars().take(200).collect()
    }
}

fn chat_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let docs = app
        .path()
        .document_dir()
        .map_err(|e| AppError::Other(format!("folder Documents tidak ditemukan: {e}")))?;
    Ok(docs.join("GPOS Chat"))
}

/// Letak file sebuah pesan: `GPOS Chat\<kode lawan bicara>\<id>-<nama>`. Id
/// pesan ikut di nama supaya dua file bernama sama tidak saling menimpa, dan
/// riwayat bisa menemukan file-nya lagi tanpa catatan tambahan. File yang
/// DIKIRIM dari PC ini juga disimpan salinannya (supaya voice note sendiri bisa
/// diputar ulang), di folder toko tujuan.
fn file_path(app: &AppHandle, peer: &str, id: i64, name: &str) -> AppResult<PathBuf> {
    Ok(chat_dir(app)?.join(safe_name(peer)).join(format!("{id}-{}", safe_name(name))))
}

fn peer_of(msg: &Value, me: &str) -> Option<String> {
    let from = msg.get("from").and_then(Value::as_str)?;
    let to = msg.get("to").and_then(Value::as_str)?;
    Some(if to == me { from.to_string() } else { to.to_string() })
}

/// Tempel `local_path` ke pesan file/voice yang file-nya ada di PC ini.
fn attach_local_path(app: &AppHandle, me: &str, msg: &mut Value) {
    if !has_payload(msg) {
        return;
    }
    let (Some(peer), Some(id), Some(name)) = (
        peer_of(msg, me),
        msg.get("id").and_then(Value::as_i64),
        msg.get("file_name").and_then(Value::as_str),
    ) else {
        return;
    };
    if let Ok(path) = file_path(app, &peer, id, name) {
        if path.exists() {
            msg["local_path"] = json!(path.to_string_lossy());
        }
    }
}

fn write_payload(app: &AppHandle, me: &str, msg: &Value, bytes: &[u8]) -> AppResult<()> {
    let peer = peer_of(msg, me).unwrap_or_else(|| "lain".into());
    let id = msg.get("id").and_then(Value::as_i64).unwrap_or(0);
    let name = msg.get("file_name").and_then(Value::as_str).unwrap_or("file");
    let path = file_path(app, &peer, id, name)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, bytes)?;
    Ok(())
}

// ---------- Loop koneksi ----------

fn start(app: AppHandle, state: &ChatState, config: ChatConfig) -> AppResult<()> {
    if let Some(old) = state.handle.lock().map_err(lock_err)?.take() {
        let _ = old.stop_tx.send(true);
    }
    let (stop_tx, mut stop_rx) = watch::channel(false);
    let (out_tx, mut out_rx) = mpsc::channel::<Outgoing>(64);
    let status = state.status.clone();
    set_status(&app, &status, |s| {
        s.configured = true;
        s.connected = false;
        s.code = config.code.clone();
        s.error = None;
    });

    tauri::async_runtime::spawn(async move {
        let mut attempt = 0usize;
        while !*stop_rx.borrow() {
            match session(&app, &config, &status, &mut out_rx, &mut stop_rx).await {
                Ok(()) => attempt = 0,
                Err((e, fatal)) => {
                    set_status(&app, &status, |s| {
                        s.connected = false;
                        s.error = Some(e.to_string());
                    });
                    if fatal {
                        // Kunci salah/diganti: menyambung ulang tidak ada gunanya
                        // sampai kunci baru dimasukkan.
                        break;
                    }
                    attempt = (attempt + 1).min(BACKOFF_SECS.len() - 1);
                }
            }
            set_status(&app, &status, |s| s.connected = false);
            if *stop_rx.borrow() {
                break;
            }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(BACKOFF_SECS[attempt])) => {}
                _ = stop_rx.changed() => {}
            }
        }
    });

    *state.handle.lock().map_err(lock_err)? = Some(ChatHandle { stop_tx, out_tx });
    Ok(())
}

/// `Err((pesan, fatal))` — fatal = kunci ditolak server.
async fn session(
    app: &AppHandle,
    config: &ChatConfig,
    status: &Arc<Mutex<ChatStatus>>,
    out_rx: &mut mpsc::Receiver<Outgoing>,
    stop_rx: &mut watch::Receiver<bool>,
) -> Result<(), (AppError, bool)> {
    let err = |e: String| (AppError::Other(e), false);
    let mut request = ws_url(&config.server)
        .into_client_request()
        .map_err(|e| err(format!("alamat server tidak valid: {e}")))?;
    let h = request.headers_mut();
    h.insert("X-Chat-Store", config.code.parse().map_err(|_| err("kode toko tidak valid".into()))?);
    h.insert("X-Chat-Key", config.key.parse().map_err(|_| err("Kunci Chat tidak valid".into()))?);

    let (ws, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| err(format!("gagal menyambung ke server chat: {e}")))?;
    let (mut writer, mut reader) = ws.split();
    let mut ping = tokio::time::interval(PING_INTERVAL);
    ping.tick().await;

    // Kepala pesan file yang sedang menunggu frame biner isinya.
    let mut pending_file: Option<Value> = None;
    // Isi file/voice yang baru DIKIRIM, menunggu konfirmasi server (yang
    // membawa id pesan) supaya salinannya bisa disimpan di PC pengirim.
    let mut sent_payloads: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();

    // Buang kiriman yang sempat masuk antrean saat koneksi sebelumnya putus —
    // di layar sudah ditandai gagal (✕), jadi tidak boleh terkirim sendiri.
    while out_rx.try_recv().is_ok() {}

    loop {
        tokio::select! {
            _ = stop_rx.changed() => break,
            _ = ping.tick() => {
                if writer.send(Message::Text(r#"{"type":"ping"}"#.into())).await.is_err() {
                    break;
                }
            }
            outgoing = out_rx.recv() => {
                let Some(out) = outgoing else { break };
                let ok = match out {
                    Outgoing::Text(t) => writer.send(Message::Text(t)).await.is_ok(),
                    Outgoing::File(head, bytes) => {
                        let client_id = serde_json::from_str::<Value>(&head)
                            .ok()
                            .and_then(|h| h.get("client_id").and_then(Value::as_str).map(str::to_string));
                        let ok = writer.send(Message::Text(head)).await.is_ok()
                            && writer.send(Message::Binary(bytes.clone())).await.is_ok();
                        if let (true, Some(id)) = (ok, client_id) {
                            sent_payloads.insert(id, bytes);
                        }
                        ok
                    }
                };
                if !ok {
                    break;
                }
            }
            incoming = reader.next() => {
                let Some(Ok(message)) = incoming else { break };
                match message {
                    Message::Binary(bytes) => {
                        if let Some(mut msg) = pending_file.take() {
                            save_incoming_file(app, &config.code, &mut msg, &bytes);
                            let _ = app.emit("chat://message", msg);
                        }
                    }
                    Message::Text(text) => {
                        let Ok(v) = serde_json::from_str::<Value>(&text) else { continue };
                        match v.get("type").and_then(Value::as_str) {
                            Some("hello") => set_status(app, status, |s| {
                                s.connected = true;
                                s.error = None;
                                s.name = v.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
                            }),
                            Some("message") => {
                                let Some(mut msg) = v.get("message").cloned() else { continue };
                                let is_incoming_file = has_payload(&msg)
                                    && msg.get("to").and_then(Value::as_str) == Some(config.code.as_str());
                                if is_incoming_file {
                                    // Isi file menyusul di frame berikutnya.
                                    pending_file = Some(msg);
                                } else {
                                    // Konfirmasi file/voice yang dikirim dari PC ini:
                                    // simpan salinannya sekarang karena id sudah ada.
                                    let own = msg.get("client_id").and_then(Value::as_str)
                                        .and_then(|c| sent_payloads.remove(c));
                                    if let Some(bytes) = own {
                                        if write_payload(app, &config.code, &msg, &bytes).is_ok() {
                                            attach_local_path(app, &config.code, &mut msg);
                                        }
                                    }
                                    let _ = app.emit("chat://message", msg);
                                }
                            }
                            // Perubahan centang (diterima/dibaca).
                            Some("receipt") => {
                                let _ = app.emit("chat://receipt", v);
                            }
                            Some("error") => {
                                if v.get("fatal").and_then(Value::as_bool) == Some(true) {
                                    let m = v.get("message").and_then(Value::as_str).unwrap_or("ditolak server");
                                    let _ = writer.close().await;
                                    return Err((AppError::Other(m.to_string()), true));
                                }
                                if let Some(c) = v.get("client_id").and_then(Value::as_str) {
                                    sent_payloads.remove(c);
                                }
                                let _ = app.emit("chat://error", v);
                            }
                            _ => {}
                        }
                    }
                    Message::Close(frame) => {
                        // 4401 = kunci diganti/toko dihapus dari panel admin.
                        let code: u16 = frame.map(|f| f.code.into()).unwrap_or(1000);
                        if code == 4401 {
                            return Err((AppError::Other("Kunci Chat sudah diganti atau toko dihapus dari chat. Masukkan kunci baru.".into()), true));
                        }
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
    let _ = writer.close().await;
    Ok(())
}

fn save_incoming_file(app: &AppHandle, me: &str, msg: &mut Value, bytes: &[u8]) {
    let name = msg.get("file_name").and_then(Value::as_str).unwrap_or("file").to_string();
    let voice = msg.get("kind").and_then(Value::as_str) == Some("voice");
    // Dicek lagi di sini walau server sudah menyaring.
    let allowed = if voice {
        voice_ext_allowed(&name) && bytes.len() <= MAX_VOICE
    } else {
        ext_allowed(&name) && bytes.len() <= MAX_FILE
    };
    if !allowed {
        msg["save_error"] = json!("Jenis/ukuran file tidak diizinkan — tidak disimpan.");
        return;
    }
    match write_payload(app, me, msg, bytes) {
        Ok(()) => attach_local_path(app, me, msg),
        Err(e) => msg["save_error"] = json!(format!("Gagal menyimpan file: {e}")),
    }
}

/// Dipanggil saat app dibuka: sambungkan chat bila sudah diatur.
pub fn autostart(app: &AppHandle) {
    let config = {
        let state = app.state::<AppState>();
        let guard = state.conn.lock();
        guard.ok().and_then(|c| load_config(&c).ok().flatten())
    };
    if let Some(config) = config {
        let chat = app.state::<ChatState>();
        let _ = start(app.clone(), &chat, config);
    }
}

// ---------- Command ----------

fn config_of(app_state: &State<'_, AppState>) -> AppResult<ChatConfig> {
    let c = app_state.conn.lock().map_err(lock_err)?;
    load_config(&c)?.ok_or_else(|| AppError::Other("Chat belum diatur.".into()))
}

/// Pengirim ke socket — HANYA saat benar-benar tersambung. Pesan tidak
/// pernah diantre untuk dikirim otomatis nanti: pemilik ingin pesan yang gagal
/// ditandai ✕ lalu dikirim ulang manual.
fn sender(state: &State<'_, ChatState>) -> AppResult<mpsc::Sender<Outgoing>> {
    let connected = state.status.lock().map(|s| s.connected).unwrap_or(false);
    if !connected {
        return Err(AppError::Other("PC ini sedang offline — pesan tidak terkirim.".into()));
    }
    state
        .handle
        .lock()
        .map_err(lock_err)?
        .as_ref()
        .map(|h| h.out_tx.clone())
        .ok_or_else(|| AppError::Other("Chat belum tersambung.".into()))
}

#[tauri::command]
pub async fn chat_status(state: State<'_, ChatState>, app_state: State<'_, AppState>) -> AppResult<Value> {
    let status = state.status.lock().map_err(lock_err)?.clone();
    // Saran kode toko ini = Store ID sync, supaya form awal terisi sendiri.
    let suggested = {
        let c = app_state.conn.lock().map_err(lock_err)?;
        crate::db::get_setting(&c, "store_id")?.unwrap_or_default()
    };
    Ok(json!({ "status": status, "suggested_code": suggested }))
}

#[tauri::command]
pub async fn chat_setup(
    app: AppHandle,
    state: State<'_, ChatState>,
    app_state: State<'_, AppState>,
    code: String,
    key: String,
) -> AppResult<ChatStatus> {
    let code = code.trim().to_string();
    let key = key.trim().to_string();
    if code.is_empty() || key.is_empty() {
        return Err(AppError::Other("Kode toko dan Kunci Chat wajib diisi.".into()));
    }
    let config = {
        let c = app_state.conn.lock().map_err(lock_err)?;
        crate::db::set_setting(&c, "chat_store_code", &code)?;
        crate::db::set_setting(&c, "chat_key", &key)?;
        load_config(&c)?.ok_or_else(|| AppError::Other("Chat belum diatur.".into()))?
    };
    start(app, &state, config)?;
    let s = state.status.lock().map_err(lock_err)?.clone();
    Ok(s)
}

#[tauri::command]
pub async fn chat_logout(
    app: AppHandle,
    state: State<'_, ChatState>,
    app_state: State<'_, AppState>,
) -> AppResult<()> {
    if let Some(old) = state.handle.lock().map_err(lock_err)?.take() {
        let _ = old.stop_tx.send(true);
    }
    {
        let c = app_state.conn.lock().map_err(lock_err)?;
        crate::db::set_setting(&c, "chat_key", "")?;
    }
    set_status(&app, &state.status, |s| *s = ChatStatus::default());
    Ok(())
}

async fn rest_get(config: &ChatConfig, path: &str, query: &[(&str, &str)]) -> AppResult<Value> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(20)).build()?;
    let resp = client
        .get(format!("{}/api/v1/chat/{path}", http_base(&config.server)))
        .query(query)
        .header("X-Chat-Store", &config.code)
        .header("X-Chat-Key", &config.key)
        .send()
        .await?;
    let ok = resp.status().is_success();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    if !ok {
        let msg = body.get("error").and_then(Value::as_str).unwrap_or("permintaan chat ditolak server");
        return Err(AppError::Other(msg.to_string()));
    }
    Ok(body)
}

/// Riwayat 7 hari semua percakapan toko ini.
#[tauri::command]
pub async fn chat_history(app: AppHandle, app_state: State<'_, AppState>) -> AppResult<Vec<Value>> {
    let config = config_of(&app_state)?;
    let body = rest_get(&config, "messages", &[]).await?;
    let mut messages = body.get("messages").and_then(Value::as_array).cloned().unwrap_or_default();
    for m in messages.iter_mut() {
        attach_local_path(&app, &config.code, m);
    }
    Ok(messages)
}

#[tauri::command]
pub async fn chat_lookup(app_state: State<'_, AppState>, code: String) -> AppResult<Value> {
    let config = config_of(&app_state)?;
    rest_get(&config, "lookup", &[("code", code.trim())]).await
}

#[tauri::command]
pub async fn chat_send(
    state: State<'_, ChatState>,
    to: String,
    body: String,
    push: bool,
    client_id: Option<String>,
) -> AppResult<String> {
    let body = body.trim().to_string();
    if body.is_empty() {
        return Err(AppError::Other("Pesan kosong.".into()));
    }
    if body.chars().count() > MAX_TEXT {
        return Err(AppError::Other(format!("Pesan terlalu panjang (maks {MAX_TEXT} karakter).")));
    }
    let client_id = client_id.filter(|c| !c.is_empty()).unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string());
    let msg = json!({ "type": "send", "to": to.trim(), "body": body, "push": push, "client_id": client_id });
    sender(&state)?
        .send(Outgoing::Text(msg.to_string()))
        .await
        .map_err(|_| AppError::Other("Chat belum tersambung.".into()))?;
    Ok(client_id)
}

/// Tandai pesan dari `peer` sampai id `up_to` sudah dibaca (✓✓ biru di
/// pengirim). Dipanggil frontend hanya saat percakapan benar-benar terlihat.
#[tauri::command]
pub async fn chat_mark_read(state: State<'_, ChatState>, peer: String, up_to: i64) -> AppResult<()> {
    let msg = json!({ "type": "read", "peer": peer.trim(), "up_to": up_to });
    if let Ok(tx) = sender(&state) {
        let _ = tx.try_send(Outgoing::Text(msg.to_string()));
    }
    Ok(())
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Isi file dikirim sebagai body mentah (bukan array JSON 10 juta angka);
/// tujuan, nama (di-URL-encode), dan flag alert lewat header.
#[tauri::command]
pub async fn chat_send_file(state: State<'_, ChatState>, request: tauri::ipc::Request<'_>) -> AppResult<String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(AppError::Other("isi file tidak terbaca".into()));
    };
    let header = |k: &str| request.headers().get(k).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let to = header("x-chat-to");
    let name = safe_name(&percent_decode(&header("x-chat-name")));
    let push = header("x-chat-push") == "1";
    let voice = header("x-chat-voice") == "1";
    let duration: f64 = header("x-chat-duration").parse().unwrap_or(0.0);
    if to.is_empty() {
        return Err(AppError::Other("Pilih kontak tujuan dulu.".into()));
    }
    if voice {
        if !voice_ext_allowed(&name) {
            return Err(AppError::Other("Format voice note tidak dikenal.".into()));
        }
        if bytes.len() > MAX_VOICE {
            return Err(AppError::Other("Voice note terlalu panjang.".into()));
        }
    } else {
        if !ext_allowed(&name) {
            return Err(AppError::Other("Jenis file tidak diizinkan. Hanya gambar, Word, Excel, dan PDF.".into()));
        }
        if bytes.len() > MAX_FILE {
            return Err(AppError::Other("File terlalu besar (maks 10 MB).".into()));
        }
    }
    let client_id = Some(header("x-chat-client-id"))
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string());
    let head = json!({
        "type": "file", "to": to, "name": name, "push": push, "client_id": client_id,
        "voice": voice, "duration": duration,
    });
    sender(&state)?
        .send(Outgoing::File(head.to_string(), bytes.clone()))
        .await
        .map_err(|_| AppError::Other("Chat belum tersambung.".into()))?;
    Ok(client_id)
}

#[tauri::command]
pub async fn chat_contacts(app_state: State<'_, AppState>) -> AppResult<Vec<ChatContact>> {
    let c = app_state.conn.lock().map_err(lock_err)?;
    let raw = crate::db::get_setting(&c, "chat_contacts")?.unwrap_or_default();
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

fn write_contacts(app_state: &State<'_, AppState>, contacts: &[ChatContact]) -> AppResult<()> {
    let c = app_state.conn.lock().map_err(lock_err)?;
    let raw = serde_json::to_string(contacts).map_err(|e| AppError::Other(e.to_string()))?;
    crate::db::set_setting(&c, "chat_contacts", &raw)
}

#[tauri::command]
pub async fn chat_save_contact(
    app_state: State<'_, AppState>,
    code: String,
    name: String,
) -> AppResult<Vec<ChatContact>> {
    let code = code.trim().to_string();
    let name = name.trim().to_string();
    if code.is_empty() || name.is_empty() {
        return Err(AppError::Other("Kode dan nama kontak wajib diisi.".into()));
    }
    let mut contacts = chat_contacts(app_state.clone()).await?;
    match contacts.iter_mut().find(|c| c.code == code) {
        Some(c) => c.name = name,
        None => contacts.push(ChatContact { code, name }),
    }
    contacts.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    write_contacts(&app_state, &contacts)?;
    Ok(contacts)
}

#[tauri::command]
pub async fn chat_delete_contact(app_state: State<'_, AppState>, code: String) -> AppResult<Vec<ChatContact>> {
    let mut contacts = chat_contacts(app_state.clone()).await?;
    contacts.retain(|c| c.code != code);
    write_contacts(&app_state, &contacts)?;
    Ok(contacts)
}

/// Buka file hasil chat dengan aplikasi bawaan Windows. Hanya file di dalam
/// folder `GPOS Chat` yang boleh dibuka lewat command ini.
#[tauri::command]
pub async fn chat_open_file(app: AppHandle, path: String, reveal: Option<bool>) -> AppResult<()> {
    let root = chat_dir(&app)?;
    let target = Path::new(&path);
    let canon_root = std::fs::canonicalize(&root)?;
    let canon = std::fs::canonicalize(target)?;
    if !canon.starts_with(&canon_root) {
        return Err(AppError::Other("File di luar folder GPOS Chat.".into()));
    }
    let result = if reveal.unwrap_or(false) {
        tauri_plugin_opener::reveal_item_in_dir(&canon)
    } else {
        let path_str = canon.to_string_lossy();
        if !ext_allowed(&path_str) && !voice_ext_allowed(&path_str) {
            return Err(AppError::Other("Jenis file tidak diizinkan.".into()));
        }
        tauri_plugin_opener::open_path(&canon, None::<&str>)
    };
    result.map_err(|e| AppError::Other(format!("gagal membuka file: {e}")))
}

/// Isi file di folder `GPOS Chat` sebagai bytes mentah — dipakai pemutar
/// voice note di layar chat.
#[tauri::command]
pub async fn chat_read_file(app: AppHandle, path: String) -> AppResult<tauri::ipc::Response> {
    let canon_root = std::fs::canonicalize(chat_dir(&app)?)?;
    let canon = std::fs::canonicalize(Path::new(&path))?;
    if !canon.starts_with(&canon_root) {
        return Err(AppError::Other("File di luar folder GPOS Chat.".into()));
    }
    if !voice_ext_allowed(&canon.to_string_lossy()) {
        return Err(AppError::Other("Hanya voice note yang bisa diputar di sini.".into()));
    }
    let bytes = std::fs::read(&canon)?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitelist_sama_dengan_server() {
        for ok in ["a.JPG", "b.docx", "c.xlsx", "d.pdf", "e.csv", "f.heic"] {
            assert!(ext_allowed(ok), "{ok}");
        }
        for bad in ["a.exe", "b.svg", "c.xlsm", "d.docm", "noext", "e.pdf.exe", "f.bat", "g.webm"] {
            assert!(!ext_allowed(bad), "{bad}");
        }
        // webm/ogg hanya lewat jalur voice note.
        assert!(voice_ext_allowed("vn.webm") && voice_ext_allowed("vn.OGG"));
        assert!(!voice_ext_allowed("vn.pdf") && !voice_ext_allowed("vn.webm.exe"));
    }

    #[test]
    fn nama_file_dibersihkan() {
        assert_eq!(safe_name(r"C:\x\..\lap:1?.pdf"), "lap_1_.pdf");
        assert_eq!(safe_name("../../etc/passwd"), "passwd");
        assert_eq!(safe_name(" . "), "file");
    }

    #[test]
    fn percent_decode_utf8() {
        assert_eq!(percent_decode("laporan%20bulan%20ini.pdf"), "laporan bulan ini.pdf");
        assert_eq!(percent_decode("caf%C3%A9.jpg"), "café.jpg");
        assert_eq!(percent_decode("100%"), "100%");
    }

    #[test]
    fn alamat_ws() {
        assert_eq!(ws_url("https://jjapps.net"), "wss://jjapps.net/api/v1/chat/ws");
        assert_eq!(ws_url("http://localhost:8000"), "ws://localhost:8000/api/v1/chat/ws");
    }
}
