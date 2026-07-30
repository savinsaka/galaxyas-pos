//! Agent relay — supaya HP kasir bisa dipakai dari luar wifi toko.
//!
//! PC ini yang **menelepon keluar** ke relay di VPS lewat WebSocket, lalu
//! menunggu permintaan dari HP dan menjalankannya di SQLite lokal memakai
//! `lan::dispatch` yang sama persis dengan jalur LAN. Arahnya keluar karena PC
//! kasir umumnya tidak punya IP publik (CGNAT), jadi tidak bisa dihubungi
//! langsung dari internet.
//!
//! Tidak ada antrian di sisi mana pun: kalau socket ini putus, relay langsung
//! menolak permintaan HP dengan 503 dan tidak menyimpannya. Begitu tersambung
//! lagi, tidak ada permintaan lama yang menyusul masuk — stok/opname selalu
//! mencerminkan keadaan sekarang.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

use crate::error::{AppError, AppResult};

/// Selang denyut ke relay. Sekaligus mengirim ulang daftar hash token
/// perangkat, supaya pencabutan HP tersaring di relay tanpa plumbing tambahan
/// (PC ini sendiri sudah menolak seketika).
///
/// **Harus lebih rapat dari ping-timeout server** (default library `websockets`
/// di uvicorn: ping tiap 20 detik, timeout 20 detik). Pong balasan tungstenite
/// hanya ikut ter-flush saat kita menulis sesuatu, jadi denyut yang terlalu
/// jarang membuat relay menganggap PC ini mati padahal sehat.
const PING_INTERVAL: Duration = Duration::from_secs(15);

/// Jeda reconnect, naik bertahap supaya internet toko yang putus lama tidak
/// dihujani percobaan koneksi.
const BACKOFF_SECS: [u64; 5] = [1, 2, 5, 15, 30];

#[derive(Debug, Clone, Default, Serialize)]
pub struct RelayStatus {
    pub enabled: bool,
    pub connected: bool,
    pub url: String,
    pub store_id: String,
    /// Pesan kegagalan terakhir (mis. agent key salah, DNS tidak ketemu).
    pub last_error: Option<String>,
    pub connected_since: Option<String>,
}

/// Konfigurasi tersimpan di tabel settings.
#[derive(Debug, Clone)]
pub struct RelayConfig {
    pub url: String,
    pub store_id: String,
    pub agent_key: String,
}

impl RelayConfig {
    /// Baca dari settings; `None` bila salah satu bagian belum diisi.
    pub fn load(conn: &rusqlite::Connection) -> AppResult<Option<Self>> {
        let get = |k: &str| -> AppResult<String> {
            Ok(crate::db::get_setting(conn, k)?.unwrap_or_default().trim().to_string())
        };
        let url = get("relay_url")?;
        let store_id = get("relay_store_id")?;
        let agent_key = get("relay_agent_key")?;
        if url.is_empty() || store_id.is_empty() || agent_key.is_empty() {
            return Ok(None);
        }
        Ok(Some(RelayConfig { url, store_id, agent_key }))
    }

    /// URL WebSocket lengkap. Menerima bentuk pendek (`relay.jjapps.net`) maupun
    /// lengkap (`wss://relay.jjapps.net/agent/ws`) supaya isian di Pengaturan
    /// tidak rewel.
    fn ws_url(&self) -> String {
        let raw = self.url.trim().trim_end_matches('/');
        let with_scheme = if raw.starts_with("ws://") || raw.starts_with("wss://") {
            raw.to_string()
        } else if let Some(rest) = raw.strip_prefix("https://") {
            format!("wss://{rest}")
        } else if let Some(rest) = raw.strip_prefix("http://") {
            format!("ws://{rest}")
        } else {
            format!("wss://{raw}")
        };
        if with_scheme.ends_with("/agent/ws") {
            with_scheme
        } else {
            format!("{with_scheme}/agent/ws")
        }
    }
}

/// Kendali hidup-matinya agent.
///
/// Sinyal berhenti memakai `watch` (bukan sekadar flag yang dipoll) supaya
/// `stop()` **langsung** memutus socket: kalau berhentinya baru ketahuan saat
/// denyut berikutnya, relay masih mengira PC kasir hidup sampai 15 detik dan
/// HP menunggu sia-sia sampai 504 — padahal PC-nya sengaja dimatikan.
/// Handle yang di-drop tanpa `stop()` juga menghentikan agent (channel tutup).
pub struct RelayHandle {
    stop_tx: tokio::sync::watch::Sender<bool>,
}

