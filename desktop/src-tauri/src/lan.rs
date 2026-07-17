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
pub fn start(
    conn: Arc<Mutex<rusqlite::Connection>>,
    port: u16,
    token: String,
) -> AppResult<LanServerHandle> {
    let server = tiny_http::Server::http(("0.0.0.0", port))
        .map_err(|e| AppError::Other(format!("gagal membuka port {port} untuk Server Pusat: {e}")))?;

    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();

    std::thread::spawn(move || loop {
        if stop_thread.load(Ordering::SeqCst) {
            break;
        }
        match server.recv_timeout(Duration::from_millis(400)) {
            Ok(Some(request)) => handle_request(request, &conn, &token),
            Ok(None) => continue,
            Err(_) => continue,
        }
    });

    Ok(LanServerHandle { stop })
}

fn handle_request(mut request: tiny_http::Request, conn: &Arc<Mutex<rusqlite::Connection>>, token: &str) {
    let url = request.url().to_string();
    let method = request.method().clone();

    let provided_header = request
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(TOKEN_HEADER))
        .map(|h| h.value.as_str().to_string());

    if method == tiny_http::Method::Get && url == "/health" {
        // Tanpa header token: cek liveness saja (dipakai calon fitur "scan
        // LAN" nanti). Dengan header token: validasi PIN pairing sungguhan
        // supaya alur "+ Tambah Server" di client bisa menolak kode salah
        // SEBELUM disimpan, bukan baru ketahuan saat panggilan /rpc/* 401.
        match provided_header {
            None => respond(request, 200, r#"{"ok":true}"#.to_string()),
            Some(ref t) if t == token => {
                respond(request, 200, r#"{"ok":true,"authorized":true}"#.to_string())
            }
            Some(_) => respond(request, 401, r#"{"error":"kode pairing salah"}"#.to_string()),
        }
        return;
    }

    // Semua rute selain /health butuh token yang cocok.
    let provided = provided_header.unwrap_or_default();
    if provided != token {
        respond(request, 401, r#"{"error":"unauthorized"}"#.to_string());
        return;
    }

    if method != tiny_http::Method::Post || !url.starts_with("/rpc/") {
        respond(request, 404, r#"{"error":"unknown command"}"#.to_string());
        return;
    }

    let name = url.trim_start_matches("/rpc/").to_string();

    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        respond(request, 400, r#"{"error":"gagal membaca isi permintaan"}"#.to_string());
        return;
    }
    let args: serde_json::Value = if body.trim().is_empty() {
        serde_json::Value::Null
    } else {
        match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(e) => {
                respond(request, 400, format!(r#"{{"error":"JSON tidak valid: {e}"}}"#));
                return;
            }
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
fn dispatch(
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
                limit: Option<i64>,
            }
            let a: A = args_of(args)?;
            ok_value(db::list_transactions(conn, a.from, a.to, a.limit.unwrap_or(100))?)
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
            }
            let a: A = args_of(args)?;
            ok_value(db::list_stock_movement_batches(conn, a.kind, a.from, a.to, a.limit.unwrap_or(500))?)
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
        _ => Err(AppError::Other("unknown command".to_string())),
    }
}
