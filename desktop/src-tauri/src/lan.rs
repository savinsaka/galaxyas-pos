//! "Server Pusat" (LAN central-server) — client transport + host HTTP server.
//!
//! Satu PC ("host") bisa mengaktifkan Server Pusat: SQLite lokalnya diekspos
//! lewat HTTP kecil (via `tiny_http`) di LAN. PC lain ("client") memanggil
//! command Tauri yang sama seperti biasa, tapi bila `AppState.remote` terisi,
//! command tsb memanggil `lan::call(...)` untuk minta host menjalankan query
//! yang sesungguhnya lewat SQLite-nya sendiri — jadi beberapa kasir bisa
//! berbagi satu database yang sama tanpa cloud/Postgres/MariaDB.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{AppError, AppResult};

const TOKEN_HEADER: &str = "X-Galaxyas-Token";
const CONNECT_ERR_MSG: &str =
    "Tidak dapat terhubung ke Server Pusat — periksa koneksi wifi atau apakah PC pusat masih menyala.";

#[derive(Clone, Debug)]
pub struct RemoteConfig {
    pub base_url: String,
    pub token: String,
}

fn client() -> AppResult<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?)
}

/// Panggil satu "command" di Server Pusat lewat HTTP, meniru pemanggilan
/// Tauri command lokal (nama command + args JSON dengan field yang persis
/// sama namanya seperti parameter command aslinya).
pub async fn call<T: Serialize, R: DeserializeOwned>(
    remote: &RemoteConfig,
    name: &str,
    args: T,
) -> AppResult<R> {
    let url = format!("{}/rpc/{}", remote.base_url.trim_end_matches('/'), name);
    let resp = client()?
        .post(&url)
        .header(TOKEN_HEADER, &remote.token)
        .json(&args)
        .send()
        .await
        .map_err(|_| AppError::Other(CONNECT_ERR_MSG.into()))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        let msg = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| "Server Pusat menolak permintaan".to_string());
        return Err(AppError::Other(msg));
    }

    let body = resp.text().await.map_err(|_| AppError::Other(CONNECT_ERR_MSG.into()))?;
    // Body kosong dianggap `null` (untuk command yang me-return `()`).
    let value: serde_json::Value = if body.trim().is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_str(&body)
            .map_err(|e| AppError::Other(format!("respons Server Pusat tidak valid: {e}")))?
    };
    serde_json::from_value(value)
        .map_err(|e| AppError::Other(format!("respons Server Pusat tidak valid: {e}")))
}

/// Cek keterjangkauan host (dipakai layar pairing "+ Tambah Server" sebelum
/// disimpan). Tidak butuh data toko apa pun, cuma liveness + validasi token.
pub async fn health_check(host: &str, port: u16, token: &str) -> AppResult<String> {
    let url = format!("http://{host}:{port}/health");
    let resp = client()?
        .get(&url)
        .header(TOKEN_HEADER, token)
        .send()
        .await
        .map_err(|_| AppError::Other(CONNECT_ERR_MSG.into()))?;
    if !resp.status().is_success() {
        return Err(AppError::Other(CONNECT_ERR_MSG.into()));
    }
    Ok("ok".to_string())
}

/// Buat kode pairing pendek (6 karakter, uppercase hex-ish) untuk token Server Pusat.
pub fn generate_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..6].to_uppercase()
}

// ---------- Autentikasi (dipakai transport LAN maupun relay) ----------

/// Hasil autentikasi satu permintaan masuk.
#[derive(Debug, Clone)]
pub enum Auth {
    /// Kode pairing 6 karakter milik Server Pusat. Cukup untuk LAN, TIDAK
    /// diterima lewat relay internet karena terlalu pendek untuk ditebak-tebak
    /// dari luar.
    PairingCode,
    /// Token per-perangkat milik satu HP. Sah di kedua jalur.
    Device(crate::models::MobileDevice),
}

/// Validasi kredensial yang dikirim client. Kode pairing dibaca dari settings
/// setiap kali (bukan disalin saat server start) supaya "Buat Kode Baru" langsung
/// berlaku di semua transport. `allow_pairing_code = false` untuk jalur relay.
pub fn authenticate(
    conn: &rusqlite::Connection,
    provided: &str,
    allow_pairing_code: bool,
) -> AppResult<Option<Auth>> {
    if provided.is_empty() {
        return Ok(None);
    }
    if let Some(device) = crate::db::find_mobile_device_by_token(conn, provided)? {
        return Ok(Some(Auth::Device(device)));
    }
    if allow_pairing_code {
        let code = crate::db::get_setting(conn, "lan_server_token")?.unwrap_or_default();
        if !code.is_empty() && provided.eq_ignore_ascii_case(&code) {
            return Ok(Some(Auth::PairingCode));
        }
    }
    Ok(None)
}