impl RelayHandle {
    pub fn stop(&self) {
        let _ = self.stop_tx.send(true);
    }
}

/// Nyalakan agent. Loop-nya hidup di async runtime Tauri dan menyambung ulang
/// sendiri sampai `stop()` dipanggil.
pub fn start(
    conn: Arc<Mutex<rusqlite::Connection>>,
    config: RelayConfig,
    status: Arc<Mutex<RelayStatus>>,
) -> RelayHandle {
    let (stop_tx, mut stop_rx) = tokio::sync::watch::channel(false);

    {
        if let Ok(mut s) = status.lock() {
            s.enabled = true;
            s.connected = false;
            s.url = config.url.clone();
            s.store_id = config.store_id.clone();
            s.last_error = None;
            s.connected_since = None;
        }
    }

    tauri::async_runtime::spawn(async move {
        let mut attempt = 0usize;
        while !*stop_rx.borrow() {
            match run_session(&conn, &config, &status, &mut stop_rx).await {
                Ok(()) => attempt = 0,
                Err(e) => {
                    if let Ok(mut s) = status.lock() {
                        s.last_error = Some(e.to_string());
                    }
                    attempt = (attempt + 1).min(BACKOFF_SECS.len() - 1);
                }
            }
            if let Ok(mut s) = status.lock() {
                s.connected = false;
                s.connected_since = None;
            }
            if *stop_rx.borrow() {
                break;
            }
            // Jeda reconnect ikut bisa disela, supaya mematikan Akses Online
            // saat sedang menunggu tidak perlu menunggu jeda itu habis.
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(BACKOFF_SECS[attempt])) => {}
                _ = stop_rx.changed() => {}
            }
        }
        if let Ok(mut s) = status.lock() {
            s.enabled = false;
            s.connected = false;
            s.connected_since = None;
        }
    });

    RelayHandle { stop_tx }
}