/// Tukar kode pairing 6 karakter dengan token panjang milik satu HP. Ini
/// satu-satunya saat token mentah keluar dari PC — pemanggil langsung
/// mengirimkannya ke HP dan tidak menyimpannya di mana pun.
pub fn pair_device(
    conn: &rusqlite::Connection,
    code: &str,
    device_name: &str,
) -> AppResult<crate::models::PairResult> {
    let expected = crate::db::get_setting(conn, "lan_server_token")?.unwrap_or_default();
    if expected.is_empty() || !code.trim().eq_ignore_ascii_case(&expected) {
        return Err(AppError::Other("kode pairing salah".into()));
    }
    let (device, token) = crate::db::create_mobile_device(conn, device_name)?;
    let store_name = crate::db::get_setting(conn, "store_name")?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "GALAXYAS POS".to_string());
    Ok(crate::models::PairResult {
        device_id: device.id,
        device_token: token,
        store_name,
    })
}

// ---------- QR pairing ----------

/// Isi QR pairing. Semua yang perlu diketik kasir di HP dijadikan satu supaya
/// tidak ada yang salah ketik — terutama `store_id` relay yang 32 karakter hex.
/// Field pakai nama panjang biar payload-nya masih terbaca manusia saat
/// menelusuri masalah; ukurannya masih jauh di bawah batas QR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PairingPayload {
    /// Versi format, supaya app HP lama bisa menolak QR yang lebih baru dengan
    /// pesan jelas alih-alih salah menafsirkan.
    pub v: u32,
    pub name: String,
    /// Alamat LAN; kosong bila IP tidak terdeteksi.
    pub host: String,
    pub port: u16,
    /// Alamat relay; kosong bila Akses Online belum diatur.
    pub relay: String,
    pub store_id: String,
    pub code: String,
}

pub const PAIRING_PAYLOAD_VERSION: u32 = 1;

/// Susun isi QR dari settings. `local_ip` dilewatkan (bukan dideteksi di sini)
/// supaya fungsinya murni dan bisa diuji.
pub fn pairing_payload(
    conn: &rusqlite::Connection,
    local_ip: Option<String>,
    port: u16,
) -> AppResult<PairingPayload> {
    let get = |k: &str| -> AppResult<String> {
        Ok(crate::db::get_setting(conn, k)?.unwrap_or_default().trim().to_string())
    };
    let code = get("lan_server_token")?;
    if code.is_empty() {
        return Err(AppError::Other(
            "Kode pairing belum ada — nyalakan Server Pusat dulu.".into(),
        ));
    }
    // Relay hanya dicantumkan kalau Akses Online benar-benar aktif; kalau tidak,
    // HP akan menyimpan alamat yang tidak pernah menjawab.
    let relay_on = crate::db::get_setting(conn, "relay_enabled")?.as_deref() == Some("1");
    let store_name = get("store_name")?;
    Ok(PairingPayload {
        v: PAIRING_PAYLOAD_VERSION,
        name: if store_name.is_empty() { "GALAXYAS POS".to_string() } else { store_name },
        host: local_ip.unwrap_or_default(),
        port,
        relay: if relay_on { get("relay_url")? } else { String::new() },
        store_id: if relay_on { get("relay_store_id")? } else { String::new() },
        code,
    })
}

// ---------- Host server ----------

pub struct LanServerHandle {
    stop: Arc<AtomicBool>,
}

impl LanServerHandle {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn json_header() -> tiny_http::Header {
    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .expect("header statis valid")
}

fn respond(request: tiny_http::Request, code: u16, body: String) {
    let response = tiny_http::Response::from_string(body)
        .with_status_code(code)
        .with_header(json_header());
    let _ = request.respond(response);
}

/// Mulai server HTTP Server Pusat di thread terpisah. `conn` dibagi (Arc<Mutex<..>>)
/// dengan command Tauri lokal, jadi data yang dilihat client via LAN selalu
/// konsisten dengan data yang dipakai host untuk dirinya sendiri.
pub fn start(conn: Arc<Mutex<rusqlite::Connection>>, port: u16) -> AppResult<LanServerHandle> {
    let server = tiny_http::Server::http(("0.0.0.0", port))
        .map_err(|e| AppError::Other(format!("gagal membuka port {port} untuk Server Pusat: {e}")))?;

    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();

    std::thread::spawn(move || loop {
        if stop_thread.load(Ordering::SeqCst) {
            break;
        }
        match server.recv_timeout(Duration::from_millis(400)) {
            Ok(Some(request)) => handle_request(request, &conn),
            Ok(None) => continue,
            Err(_) => continue,
        }
    });

    Ok(LanServerHandle { stop })
}

fn handle_request(mut request: tiny_http::Request, conn: &Arc<Mutex<rusqlite::Connection>>) {
    let url = request.url().to_string();
    let method = request.method().clone();

    let provided_header = request
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(TOKEN_HEADER))
        .map(|h| h.value.as_str().to_string());

    /// Baca body permintaan sebagai JSON (body kosong = `null`).
    fn read_json_body(request: &mut tiny_http::Request) -> Result<serde_json::Value, String> {
        let mut body = String::new();
        if request.as_reader().read_to_string(&mut body).is_err() {
            return Err("gagal membaca isi permintaan".to_string());
        }
        if body.trim().is_empty() {
            return Ok(serde_json::Value::Null);
        }
        serde_json::from_str(&body).map_err(|e| format!("JSON tidak valid: {e}"))
    }