/// Satu sesi koneksi: sambung, layani sampai putus. Balik `Ok` bila putus wajar.
async fn run_session(
    conn: &Arc<Mutex<rusqlite::Connection>>,
    config: &RelayConfig,
    status: &Arc<Mutex<RelayStatus>>,
    stop_rx: &mut tokio::sync::watch::Receiver<bool>,
) -> AppResult<()> {
    let mut request = config
        .ws_url()
        .into_client_request()
        .map_err(|e| AppError::Other(format!("URL relay tidak valid: {e}")))?;
    request.headers_mut().insert(
        "X-Store-Id",
        config
            .store_id
            .parse()
            .map_err(|_| AppError::Other("Store ID relay tidak valid".to_string()))?,
    );
    request.headers_mut().insert(
        "X-Agent-Key",
        config
            .agent_key
            .parse()
            .map_err(|_| AppError::Other("Agent key relay tidak valid".to_string()))?,
    );

    let (ws, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| AppError::Other(format!("gagal menyambung ke relay: {e}")))?;
    let (mut writer, mut reader) = ws.split();

    if let Ok(mut s) = status.lock() {
        s.connected = true;
        s.last_error = None;
        s.connected_since = Some(chrono::Utc::now().to_rfc3339());
    }

    // Semua kiriman keluar lewat satu channel supaya beberapa permintaan bisa
    // diproses berbarengan tanpa berebut writer.
    let (tx, mut rx) = mpsc::channel::<String>(64);
    let writer_task = tauri::async_runtime::spawn(async move {
        while let Some(text) = rx.recv().await {
            if writer.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
        let _ = writer.close().await;
    });

    let _ = tx.send(token_hashes_message(conn)).await;

    let mut ping = tokio::time::interval(PING_INTERVAL);
    ping.tick().await; // tick pertama langsung selesai; lewati.

    loop {
        if *stop_rx.borrow() {
            break;
        }
        tokio::select! {
            // Cabang pertama: berhenti harus memutus socket SEKARANG, bukan
            // menunggu denyut atau pesan berikutnya datang.
            _ = stop_rx.changed() => break,
            _ = ping.tick() => {
                // Denyut sekaligus penyegar daftar token yang sah di relay.
                if tx.send(token_hashes_message(conn)).await.is_err() {
                    break;
                }
            }
            incoming = reader.next() => {
                let Some(message) = incoming else { break };
                let message = message
                    .map_err(|e| AppError::Other(format!("koneksi relay terputus: {e}")))?;
                let text = match message {
                    Message::Text(t) => t,
                    Message::Close(_) => break,
                    _ => continue,
                };
                let Ok(envelope) = serde_json::from_str::<serde_json::Value>(&text) else {
                    continue;
                };
                let conn = conn.clone();
                let tx = tx.clone();
                tauri::async_runtime::spawn(async move {
                    for reply in handle_envelope(conn, envelope).await {
                        if tx.send(reply).await.is_err() {
                            break;
                        }
                    }
                });
            }
        }
    }

    drop(tx);
    let _ = writer_task.await;
    Ok(())
}

/// `{"type":"tokens","hashes":[…]}` — relay memakainya untuk menolak token
/// asing tanpa membangunkan PC ini.
fn token_hashes_message(conn: &Arc<Mutex<rusqlite::Connection>>) -> String {
    let hashes = conn
        .lock()
        .ok()
        .and_then(|guard| crate::db::mobile_device_token_hashes(&guard).ok())
        .unwrap_or_default();
    serde_json::json!({ "type": "tokens", "hashes": hashes }).to_string()
}

/// Jalankan satu permintaan dari relay dan susun balasannya. Balik daftar kosong
/// untuk pesan yang bukan permintaan (mis. ping dari relay).
async fn handle_envelope(
    conn: Arc<Mutex<rusqlite::Connection>>,
    envelope: serde_json::Value,
) -> Vec<String> {
    let Some(id) = envelope.get("id").and_then(|v| v.as_str()).map(str::to_string) else {
        return Vec::new();
    };
    let kind = envelope.get("kind").and_then(|v| v.as_str()).unwrap_or("rpc").to_string();
    let conn_for_tokens = conn.clone();

    // rusqlite dikunci Mutex biasa (bukan async), jadi kerjanya dipindah ke
    // thread blocking supaya runtime tidak ikut tertahan saat query panjang.
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut guard = match conn.lock() {
            Ok(g) => g,
            Err(_) => return (500u16, serde_json::json!({"error": "gagal mengunci database"}), false),
        };
        match kind.as_str() {
            "pair" => {
                let code = envelope.get("code").and_then(|v| v.as_str()).unwrap_or_default();
                let name = envelope.get("device_name").and_then(|v| v.as_str()).unwrap_or_default();
                match crate::lan::pair_device(&guard, code, name) {
                    Ok(res) => (
                        200,
                        serde_json::to_value(res).unwrap_or(serde_json::Value::Null),
                        true,
                    ),
                    Err(e) => (401, serde_json::json!({ "error": e.to_string() }), false),
                }
            }
            _ => {
                let auth = envelope.get("auth").and_then(|v| v.as_str()).unwrap_or_default();
                // `false` = kode pairing 6 karakter TIDAK diterima lewat relay;
                // terlalu pendek untuk dijaga dari tebakan lewat internet.
                let device = match crate::lan::authenticate(&guard, auth, false) {
                    Ok(Some(crate::lan::Auth::Device(d))) => d,
                    _ => return (401, serde_json::json!({"error": "unauthorized"}), false),
                };
                let cmd = envelope.get("cmd").and_then(|v| v.as_str()).unwrap_or_default();
                let args = envelope.get("args").cloned().unwrap_or(serde_json::Value::Null);
                // Jejak diagnostik saat menelusuri laporan "stok kok beda" —
                // hanya ke stderr (terlihat lewat `npm run tauri dev`), bukan
                // audit trail permanen. Yang tersimpan cuma `last_seen_at` per
                // perangkat, ditampilkan di layar Pengaturan.
                eprintln!("[relay] {} ({}) -> {}", device.name, device.id, cmd);
                match crate::lan::dispatch(&mut guard, cmd, args) {
                    Ok(value) => (200, value, false),
                    Err(e) => (400, serde_json::json!({ "error": e.to_string() }), false),
                }
            }
        }
    })
    .await;

    let Ok((status, body, paired)) = result else {
        return Vec::new();
    };
    let mut out =
        vec![serde_json::json!({ "id": id, "status": status, "body": body }).to_string()];
    if paired {
        // HP baru harus langsung dikenali penyaring token di relay; kalau
        // menunggu ping berikutnya, HP yang baru saja berhasil pairing malah
        // ditolak 401 sampai 30 detik.
        out.push(token_hashes_message(&conn_for_tokens));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn(code: &str) -> Arc<Mutex<rusqlite::Connection>> {
        let conn = rusqlite::Connection::open_in_memory().expect("buka db memori");
        crate::db::init_schema(&conn).expect("skema");
        crate::db::seed_defaults(&conn).expect("seed");
        crate::db::set_setting(&conn, "lan_server_token", code).expect("kode");
        Arc::new(Mutex::new(conn))
    }

    fn cfg(url: &str) -> RelayConfig {
        RelayConfig {
            url: url.to_string(),
            store_id: "toko".into(),
            agent_key: "kunci".into(),
        }
    }

    /// Balas satu envelope dan kembalikan (status, body) dari pesan pertama.
    fn call(
        conn: &Arc<Mutex<rusqlite::Connection>>,
        envelope: serde_json::Value,
    ) -> (u64, serde_json::Value, usize) {
        let replies = tauri::async_runtime::block_on(handle_envelope(conn.clone(), envelope));
        let first: serde_json::Value =
            serde_json::from_str(&replies[0]).expect("balasan JSON valid");
        (
            first["status"].as_u64().expect("status"),
            first["body"].clone(),
            replies.len(),
        )
    }

    #[test]
    fn url_relay_dinormalkan_dari_berbagai_bentuk_isian() {
        // Isian di Pengaturan sengaja dibuat permisif — pemilik toko mengetik
        // apa saja yang dia lihat di DEPLOY.md.
        assert_eq!(cfg("relay.jjapps.net").ws_url(), "wss://relay.jjapps.net/agent/ws");
        assert_eq!(cfg("https://relay.jjapps.net").ws_url(), "wss://relay.jjapps.net/agent/ws");
        assert_eq!(cfg("https://relay.jjapps.net/").ws_url(), "wss://relay.jjapps.net/agent/ws");
        assert_eq!(
            cfg("wss://relay.jjapps.net/agent/ws").ws_url(),
            "wss://relay.jjapps.net/agent/ws"
        );
        // Uji lokal tanpa TLS harus tetap bisa (lihat relay/README.md).
        assert_eq!(cfg("http://localhost:9010").ws_url(), "ws://localhost:9010/agent/ws");
        assert_eq!(cfg("ws://localhost:9010").ws_url(), "ws://localhost:9010/agent/ws");
    }

    #[test]
    fn rpc_tanpa_token_ditolak_401() {
        let conn = test_conn("A1B2C3");
        let (status, body, _) = call(
            &conn,
            serde_json::json!({"id":"1","kind":"rpc","cmd":"list_products","args":{},"auth":""}),
        );
        assert_eq!(status, 401);
        assert_eq!(body["error"], "unauthorized");
    }

    #[test]
    fn kode_pairing_6_karakter_tidak_bisa_dipakai_sebagai_token_lewat_relay() {
        let conn = test_conn("A1B2C3");
        let (status, _, _) = call(
            &conn,
            serde_json::json!({"id":"1","kind":"rpc","cmd":"list_products","args":{},"auth":"A1B2C3"}),
        );
        assert_eq!(status, 401, "kode pendek tidak boleh sah lewat internet");
    }

    #[test]
    fn pair_lalu_rpc_berhasil_dan_daftar_hash_ikut_dikirim() {
        let conn = test_conn("A1B2C3");

        let (status, body, count) = call(
            &conn,
            serde_json::json!({"id":"p1","kind":"pair","code":"A1B2C3","device_name":"Redmi"}),
        );
        assert_eq!(status, 200);
        assert_eq!(count, 2, "pairing harus disusul kiriman daftar hash token");
        let token = body["device_token"].as_str().expect("token").to_string();

        let (status, body, _) = call(
            &conn,
            serde_json::json!({"id":"r1","kind":"rpc","cmd":"list_products","args":{},"auth":token}),
        );
        assert_eq!(status, 200);
        assert!(body.is_array(), "list_products harus balik array, dapat {body}");
    }

    #[test]
    fn pair_dengan_kode_salah_dibalas_401() {
        let conn = test_conn("A1B2C3");
        let (status, body, count) = call(
            &conn,
            serde_json::json!({"id":"p1","kind":"pair","code":"SALAH1","device_name":"Redmi"}),
        );
        assert_eq!(status, 401);
        assert_eq!(count, 1, "pairing gagal tidak boleh mengirim daftar hash");
        assert!(body["error"].is_string());
    }

    #[test]
    fn command_tak_dikenal_dibalas_error_bukan_panik() {
        let conn = test_conn("A1B2C3");
        let (_, body, _) = call(
            &conn,
            serde_json::json!({"id":"p1","kind":"pair","code":"A1B2C3","device_name":"HP"}),
        );
        let token = body["device_token"].as_str().unwrap().to_string();

        let (status, body, _) = call(
            &conn,
            serde_json::json!({"id":"x","kind":"rpc","cmd":"tidak_ada","args":{},"auth":token}),
        );
        assert_eq!(status, 400);
        assert_eq!(body["error"], "unknown command");
    }

    #[test]
    fn pesan_tanpa_id_diabaikan() {
        let conn = test_conn("A1B2C3");
        let replies = tauri::async_runtime::block_on(handle_envelope(
            conn.clone(),
            serde_json::json!({"type":"ping"}),
        ));
        assert!(replies.is_empty());
    }

    /// Rangkaian penuh MELALUI relay sungguhan: agent di file ini menyambung ke
    /// relay yang benar-benar berjalan, lalu **kode klien PC kasir yang
    /// sesungguhnya** (`lan::health_check`, `lan::pair_client`, `lan::call`
    /// dengan jalur online) berperan sebagai kasir dari luar toko.
    /// Ini yang membuktikan hal-hal yang tidak bisa diuji per bagian: handshake
    /// WebSocket, header auth agent, envelope bolak-balik, penyusunan base URL
    /// `/s/<store_id>`, dan janji "tanpa antrian" saat agent dimatikan.
    ///
    /// Butuh relay hidup, jadi `#[ignore]` — jalankan lewat
    /// `relay/scripts/e2e_desktop.py` yang menyalakan relay lalu memanggil:
    ///
    /// ```text
    /// cargo test --lib relay_e2e -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "butuh relay berjalan; lihat relay/scripts/e2e_desktop.py"]
    fn relay_e2e_lewat_relay_sungguhan() {
        let http_base = std::env::var("GALAXYAS_RELAY_HTTP")
            .expect("GALAXYAS_RELAY_HTTP belum diset (mis. http://127.0.0.1:9010)");
        let ws_url = std::env::var("GALAXYAS_RELAY_WS").expect("GALAXYAS_RELAY_WS belum diset");
        let store_id = std::env::var("GALAXYAS_RELAY_STORE").expect("GALAXYAS_RELAY_STORE");
        let agent_key = std::env::var("GALAXYAS_RELAY_KEY").expect("GALAXYAS_RELAY_KEY");

        let conn = test_conn("TEST01");
        let product = {
            let guard = conn.lock().unwrap();
            let p = crate::db::upsert_product(
                &guard,
                crate::models::ProductInput {
                    id: None,
                    name: "Aqua 600ml".into(),
                    barcode: Some("899000111".into()),
                    category: None,
                    brand: Some("Aqua".into()),
                    unit: Some("pcs".into()),
                    sell_price: 4000.0,
                    cost_price: 3000.0,
                    default_discount: 0.0,
                    is_active: true,
                },
            )
            .expect("buat produk");
            crate::db::set_stock(&guard, &p.id, 10.0).expect("set stok");
            p
        };

        let status = Arc::new(Mutex::new(RelayStatus::default()));
        let handle = start(
            conn.clone(),
            RelayConfig { url: ws_url, store_id: store_id.clone(), agent_key },
            status.clone(),
        );

        let stok_sekarang = || -> f64 {
            conn.lock()
                .unwrap()
                .query_row("SELECT qty FROM stock WHERE product_id = ?1", [&product.id], |r| {
                    r.get(0)
                })
                .expect("baca stok")
        };

        tauri::async_runtime::block_on(async {
            // Tunggu socket agent benar-benar tersambung.
            let mut connected = false;
            for _ in 0..100 {
                if status.lock().unwrap().connected {
                    connected = true;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            assert!(connected, "agent tidak tersambung: {:?}", status.lock().unwrap().last_error);

            // Base URL disusun oleh registry, sama seperti di app sungguhan —
            // jadi salah bentuk `/s/<store_id>` ketahuan di sini.
            let entry = crate::servers::ServerInfo::new_remote(
                "Kasir Pusat".into(),
                None,
                None,
                Some(http_base.clone()),
                Some(store_id.clone()),
                String::new(),
            );
            let base = entry
                .base_url(crate::servers::ServerPath::Online)
                .expect("base url online");
            assert_eq!(base, format!("{http_base}/s/{store_id}"));

            // 1. Relay tahu PC kasir online.
            crate::lan::health_check(&base, "", true)
                .await
                .expect("relay harus melihat agent online");

            // 2. Pairing lewat relay menghasilkan token panjang.
            let paired = crate::lan::pair_client(&base, "TEST01", "PC Uji", true)
                .await
                .expect("pair");
            assert_eq!(paired.device_token.len(), 64);

            let remote = crate::lan::RemoteConfig {
                base_url: base.clone(),
                token: paired.device_token.clone(),
                online: true,
            };

            // 3. RPC baca menembus sampai SQLite.
            let products: Vec<crate::models::ProductWithStock> =
                crate::lan::call(&remote, "list_products", serde_json::json!({}))
                    .await
                    .expect("list_products");
            assert_eq!(products[0].product.name, "Aqua 600ml");
            assert_eq!(products[0].stock_qty, 10.0);

            // 4. RPC tulis benar-benar mengubah database, bukan cuma membalas OK.
            let sisa: f64 = crate::lan::call(
                &remote,
                "adjust_stock",
                serde_json::json!({"product_id": product.id, "delta": -3.0}),
            )
            .await
            .expect("adjust_stock");
            assert_eq!(sisa, 7.0);
            assert_eq!(stok_sekarang(), 7.0, "perubahan harus benar-benar tersimpan");

            // 5. Checkout yang dikirim DUA KALI dengan kunci idempoten yang sama
            //    (skenario: jawaban pertama hilang di jaringan lalu kasir menekan
            //    Bayar lagi) hanya boleh tercatat sekali.
            let sale = serde_json::json!({"sale": {
                "cashier_id": "admin",
                "payment_method": "Tunai",
                "paid": 4000.0,
                "items": [{
                    "product_id": product.id,
                    "name": "Aqua 600ml",
                    "price": 4000.0,
                    "qty": 1.0,
                    "discount": 0.0
                }],
                "client_ref": "e2e-ref-1"
            }});
            let pertama: serde_json::Value = crate::lan::call(&remote, "checkout", sale.clone())
                .await
                .expect("checkout pertama");
            assert_eq!(stok_sekarang(), 6.0, "penjualan pertama harus mengurangi stok");

            let ulang: serde_json::Value = crate::lan::call(&remote, "checkout", sale)
                .await
                .expect("checkout ulang");
            assert_eq!(
                ulang["invoice_no"], pertama["invoice_no"],
                "percobaan ulang harus mengembalikan transaksi yang sama"
            );
            assert_eq!(stok_sekarang(), 6.0, "stok TIDAK boleh berkurang dua kali");

            // 6. Token asing ditolak.
            let palsu = crate::lan::RemoteConfig {
                base_url: base.clone(),
                token: "x".repeat(64),
                online: true,
            };
            let err = crate::lan::call::<_, serde_json::Value>(
                &palsu,
                "list_products",
                serde_json::json!({}),
            )
            .await
            .expect_err("token palsu harus ditolak");
            assert!(err.to_string().contains("unauthorized"), "dapat: {err}");

            // 7. Agent dimatikan (= PC kasir mati): gagal SEKETIKA, tanpa antrian.
            handle.stop();
            let mut offline = false;
            for _ in 0..100 {
                if crate::lan::health_check(&base, "", true).await.is_err() {
                    offline = true;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            assert!(offline, "relay harus tahu agent sudah putus");

            let mulai = std::time::Instant::now();
            let err = crate::lan::call::<_, serde_json::Value>(
                &remote,
                "checkout",
                serde_json::json!({"sale": {}}),
            )
            .await
            .expect_err("checkout saat PC pusat mati harus gagal");
            let lama = mulai.elapsed();
            assert!(lama < Duration::from_secs(2), "harus gagal cepat, bukan menggantung: {lama:?}");
            // Alasan dari relay harus sampai ke kasir apa adanya, bukan tertutup
            // pesan generik "tidak dapat terhubung".
            assert!(err.to_string().contains("PC kasir"), "pesan relay hilang: {err}");

            // 8. Stok TIDAK berubah lagi — tidak ada permintaan tertahan yang
            //    menyusul dieksekusi. Inilah janji "tanpa antrian".
            tokio::time::sleep(Duration::from_secs(2)).await;
            assert_eq!(stok_sekarang(), 6.0, "tidak boleh ada permintaan yang menyusul jalan");
        });
    }
}