    if method == tiny_http::Method::Get && url == "/health" {
        // Tanpa header token: cek liveness saja (dipakai calon fitur "scan
        // LAN" nanti). Dengan header token: validasi kredensial sungguhan
        // supaya alur "+ Tambah Server" di client bisa menolak kode salah
        // SEBELUM disimpan, bukan baru ketahuan saat panggilan /rpc/* 401.
        let Some(provided) = provided_header else {
            respond(request, 200, r#"{"ok":true}"#.to_string());
            return;
        };
        let authorized = conn
            .lock()
            .ok()
            .and_then(|guard| authenticate(&guard, &provided, true).ok())
            .flatten()
            .is_some();
        if authorized {
            respond(request, 200, r#"{"ok":true,"authorized":true}"#.to_string());
        } else {
            respond(request, 401, r#"{"error":"kode pairing salah"}"#.to_string());
        }
        return;
    }

    // Pendaftaran HP: kredensialnya ada di dalam body (kode pairing), bukan di
    // header, jadi rute ini sengaja dilewati pengecekan token di bawah.
    if method == tiny_http::Method::Post && url == "/pair" {
        let args = match read_json_body(&mut request) {
            Ok(v) => v,
            Err(msg) => {
                respond(request, 400, format!(r#"{{"error":"{msg}"}}"#));
                return;
            }
        };
        let code = args.get("code").and_then(|v| v.as_str()).unwrap_or_default();
        let device_name = args
            .get("device_name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let Ok(guard) = conn.lock() else {
            respond(request, 500, r#"{"error":"gagal mengunci database"}"#.to_string());
            return;
        };
        match pair_device(&guard, code, device_name) {
            Ok(result) => {
                let body = serde_json::to_string(&result).unwrap_or_else(|_| "null".to_string());
                respond(request, 200, body);
            }
            Err(e) => {
                let msg = e.to_string().replace('"', "'");
                respond(request, 401, format!(r#"{{"error":"{msg}"}}"#));
            }
        }
        return;
    }

    // Semua rute selain /health dan /pair butuh kredensial yang sah.
    let provided = provided_header.unwrap_or_default();
    {
        let Ok(guard) = conn.lock() else {
            respond(request, 500, r#"{"error":"gagal mengunci database"}"#.to_string());
            return;
        };
        if !matches!(authenticate(&guard, &provided, true), Ok(Some(_))) {
            drop(guard);
            respond(request, 401, r#"{"error":"unauthorized"}"#.to_string());
            return;
        }
    }

    if method != tiny_http::Method::Post || !url.starts_with("/rpc/") {
        respond(request, 404, r#"{"error":"unknown command"}"#.to_string());
        return;
    }

    let name = url.trim_start_matches("/rpc/").to_string();

    let args = match read_json_body(&mut request) {
        Ok(v) => v,
        Err(msg) => {
            respond(request, 400, format!(r#"{{"error":"{msg}"}}"#));
            return;
        }
    };

    let mut guard = match conn.lock() {
        Ok(g) => g,
        Err(_) => {
            respond(request, 500, r#"{"error":"gagal mengunci database"}"#.to_string());
            return;
        }
    };

    match dispatch(&mut guard, &name, args) {
        Ok(value) => {
            let body = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
            respond(request, 200, body);
        }
        Err(e) if name_unknown(&e, &name) => {
            respond(request, 404, r#"{"error":"unknown command"}"#.to_string());
        }
        Err(e) => {
            let msg = e.to_string().replace('"', "'");
            respond(request, 400, format!(r#"{{"error":"{msg}"}}"#));
        }
    }
}

fn name_unknown(e: &AppError, _name: &str) -> bool {
    matches!(e, AppError::Other(msg) if msg == "unknown command")
}

/// Deserialize args JSON menjadi tipe `T`, membungkus error parsing sebagai `AppError`.
fn args_of<T: DeserializeOwned>(args: serde_json::Value) -> AppResult<T> {
    serde_json::from_value(args).map_err(|e| AppError::Other(format!("argumen tidak valid: {e}")))
}

fn ok_value<T: Serialize>(value: T) -> AppResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|e| AppError::Other(format!("gagal serialisasi hasil: {e}")))
}

/// Tabel dispatch command yang bisa dilayani lewat LAN — daftar ini harus
/// sinkron dengan command Tauri yang di-proxy di `commands.rs` (lihat guard
/// `if let Some(remote) = state.remote_config() { ... }` di masing-masing).
/// Dipakai dua transport: server LAN di file ini dan agent relay (`relay.rs`).
pub fn dispatch(
    conn: &mut rusqlite::Connection,
    name: &str,
    args: serde_json::Value,
) -> AppResult<serde_json::Value> {
    use crate::db;

    match name {
        "list_products" => {
            #[derive(serde::Deserialize)]
            struct A {
                search: Option<String>,
                include_inactive: Option<bool>,
                limit: Option<i64>,
            }
            let a: A = args_of(args)?;
            ok_value(db::list_products(conn, a.search, a.include_inactive.unwrap_or(false), a.limit)?)
        }
        "list_products_page" => {
            #[derive(serde::Deserialize)]
            struct A {
                search: Option<String>,
                include_inactive: Option<bool>,
                brand: Option<String>,
                sort_by: Option<String>,
                sort_dir: Option<String>,
                limit: i64,
                offset: i64,
            }
            let a: A = args_of(args)?;
            ok_value(db::list_products_page(
                conn,
                a.search,
                a.include_inactive.unwrap_or(false),
                a.brand,
                a.sort_by,
                a.sort_dir,
                a.limit,
                a.offset,
            )?)
        }
        "save_product" => {
            #[derive(serde::Deserialize)]
            struct A {
                input: crate::models::ProductInput,
            }
            let a: A = args_of(args)?;
            ok_value(db::upsert_product(conn, a.input)?)
        }
        "toggle_product_active" => {
            #[derive(serde::Deserialize)]
            struct A {
                id: String,
                active: bool,
            }
            let a: A = args_of(args)?;
            ok_value(db::set_product_active(conn, &a.id, a.active)?)
        }
        "delete_product" => {
            #[derive(serde::Deserialize)]
            struct A {
                id: String,
            }
            let a: A = args_of(args)?;
            ok_value(db::delete_product(conn, &a.id)?)
        }
        "dedupe_products" => ok_value(db::dedupe_products_by_barcode(conn)?),
        "find_by_barcode" => {
            #[derive(serde::Deserialize)]
            struct A {
                barcode: String,
            }
            let a: A = args_of(args)?;
            ok_value(db::find_by_barcode(conn, &a.barcode)?)
        }
        "adjust_stock" => {
            #[derive(serde::Deserialize)]
            struct A {
                product_id: String,
                delta: f64,
            }
            let a: A = args_of(args)?;
            ok_value(db::adjust_stock(conn, &a.product_id, a.delta)?)
        }
        "set_stock" => {
            #[derive(serde::Deserialize)]
            struct A {
                product_id: String,
                qty: f64,
            }
            let a: A = args_of(args)?;
            ok_value(db::set_stock(conn, &a.product_id, a.qty)?)
        }
        "checkout" => {
            #[derive(serde::Deserialize)]
            struct A {
                sale: crate::models::SaleInput,
            }
            let a: A = args_of(args)?;
            ok_value(db::create_sale(conn, a.sale)?)
        }
        "list_transactions" => {
            #[derive(serde::Deserialize)]
            struct A {
                from: Option<String>,
                to: Option<String>,
                #[serde(default)]
                search: Option<String>,
                limit: Option<i64>,
                offset: Option<i64>,
            }
            let a: A = args_of(args)?;
            ok_value(db::list_transactions(
                conn,
                a.from,
                a.to,
                a.search,
                a.limit.unwrap_or(100),
                a.offset.unwrap_or(0),
            )?)
        }
        "get_transaction" => {
            #[derive(serde::Deserialize)]
            struct A {
                id: String,
            }
            let a: A = args_of(args)?;
            ok_value(db::get_transaction(conn, &a.id)?)
        }
        "delete_transaction" => {
            #[derive(serde::Deserialize)]
            struct A {
                id: String,
            }
            let a: A = args_of(args)?;
            ok_value(db::delete_transaction(conn, &a.id)?)
        }
        "update_transaction" => {
            #[derive(serde::Deserialize)]
            struct A {
                id: String,
                input: crate::models::SaleInput,
            }
            let a: A = args_of(args)?;
            ok_value(db::update_transaction(conn, &a.id, a.input)?)
        }
        "login" => {
            #[derive(serde::Deserialize)]
            struct A {
                username: String,
                pin: String,
            }
            let a: A = args_of(args)?;
            ok_value(db::login(conn, &a.username, &a.pin)?)
        }
        "list_users" => ok_value(db::list_users(conn)?),
        "save_user" => {
            #[derive(serde::Deserialize)]
            struct A {
                input: crate::models::UserInput,
            }
            let a: A = args_of(args)?;
            ok_value(db::save_user(conn, a.input)?)
        }
        "delete_user" => {
            #[derive(serde::Deserialize)]
            struct A {
                id: String,
            }
            let a: A = args_of(args)?;
            ok_value(db::delete_user(conn, &a.id)?)
        }
        "create_stock_movement" => {
            #[derive(serde::Deserialize)]
            struct A {
                input: crate::models::StockMovementInput,
            }
            let a: A = args_of(args)?;
            ok_value(db::create_stock_movement(conn, a.input)?)
        }
        "list_stock_movements" => {
            #[derive(serde::Deserialize)]
            struct A {
                kind: Option<String>,
                from: Option<String>,
                to: Option<String>,
                limit: Option<i64>,
            }
            let a: A = args_of(args)?;
            ok_value(db::list_stock_movements(conn, a.kind, a.from, a.to, a.limit.unwrap_or(500))?)
        }
        "delete_stock_movement" => {
            #[derive(serde::Deserialize)]
            struct A {
                id: i64,
            }
            let a: A = args_of(args)?;
            ok_value(db::delete_stock_movement(conn, a.id)?)
        }
        "create_stock_movement_batch" => {
            #[derive(serde::Deserialize)]
            struct A {
                input: crate::models::StockMovementBatchInput,
            }
            let a: A = args_of(args)?;
            ok_value(db::create_stock_movement_batch(conn, a.input)?)
        }
        "list_stock_movement_batches" => {
            #[derive(serde::Deserialize)]
            struct A {
                kind: Option<String>,
                from: Option<String>,
                to: Option<String>,
                limit: Option<i64>,
                offset: Option<i64>,
            }
            let a: A = args_of(args)?;
            ok_value(db::list_stock_movement_batches(
                conn,
                a.kind,
                a.from,
                a.to,
                a.limit.unwrap_or(500),
                a.offset.unwrap_or(0),
            )?)
        }
        "get_stock_movement_batch" => {
            #[derive(serde::Deserialize)]
            struct A {
                id: String,
            }
            let a: A = args_of(args)?;
            ok_value(db::get_stock_movement_batch(conn, &a.id)?)
        }
        "update_stock_movement_batch" => {
            #[derive(serde::Deserialize)]
            struct A {
                id: String,
                items: Vec<crate::models::StockMovementBatchItemInput>,
                note: Option<String>,
            }
            let a: A = args_of(args)?;
            ok_value(db::update_stock_movement_batch(conn, &a.id, a.items, a.note)?)
        }
        "delete_stock_movement_batch" => {
            #[derive(serde::Deserialize)]
            struct A {
                id: String,
            }
            let a: A = args_of(args)?;
            ok_value(db::delete_stock_movement_batch(conn, &a.id)?)
        }
        "list_discounts" => ok_value(db::list_discounts(conn)?),
        "save_discount" => {
            #[derive(serde::Deserialize)]
            struct A {
                input: crate::models::DiscountPeriodInput,
            }
            let a: A = args_of(args)?;
            ok_value(db::save_discount(conn, a.input)?)
        }
        "delete_discount" => {
            #[derive(serde::Deserialize)]
            struct A {
                id: String,
            }
            let a: A = args_of(args)?;
            ok_value(db::delete_discount(conn, &a.id)?)
        }
        "list_brands" => ok_value(db::list_brands(conn)?),
        "save_brand" => {
            #[derive(serde::Deserialize)]
            struct A {
                input: crate::models::BrandInput,
            }
            let a: A = args_of(args)?;
            ok_value(db::save_brand(conn, a.input)?)
        }
        "delete_brand" => {
            #[derive(serde::Deserialize)]
            struct A {
                id: String,
            }
            let a: A = args_of(args)?;
            ok_value(db::delete_brand(conn, &a.id)?)
        }
        "list_customers" => {
            #[derive(serde::Deserialize)]
            struct A {
                search: Option<String>,
                include_inactive: Option<bool>,
            }
            let a: A = args_of(args)?;
            ok_value(db::list_customers(conn, a.search, a.include_inactive.unwrap_or(false))?)
        }
        "save_customer" => {
            #[derive(serde::Deserialize)]
            struct A {
                input: crate::models::CustomerInput,
            }
            let a: A = args_of(args)?;
            ok_value(db::upsert_customer(conn, a.input)?)
        }
        "delete_customer" => {
            #[derive(serde::Deserialize)]
            struct A {
                id: String,
            }
            let a: A = args_of(args)?;
            ok_value(db::delete_customer(conn, &a.id)?)
        }
        "list_expenses" => {
            #[derive(serde::Deserialize)]
            struct A {
                from: Option<String>,
                to: Option<String>,
            }
            let a: A = args_of(args)?;
            ok_value(db::list_expenses(conn, a.from, a.to)?)
        }
        "save_expense" => {
            #[derive(serde::Deserialize)]
            struct A {
                input: crate::models::ExpenseInput,
            }
            let a: A = args_of(args)?;
            ok_value(db::save_expense(conn, a.input)?)
        }
        "delete_expense" => {
            #[derive(serde::Deserialize)]
            struct A {
                id: String,
            }
            let a: A = args_of(args)?;
            ok_value(db::delete_expense(conn, &a.id)?)
        }
        "get_active_shift" => ok_value(db::get_active_shift(conn)?),
        "open_shift" => {
            #[derive(serde::Deserialize)]
            struct A {
                input: crate::models::OpenShiftInput,
            }
            let a: A = args_of(args)?;
            ok_value(db::open_shift(conn, a.input)?)
        }
        "close_shift" => {
            #[derive(serde::Deserialize)]
            struct A {
                input: crate::models::CloseShiftInput,
            }
            let a: A = args_of(args)?;
            ok_value(db::close_shift(conn, a.input)?)
        }
        "list_shifts" => {
            #[derive(serde::Deserialize)]
            struct A {
                limit: Option<i64>,
            }
            let a: A = args_of(args)?;
            ok_value(db::list_shifts(conn, a.limit.unwrap_or(100))?)
        }
        "product_sales_report" => {
            #[derive(serde::Deserialize)]
            struct A {
                from: String,
                to: String,
                brands: Vec<String>,
            }
            let a: A = args_of(args)?;
            ok_value(db::product_sales_report(conn, &a.from, &a.to, &a.brands)?)
        }
        "brand_sales_report" => {
            #[derive(serde::Deserialize)]
            struct A {
                from: String,
                to: String,
                brands: Vec<String>,
            }
            let a: A = args_of(args)?;
            ok_value(db::brand_sales_report(conn, &a.from, &a.to, &a.brands)?)
        }
        "sales_item_detail_report" => {
            #[derive(serde::Deserialize)]
            struct A {
                from: String,
                to: String,
                brands: Vec<String>,
            }
            let a: A = args_of(args)?;
            ok_value(db::sales_item_detail_report(conn, &a.from, &a.to, &a.brands)?)
        }
        "daily_sales_report" => {
            #[derive(serde::Deserialize)]
            struct A {
                from: String,
                to: String,
                brands: Vec<String>,
            }
            let a: A = args_of(args)?;
            ok_value(db::daily_sales_report(conn, &a.from, &a.to, &a.brands)?)
        }
        "stock_flow_recap" => {
            #[derive(serde::Deserialize)]
            struct A {
                from: String,
                to: String,
                search: Option<String>,
            }
            let a: A = args_of(args)?;
            ok_value(db::stock_flow_recap(conn, &a.from, &a.to, a.search)?)
        }
        "stock_flow_detail" => {
            #[derive(serde::Deserialize)]
            struct A {
                product_id: String,
                from: String,
                to: String,
            }
            let a: A = args_of(args)?;
            ok_value(db::stock_flow_detail(conn, &a.product_id, &a.from, &a.to)?)
        }
        _ => Err(AppError::Other("unknown command".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DB kosong dengan kode pairing terpasang, seperti sesudah "Jadikan PC ini
    /// Server Pusat" dicentang.
    fn test_db(code: &str) -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().expect("buka db memori");
        crate::db::init_schema(&conn).expect("skema");
        crate::db::seed_defaults(&conn).expect("seed");
        crate::db::set_setting(&conn, "lan_server_token", code).expect("set kode");
        conn
    }

    #[test]
    fn kode_pairing_sah_di_lan_tapi_ditolak_di_relay() {
        let conn = test_db("A1B2C3");
        assert!(matches!(authenticate(&conn, "A1B2C3", true), Ok(Some(Auth::PairingCode))));
        assert!(matches!(authenticate(&conn, "a1b2c3", true), Ok(Some(Auth::PairingCode))));
        // Jalur relay: kode 6 karakter TIDAK boleh cukup.
        assert!(matches!(authenticate(&conn, "A1B2C3", false), Ok(None)));
    }

    #[test]
    fn kredensial_kosong_atau_salah_ditolak() {
        let conn = test_db("A1B2C3");
        assert!(matches!(authenticate(&conn, "", true), Ok(None)));
        assert!(matches!(authenticate(&conn, "ZZZZZZ", true), Ok(None)));
    }

    #[test]
    fn token_perangkat_sah_di_kedua_jalur_dan_mati_setelah_dicabut() {
        let conn = test_db("A1B2C3");
        let paired = pair_device(&conn, "A1B2C3", "Redmi Note 12").expect("pairing");
        assert_eq!(paired.device_token.len(), 64, "token harus 64 hex, bukan kode pendek");

        assert!(matches!(
            authenticate(&conn, &paired.device_token, true),
            Ok(Some(Auth::Device(_)))
        ));
        assert!(matches!(
            authenticate(&conn, &paired.device_token, false),
            Ok(Some(Auth::Device(_)))
        ));

        crate::db::revoke_mobile_device(&conn, &paired.device_id).expect("cabut");
        assert!(matches!(authenticate(&conn, &paired.device_token, false), Ok(None)));
    }

    #[test]
    fn ganti_kode_pairing_tidak_memutus_hp_yang_sudah_terdaftar() {
        let conn = test_db("A1B2C3");
        let paired = pair_device(&conn, "A1B2C3", "HP Kasir").expect("pairing");
        crate::db::set_setting(&conn, "lan_server_token", "ZZZ999").expect("kode baru");

        assert!(matches!(authenticate(&conn, "A1B2C3", true), Ok(None)), "kode lama harus mati");
        assert!(
            matches!(authenticate(&conn, &paired.device_token, true), Ok(Some(Auth::Device(_)))),
            "HP yang sudah terdaftar tidak boleh ikut terputus"
        );
    }

    #[test]
    fn pairing_dengan_kode_salah_gagal_dan_tidak_membuat_perangkat() {
        let conn = test_db("A1B2C3");
        assert!(pair_device(&conn, "SALAH1", "HP").is_err());
        assert!(crate::db::list_mobile_devices(&conn).expect("daftar").is_empty());
    }

    #[test]
    fn qr_memuat_lan_dan_relay_saat_akses_online_aktif() {
        let conn = test_db("A1B2C3");
        crate::db::set_setting(&conn, "store_name", "GALAXYAS Toko 1").unwrap();
        crate::db::set_setting(&conn, "relay_enabled", "1").unwrap();
        crate::db::set_setting(&conn, "relay_url", "relay.jjapps.net").unwrap();
        crate::db::set_setting(&conn, "relay_store_id", "ed2a445ded3d12c68a8b9c7a3e58a18e").unwrap();

        let p = pairing_payload(&conn, Some("192.168.18.11".into()), 8899).expect("payload");
        assert_eq!(p.v, PAIRING_PAYLOAD_VERSION);
        assert_eq!(p.name, "GALAXYAS Toko 1");
        assert_eq!(p.host, "192.168.18.11");
        assert_eq!(p.port, 8899);
        assert_eq!(p.relay, "relay.jjapps.net");
        assert_eq!(p.store_id, "ed2a445ded3d12c68a8b9c7a3e58a18e");
        assert_eq!(p.code, "A1B2C3");
    }

    /// Akses Online mati = jangan titipkan alamat relay ke HP; kalau dititipkan,
    /// kasir bisa memilih [ONLINE] yang tidak akan pernah menjawab.
    #[test]
    fn qr_tidak_memuat_relay_saat_akses_online_mati() {
        let conn = test_db("A1B2C3");
        crate::db::set_setting(&conn, "relay_url", "relay.jjapps.net").unwrap();
        crate::db::set_setting(&conn, "relay_store_id", "abc123").unwrap();
        crate::db::set_setting(&conn, "relay_enabled", "0").unwrap();

        let p = pairing_payload(&conn, Some("192.168.18.11".into()), 8899).expect("payload");
        assert_eq!(p.relay, "");
        assert_eq!(p.store_id, "");
        assert_eq!(p.host, "192.168.18.11", "jalur LAN tetap ikut");
    }

    /// Bentuk JSON-nya dikunci: yang membacanya adalah parser di app HP
    /// (`PairingQrTest.kt`), dan tidak ada codegen yang menjaga keduanya sinkron.
    /// Kalau uji ini gagal karena field diubah, ubah juga `PairingPayload` di
    /// `Models.kt` + fixture di `PairingQrTest.kt` dalam commit yang sama.
    #[test]
    fn bentuk_json_qr_dikunci_karena_diparse_app_hp() {
        let payload = PairingPayload {
            v: 1,
            name: "GALAXYAS Toko 1".into(),
            host: "192.168.18.11".into(),
            port: 8899,
            relay: "relay.jjapps.net".into(),
            store_id: "ed2a445ded3d12c68a8b9c7a3e58a18e".into(),
            code: "A1B2C3".into(),
        };
        assert_eq!(
            serde_json::to_string(&payload).unwrap(),
            r#"{"v":1,"name":"GALAXYAS Toko 1","host":"192.168.18.11","port":8899,"relay":"relay.jjapps.net","store_id":"ed2a445ded3d12c68a8b9c7a3e58a18e","code":"A1B2C3"}"#
        );
    }

    #[test]
    fn qr_ditolak_bila_kode_pairing_belum_ada() {
        let conn = test_db("");
        assert!(pairing_payload(&conn, Some("192.168.1.2".into()), 8899).is_err());
    }

    #[test]
    fn qr_tetap_terbentuk_walau_ip_lan_tidak_terdeteksi() {
        let conn = test_db("A1B2C3");
        let p = pairing_payload(&conn, None, 8899).expect("payload");
        assert_eq!(p.host, "", "wifi mati bukan alasan QR gagal dibuat");
        assert_eq!(p.code, "A1B2C3");
    }

    #[test]
    fn nama_toko_kosong_pakai_nama_default() {
        let conn = test_db("A1B2C3");
        crate::db::set_setting(&conn, "store_name", "").unwrap();
        let p = pairing_payload(&conn, None, 8899).expect("payload");
        assert_eq!(p.name, "GALAXYAS POS");
    }

    #[test]
    fn token_disimpan_sebagai_hash_bukan_teks_asli() {
        let conn = test_db("A1B2C3");
        let paired = pair_device(&conn, "A1B2C3", "HP").expect("pairing");
        let stored: String = conn
            .query_row("SELECT token_hash FROM mobile_devices", [], |r| r.get(0))
            .expect("baca hash");
        assert_ne!(stored, paired.device_token);
        assert_eq!(stored, crate::db::hash_device_token(&paired.device_token));
    }
}
