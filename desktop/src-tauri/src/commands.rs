use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use rusqlite::Connection;
use tauri::State;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{
    BrandSalesRow, CloseShiftInput, Customer, CustomerInput, DiscountPeriod, DiscountPeriodInput,
    Expense, ExpenseInput, OpenShiftInput, Product, ProductInput, ProductSalesRow,
    ProductWithStock, SaleInput, Shift, StockMovement, StockMovementInput, StoreInfo, SyncResult,
    TransactionDetail, User, UserInput,
};
use crate::stores;
use crate::sync;

/// State global aplikasi: koneksi SQLite (toko aktif) dijaga Mutex, bisa
/// di-swap saat pindah toko lewat `select_store`. `conn` dibungkus `Arc` agar
/// bisa dibagi dengan thread server "Server Pusat" (`lan::start`) tanpa
/// mengubah cara command lain memakainya (`state.lock()` tetap sama).
pub struct AppState {
    pub conn: Arc<Mutex<Connection>>,
    pub data_dir: PathBuf,
    /// Terisi bila mode klien "Server Pusat" aktif: command yang di-proxy
    /// memanggil host lewat HTTP alih-alih SQLite lokal.
    pub remote: Mutex<Option<crate::lan::RemoteConfig>>,
    /// Handle thread server Server Pusat, bila PC ini bertindak sebagai host.
    pub lan: Mutex<Option<crate::lan::LanServerHandle>>,
}

impl AppState {
    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| AppError::Other("gagal mengunci database".into()))
    }

    fn remote_config(&self) -> Option<crate::lan::RemoteConfig> {
        self.remote.lock().ok().and_then(|g| g.clone())
    }
}

// ---------- Multi-database (toko) ----------

#[tauri::command]
pub fn list_stores(state: State<'_, AppState>) -> AppResult<Vec<StoreInfo>> {
    stores::list_stores(&state.data_dir)
}

#[tauri::command]
pub fn current_store(state: State<'_, AppState>) -> AppResult<StoreInfo> {
    stores::current_store(&state.data_dir)
}

#[tauri::command]
pub fn create_store(state: State<'_, AppState>, name: String) -> AppResult<StoreInfo> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Other("Nama toko wajib diisi.".into()));
    }
    stores::create_store(&state.data_dir, name.to_string())
}

/// Pindah toko aktif: buka koneksi baru ke database toko tsb (inisialisasi
/// skema bila baru), lalu tukar koneksi yang dipakai seluruh command lain.
#[tauri::command]
pub fn select_store(state: State<'_, AppState>, id: String) -> AppResult<StoreInfo> {
    let info = stores::set_active(&state.data_dir, &id)?;
    let path = stores::db_path(&state.data_dir, &info);
    let new_conn = Connection::open(&path)?;
    db::init_schema(&new_conn)?;
    db::seed_defaults(&new_conn)?;
    *state.lock()? = new_conn;
    Ok(info)
}

// ---------- Pengaturan ----------

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<HashMap<String, String>> {
    let conn = state.lock()?;
    Ok(db::all_settings(&conn)?.into_iter().collect())
}

#[tauri::command]
pub fn update_setting(state: State<'_, AppState>, key: String, value: String) -> AppResult<()> {
    let conn = state.lock()?;
    db::set_setting(&conn, &key, &value)
}

// ---------- Barang & stok ----------

#[tauri::command]
pub async fn list_products(
    state: State<'_, AppState>,
    search: Option<String>,
    include_inactive: Option<bool>,
    limit: Option<i64>,
) -> AppResult<Vec<ProductWithStock>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "list_products", serde_json::json!({
            "search": search, "include_inactive": include_inactive, "limit": limit
        })).await;
    }
    let conn = state.lock()?;
    db::list_products(&conn, search, include_inactive.unwrap_or(false), limit)
}

/// Versi paginasi untuk tabel Data Barang: filter merek, sort, limit/offset,
/// plus total baris (untuk kontrol halaman) agar tidak me-render semua data.
#[tauri::command]
pub async fn list_products_page(
    state: State<'_, AppState>,
    search: Option<String>,
    include_inactive: Option<bool>,
    brand: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    limit: i64,
    offset: i64,
) -> AppResult<crate::models::ProductPage> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "list_products_page", serde_json::json!({
            "search": search, "include_inactive": include_inactive, "brand": brand,
            "sort_by": sort_by, "sort_dir": sort_dir, "limit": limit, "offset": offset
        })).await;
    }
    let conn = state.lock()?;
    db::list_products_page(
        &conn,
        search,
        include_inactive.unwrap_or(false),
        brand,
        sort_by,
        sort_dir,
        limit,
        offset,
    )
}

#[tauri::command]
pub async fn save_product(state: State<'_, AppState>, input: ProductInput) -> AppResult<Product> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "save_product", serde_json::json!({ "input": input })).await;
    }
    let conn = state.lock()?;
    db::upsert_product(&conn, input)
}

#[tauri::command]
pub async fn toggle_product_active(
    state: State<'_, AppState>,
    id: String,
    active: bool,
) -> AppResult<()> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "toggle_product_active", serde_json::json!({
            "id": id, "active": active
        })).await;
    }
    let conn = state.lock()?;
    db::set_product_active(&conn, &id, active)
}

#[tauri::command]
pub async fn delete_product(state: State<'_, AppState>, id: String) -> AppResult<()> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "delete_product", serde_json::json!({ "id": id })).await;
    }
    let conn = state.lock()?;
    db::delete_product(&conn, &id)
}

/// Cari & non-aktifkan (soft-delete) barang dengan barcode kembar, sisakan
/// satu yang paling baru diubah. Dipakai setelah import batch yang berulang.
#[tauri::command]
pub async fn dedupe_products(state: State<'_, AppState>) -> AppResult<crate::models::DedupeResult> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "dedupe_products", serde_json::json!({})).await;
    }
    let conn = state.lock()?;
    db::dedupe_products_by_barcode(&conn)
}

/// Hapus seluruh data barang/stok/transaksi/diskon/merek. Akun & pengaturan
/// toko tetap dipertahankan. Aksi ini tidak bisa dibatalkan.
#[tauri::command]
pub fn reset_data(state: State<'_, AppState>, confirm: String) -> AppResult<()> {
    if confirm.trim() != "HAPUS SEMUA DATA" {
        return Err(AppError::Other("Konfirmasi tidak sesuai.".into()));
    }
    let conn = state.lock()?;
    db::reset_data(&conn)
}

#[tauri::command]
pub async fn find_by_barcode(
    state: State<'_, AppState>,
    barcode: String,
) -> AppResult<Option<ProductWithStock>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "find_by_barcode", serde_json::json!({ "barcode": barcode })).await;
    }
    let conn = state.lock()?;
    db::find_by_barcode(&conn, &barcode)
}

#[tauri::command]
pub async fn adjust_stock(state: State<'_, AppState>, product_id: String, delta: f64) -> AppResult<f64> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "adjust_stock", serde_json::json!({
            "product_id": product_id, "delta": delta
        })).await;
    }
    let conn = state.lock()?;
    db::adjust_stock(&conn, &product_id, delta)
}

#[tauri::command]
pub async fn set_stock(state: State<'_, AppState>, product_id: String, qty: f64) -> AppResult<f64> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "set_stock", serde_json::json!({
            "product_id": product_id, "qty": qty
        })).await;
    }
    let conn = state.lock()?;
    db::set_stock(&conn, &product_id, qty)
}

// ---------- Penjualan / kasir ----------

#[tauri::command]
pub async fn checkout(state: State<'_, AppState>, sale: SaleInput) -> AppResult<TransactionDetail> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "checkout", serde_json::json!({ "sale": sale })).await;
    }
    let mut conn = state.lock()?;
    db::create_sale(&mut conn, sale)
}

#[tauri::command]
pub async fn list_transactions(
    state: State<'_, AppState>,
    from: Option<String>,
    to: Option<String>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> AppResult<crate::models::TransactionPage> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "list_transactions", serde_json::json!({
            "from": from, "to": to, "search": search, "limit": limit, "offset": offset
        })).await;
    }
    let conn = state.lock()?;
    db::list_transactions(&conn, from, to, search, limit.unwrap_or(100), offset.unwrap_or(0))
}

#[tauri::command]
pub async fn get_transaction(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Option<TransactionDetail>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "get_transaction", serde_json::json!({ "id": id })).await;
    }
    let conn = state.lock()?;
    db::get_transaction(&conn, &id)
}

#[tauri::command]
pub async fn delete_transaction(state: State<'_, AppState>, id: String) -> AppResult<()> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "delete_transaction", serde_json::json!({ "id": id })).await;
    }
    let mut conn = state.lock()?;
    db::delete_transaction(&mut conn, &id)
}

#[tauri::command]
pub async fn update_transaction(
    state: State<'_, AppState>,
    id: String,
    input: SaleInput,
) -> AppResult<TransactionDetail> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "update_transaction", serde_json::json!({
            "id": id, "input": input
        })).await;
    }
    let mut conn = state.lock()?;
    db::update_transaction(&mut conn, &id, input)
}

// ---------- Pengguna / hak akses ----------

#[tauri::command]
pub async fn login(state: State<'_, AppState>, username: String, pin: String) -> AppResult<Option<User>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "login", serde_json::json!({
            "username": username, "pin": pin
        })).await;
    }
    let conn = state.lock()?;
    db::login(&conn, &username, &pin)
}

#[tauri::command]
pub async fn list_users(state: State<'_, AppState>) -> AppResult<Vec<User>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "list_users", serde_json::json!({})).await;
    }
    let conn = state.lock()?;
    db::list_users(&conn)
}

#[tauri::command]
pub async fn save_user(state: State<'_, AppState>, input: UserInput) -> AppResult<User> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "save_user", serde_json::json!({ "input": input })).await;
    }
    let conn = state.lock()?;
    db::save_user(&conn, input)
}

#[tauri::command]
pub async fn delete_user(state: State<'_, AppState>, id: String) -> AppResult<()> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "delete_user", serde_json::json!({ "id": id })).await;
    }
    let conn = state.lock()?;
    db::delete_user(&conn, &id)
}

// ---------- Pergerakan stok ----------

#[tauri::command]
pub async fn create_stock_movement(
    state: State<'_, AppState>,
    input: StockMovementInput,
) -> AppResult<StockMovement> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "create_stock_movement", serde_json::json!({ "input": input })).await;
    }
    let conn = state.lock()?;
    db::create_stock_movement(&conn, input)
}

#[tauri::command]
pub async fn list_stock_movements(
    state: State<'_, AppState>,
    kind: Option<String>,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
) -> AppResult<Vec<StockMovement>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "list_stock_movements", serde_json::json!({
            "kind": kind, "from": from, "to": to, "limit": limit
        })).await;
    }
    let conn = state.lock()?;
    db::list_stock_movements(&conn, kind, from, to, limit.unwrap_or(500))
}

#[tauri::command]
pub async fn delete_stock_movement(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "delete_stock_movement", serde_json::json!({ "id": id })).await;
    }
    let mut conn = state.lock()?;
    db::delete_stock_movement(&mut conn, id)
}

// ---------- Batch Item Masuk / Keluar ----------

#[tauri::command]
pub async fn create_stock_movement_batch(
    state: State<'_, AppState>,
    input: crate::models::StockMovementBatchInput,
) -> AppResult<crate::models::StockMovementBatchDetail> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "create_stock_movement_batch", serde_json::json!({ "input": input })).await;
    }
    let mut conn = state.lock()?;
    db::create_stock_movement_batch(&mut conn, input)
}

#[tauri::command]
pub async fn list_stock_movement_batches(
    state: State<'_, AppState>,
    kind: Option<String>,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> AppResult<crate::models::StockMovementBatchPage> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "list_stock_movement_batches", serde_json::json!({
            "kind": kind, "from": from, "to": to, "limit": limit, "offset": offset
        })).await;
    }
    let conn = state.lock()?;
    db::list_stock_movement_batches(&conn, kind, from, to, limit.unwrap_or(500), offset.unwrap_or(0))
}

#[tauri::command]
pub async fn get_stock_movement_batch(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Option<crate::models::StockMovementBatchDetail>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "get_stock_movement_batch", serde_json::json!({ "id": id })).await;
    }
    let conn = state.lock()?;
    db::get_stock_movement_batch(&conn, &id)
}

#[tauri::command]
pub async fn update_stock_movement_batch(
    state: State<'_, AppState>,
    id: String,
    items: Vec<crate::models::StockMovementBatchItemInput>,
    note: Option<String>,
) -> AppResult<crate::models::StockMovementBatchDetail> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "update_stock_movement_batch", serde_json::json!({
            "id": id, "items": items, "note": note
        })).await;
    }
    let mut conn = state.lock()?;
    db::update_stock_movement_batch(&mut conn, &id, items, note)
}

#[tauri::command]
pub async fn delete_stock_movement_batch(state: State<'_, AppState>, id: String) -> AppResult<()> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "delete_stock_movement_batch", serde_json::json!({ "id": id })).await;
    }
    let mut conn = state.lock()?;
    db::delete_stock_movement_batch(&mut conn, &id)
}

// ---------- Diskon periodik ----------

#[tauri::command]
pub async fn list_discounts(state: State<'_, AppState>) -> AppResult<Vec<DiscountPeriod>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "list_discounts", serde_json::json!({})).await;
    }
    let conn = state.lock()?;
    db::list_discounts(&conn)
}

#[tauri::command]
pub async fn save_discount(
    state: State<'_, AppState>,
    input: DiscountPeriodInput,
) -> AppResult<DiscountPeriod> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "save_discount", serde_json::json!({ "input": input })).await;
    }
    let conn = state.lock()?;
    db::save_discount(&conn, input)
}

#[tauri::command]
pub async fn delete_discount(state: State<'_, AppState>, id: String) -> AppResult<()> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "delete_discount", serde_json::json!({ "id": id })).await;
    }
    let conn = state.lock()?;
    db::delete_discount(&conn, &id)
}

// ---------- Merek (brand) ----------

#[tauri::command]
pub async fn list_brands(state: State<'_, AppState>) -> AppResult<Vec<crate::models::Brand>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "list_brands", serde_json::json!({})).await;
    }
    let conn = state.lock()?;
    db::list_brands(&conn)
}

#[tauri::command]
pub async fn save_brand(
    state: State<'_, AppState>,
    input: crate::models::BrandInput,
) -> AppResult<crate::models::Brand> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "save_brand", serde_json::json!({ "input": input })).await;
    }
    let conn = state.lock()?;
    db::save_brand(&conn, input)
}

#[tauri::command]
pub async fn delete_brand(state: State<'_, AppState>, id: String) -> AppResult<()> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "delete_brand", serde_json::json!({ "id": id })).await;
    }
    let conn = state.lock()?;
    db::delete_brand(&conn, &id)
}

// ---------- Pelanggan ----------

#[tauri::command]
pub async fn list_customers(
    state: State<'_, AppState>,
    search: Option<String>,
    include_inactive: Option<bool>,
) -> AppResult<Vec<Customer>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "list_customers", serde_json::json!({
            "search": search, "include_inactive": include_inactive
        })).await;
    }
    let conn = state.lock()?;
    db::list_customers(&conn, search, include_inactive.unwrap_or(false))
}

#[tauri::command]
pub async fn save_customer(state: State<'_, AppState>, input: CustomerInput) -> AppResult<Customer> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "save_customer", serde_json::json!({ "input": input })).await;
    }
    let conn = state.lock()?;
    db::upsert_customer(&conn, input)
}

#[tauri::command]
pub async fn delete_customer(state: State<'_, AppState>, id: String) -> AppResult<()> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "delete_customer", serde_json::json!({ "id": id })).await;
    }
    let conn = state.lock()?;
    db::delete_customer(&conn, &id)
}

// ---------- Pengeluaran (Kas Keluar) ----------

#[tauri::command]
pub async fn list_expenses(
    state: State<'_, AppState>,
    from: Option<String>,
    to: Option<String>,
) -> AppResult<Vec<Expense>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "list_expenses", serde_json::json!({
            "from": from, "to": to
        })).await;
    }
    let conn = state.lock()?;
    db::list_expenses(&conn, from, to)
}

#[tauri::command]
pub async fn save_expense(state: State<'_, AppState>, input: ExpenseInput) -> AppResult<Expense> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "save_expense", serde_json::json!({ "input": input })).await;
    }
    let conn = state.lock()?;
    db::save_expense(&conn, input)
}

#[tauri::command]
pub async fn delete_expense(state: State<'_, AppState>, id: String) -> AppResult<()> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "delete_expense", serde_json::json!({ "id": id })).await;
    }
    let conn = state.lock()?;
    db::delete_expense(&conn, &id)
}

// ---------- Shift kasir (buka/tutup, rekonsiliasi) ----------

#[tauri::command]
pub async fn get_active_shift(state: State<'_, AppState>) -> AppResult<Option<Shift>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "get_active_shift", serde_json::json!({})).await;
    }
    let conn = state.lock()?;
    db::get_active_shift(&conn)
}

#[tauri::command]
pub async fn open_shift(state: State<'_, AppState>, input: OpenShiftInput) -> AppResult<Shift> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "open_shift", serde_json::json!({ "input": input })).await;
    }
    let conn = state.lock()?;
    db::open_shift(&conn, input)
}

#[tauri::command]
pub async fn close_shift(state: State<'_, AppState>, input: CloseShiftInput) -> AppResult<Shift> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "close_shift", serde_json::json!({ "input": input })).await;
    }
    let conn = state.lock()?;
    db::close_shift(&conn, input)
}

#[tauri::command]
pub async fn list_shifts(state: State<'_, AppState>, limit: Option<i64>) -> AppResult<Vec<Shift>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "list_shifts", serde_json::json!({ "limit": limit })).await;
    }
    let conn = state.lock()?;
    db::list_shifts(&conn, limit.unwrap_or(100))
}

// ---------- Laporan per barang / per merek ----------

#[tauri::command]
pub async fn product_sales_report(
    state: State<'_, AppState>,
    from: String,
    to: String,
    brands: Vec<String>,
) -> AppResult<Vec<ProductSalesRow>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "product_sales_report", serde_json::json!({
            "from": from, "to": to, "brands": brands
        })).await;
    }
    let conn = state.lock()?;
    db::product_sales_report(&conn, &from, &to, &brands)
}

#[tauri::command]
pub async fn brand_sales_report(
    state: State<'_, AppState>,
    from: String,
    to: String,
    brands: Vec<String>,
) -> AppResult<Vec<BrandSalesRow>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "brand_sales_report", serde_json::json!({
            "from": from, "to": to, "brands": brands
        })).await;
    }
    let conn = state.lock()?;
    db::brand_sales_report(&conn, &from, &to, &brands)
}

#[tauri::command]
pub async fn sales_item_detail_report(
    state: State<'_, AppState>,
    from: String,
    to: String,
    brands: Vec<String>,
) -> AppResult<Vec<crate::models::SalesItemDetailRow>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "sales_item_detail_report", serde_json::json!({
            "from": from, "to": to, "brands": brands
        })).await;
    }
    let conn = state.lock()?;
    db::sales_item_detail_report(&conn, &from, &to, &brands)
}

#[tauri::command]
pub async fn daily_sales_report(
    state: State<'_, AppState>,
    from: String,
    to: String,
    brands: Vec<String>,
) -> AppResult<Vec<crate::models::DailySalesRow>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "daily_sales_report", serde_json::json!({
            "from": from, "to": to, "brands": brands
        })).await;
    }
    let conn = state.lock()?;
    db::daily_sales_report(&conn, &from, &to, &brands)
}

// ---------- File & Printer (sistem) ----------

/// Tulis file biner ke folder temp lalu set read-only (Excel buka mode read-only).
#[tauri::command]
pub fn write_temp_file(file_name: String, bytes: Vec<u8>) -> AppResult<String> {
    use std::io::Write;
    let mut dir = std::env::temp_dir();
    dir.push("galaxyas");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(&file_name);

    // Jika file lama read-only, lepas dulu agar bisa ditimpa.
    if path.exists() {
        if let Ok(meta) = std::fs::metadata(&path) {
            let mut perms = meta.permissions();
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(&path, perms);
        }
        let _ = std::fs::remove_file(&path);
    }

    {
        let mut f = std::fs::File::create(&path)?;
        f.write_all(&bytes)?;
    }
    if let Ok(meta) = std::fs::metadata(&path) {
        let mut perms = meta.permissions();
        perms.set_readonly(true);
        let _ = std::fs::set_permissions(&path, perms);
    }
    Ok(path.to_string_lossy().to_string())
}

/// Daftar printer yang terpasang (Windows). Kosong pada OS lain.
#[tauri::command]
pub fn list_printers() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-Command",
            "Get-Printer | Select-Object -ExpandProperty Name",
        ]);
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        if let Ok(o) = cmd.output() {
            let s = String::from_utf8_lossy(&o.stdout);
            return s
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
        }
    }
    Vec::new()
}

/// Cetak teks polos ke printer terpilih (Windows, via Out-Printer). Best-effort.
#[tauri::command]
pub fn print_text_to(printer: Option<String>, text: String) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut dir = std::env::temp_dir();
        dir.push("galaxyas");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("struk_print.txt");
        std::fs::write(&path, text.as_bytes())?;

        // Get-Content (Windows PowerShell 5.1) membaca file sebagai ANSI secara
        // default sehingga teks UTF-8 tercetak acak. Paksa baca sebagai UTF-8.
        let ps = match printer {
            Some(p) if !p.trim().is_empty() => format!(
                "Get-Content -Raw -Encoding UTF8 -LiteralPath '{}' | Out-Printer -Name '{}'",
                path.display(),
                p.replace('\'', "''")
            ),
            _ => format!(
                "Get-Content -Raw -Encoding UTF8 -LiteralPath '{}' | Out-Printer",
                path.display()
            ),
        };
        let mut cmd = std::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-Command", &ps]);
        cmd.creation_flags(0x0800_0000);
        cmd.output()?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (printer, text);
    }
    Ok(())
}

/// Cetak byte ESC/POS mentah langsung ke spooler (RAW), untuk struk thermal
/// dengan format/autocut yang stabil (tidak melalui driver GDI Out-Printer).
#[tauri::command]
pub fn print_escpos_to(printer: Option<String>, bytes: Vec<u8>) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        crate::winprint::print_raw(printer.as_deref(), &bytes)?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (printer, bytes);
    }
    Ok(())
}

// ---------- Window cetak terpisah ----------

/// Konten cetak yang dititipkan ke window print terpisah, diambil sekali oleh
/// halaman `print.html` saat window tersebut dimuat.
#[derive(Clone, serde::Serialize)]
pub struct PrintPayload {
    pub html: String,
    pub css: String,
}

#[derive(Default)]
pub struct PrintPayloadState(pub Mutex<HashMap<String, PrintPayload>>);

/// Buka window baru khusus preview/cetak (title bar & tombol tutup sendiri,
/// terpisah dari window utama) — supaya tombol X window utama tidak pernah
/// tertumpuk dialog cetak bawaan OS.
///
/// PENTING: command ini WAJIB `async`. Membuat WebviewWindow dari command
/// sinkron menyebabkan deadlock di Windows (event loop webview2 menunggu
/// command selesai, command menunggu window jadi) — ini penyebab freeze
/// "not responding" pada percobaan pertama fitur ini.
#[tauri::command]
pub async fn open_print_window(
    app: tauri::AppHandle,
    html: String,
    css: String,
    title: String,
    width: f64,
    height: f64,
) -> AppResult<()> {
    use tauri::Manager;

    let label = format!("print-{}", uuid::Uuid::new_v4().simple());
    {
        let state = app.state::<PrintPayloadState>();
        let mut map = state
            .0
            .lock()
            .map_err(|_| AppError::Other("gagal mengunci payload cetak".into()))?;
        map.insert(label.clone(), PrintPayload { html, css });
    }

    let url = format!("print.html?label={label}");
    tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::App(url.into()))
        .title(title)
        .inner_size(width, height)
        .center()
        .build()
        .map_err(|e| AppError::Other(format!("gagal membuka window cetak: {e}")))?;

    Ok(())
}

/// Diambil oleh window cetak saat dimuat; sekali diambil, payload dihapus dari state.
#[tauri::command]
pub fn take_print_payload(state: State<'_, PrintPayloadState>, label: String) -> AppResult<PrintPayload> {
    state
        .0
        .lock()
        .map_err(|_| AppError::Other("gagal mengunci payload cetak".into()))?
        .remove(&label)
        .ok_or_else(|| AppError::Other("payload cetak tidak ditemukan (window mungkin dimuat ulang)".into()))
}

// ---------- Sinkronisasi (manual) ----------

fn read_sync_config(state: &State<'_, AppState>) -> AppResult<(String, String, String)> {
    let conn = state.lock()?;
    let server_url = db::get_setting(&conn, "server_url")?
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::Config("server_url belum diatur".into()))?;
    let store_id = db::get_setting(&conn, "store_id")?
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::Config("store_id belum diatur".into()))?;
    let last_pull = db::get_setting(&conn, "last_pull_at")?.unwrap_or_default();
    Ok((server_url, store_id, last_pull))
}

/// Kirim Data ke Server (Upload): kirim perubahan master data lokal.
#[tauri::command]
pub async fn sync_push(state: State<'_, AppState>) -> AppResult<SyncResult> {
    let (server_url, store_id, _) = read_sync_config(&state)?;

    let dirty: Vec<Product> = {
        let conn = state.lock()?;
        db::get_dirty_products(&conn)?
    };

    if dirty.is_empty() {
        return Ok(SyncResult {
            message: "Tidak ada perubahan untuk dikirim.".into(),
            ..Default::default()
        });
    }

    let resp = sync::push(&server_url, &store_id, &dirty).await?;

    let ids: Vec<String> = dirty.iter().map(|p| p.id.clone()).collect();
    {
        let conn = state.lock()?;
        db::mark_products_synced(&conn, &ids)?;
    }

    Ok(SyncResult {
        pushed: resp.applied,
        skipped: resp.skipped,
        pulled: 0,
        message: format!(
            "Upload selesai: {} diterapkan, {} dilewati (server lebih baru).",
            resp.applied, resp.skipped
        ),
    })
}

/// Ambil Update dari Server (Download): tarik master data terbaru (Delta Sync).
#[tauri::command]
pub async fn sync_pull(state: State<'_, AppState>) -> AppResult<SyncResult> {
    let (server_url, store_id, last_pull) = read_sync_config(&state)?;

    let since = if last_pull.is_empty() { None } else { Some(last_pull.as_str()) };
    let resp = sync::pull(&server_url, &store_id, since).await?;

    let (applied, skipped) = {
        let conn = state.lock()?;
        let res = db::apply_pulled_products(&conn, &resp.products)?;
        // Catat waktu klien sebagai watermark pull berikutnya.
        db::set_setting(&conn, "last_pull_at", &Utc::now().to_rfc3339())?;
        res
    };

    Ok(SyncResult {
        pulled: applied,
        skipped,
        pushed: 0,
        message: format!(
            "Download selesai: {} diperbarui, {} dilewati (lokal lebih baru).",
            applied, skipped
        ),
    })
}

/// Sinkronisasi penuh: upload dulu, lalu download.
#[tauri::command]
pub async fn sync_all(state: State<'_, AppState>) -> AppResult<SyncResult> {
    let push_res = sync_push(state.clone()).await?;
    let pull_res = sync_pull(state).await?;
    Ok(SyncResult {
        pushed: push_res.pushed,
        pulled: pull_res.pulled,
        skipped: push_res.skipped + pull_res.skipped,
        message: format!("{} {}", push_res.message, pull_res.message),
    })
}

// ---------- Bridge: Pull dari app mobile (galaxyas-mobile, fase 6) ----------
//
// TERPISAH TOTAL dari sync di atas: bridge ini bicara ke server GALAXYAS
// Mobile (database & auth beda) buat narik batch pengiriman yang dicatat
// Pengirim lewat app mobile, lalu menuliskannya ke stock_movements lokal —
// sama seperti "Simpan Semua Item" manual di Item Masuk, cuma sumber datanya
// dari Pull, bukan input tangan. Dipanggil oleh menu Item Masuk (Pull.svelte).

fn read_bridge_config(state: &State<'_, AppState>) -> AppResult<(String, String)> {
    let conn = state.lock()?;
    let server_url = db::get_setting(&conn, "mobile_server_url")?
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::Config("URL server mobile belum diatur (menu Pengaturan).".into()))?;
    let api_key = db::get_setting(&conn, "mobile_store_api_key")?
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::Config("API key toko belum diatur (menu Pengaturan).".into()))?;
    Ok((server_url, api_key))
}

/// Cari produk lokal by barcode, atau buat baru kalau belum ada (barang baru
/// dari Pengirim yang belum pernah tercatat di POS toko ini). Menghormati
/// mode Server Pusat (LAN) sama seperti `find_by_barcode`/`save_product`.
async fn resolve_or_create_product(
    state: &State<'_, AppState>,
    barcode: &str,
    fallback_name: &str,
) -> AppResult<String> {
    if let Some(remote) = state.remote_config() {
        let existing: Option<ProductWithStock> =
            crate::lan::call(&remote, "find_by_barcode", serde_json::json!({ "barcode": barcode })).await?;
        if let Some(p) = existing {
            return Ok(p.product.id);
        }
        let input = ProductInput {
            id: None,
            name: fallback_name.to_string(),
            barcode: Some(barcode.to_string()),
            category: None,
            brand: None,
            unit: None,
            sell_price: 0.0,
            cost_price: 0.0,
            default_discount: 0.0,
            is_active: true,
        };
        let created: Product =
            crate::lan::call(&remote, "save_product", serde_json::json!({ "input": input })).await?;
        return Ok(created.id);
    }

    let conn = state.lock()?;
    if let Some(existing) = db::find_by_barcode(&conn, barcode)? {
        return Ok(existing.product.id);
    }
    let created = db::upsert_product(
        &conn,
        ProductInput {
            id: None,
            name: fallback_name.to_string(),
            barcode: Some(barcode.to_string()),
            category: None,
            brand: None,
            unit: None,
            sell_price: 0.0,
            cost_price: 0.0,
            default_discount: 0.0,
            is_active: true,
        },
    )?;
    Ok(created.id)
}

/// Cek apakah barcode ini match ke produk yang sudah ada di database toko
/// ini — dipakai bridge_list_pending buat nandain barcode "gak dikenal" ke
/// frontend (⚠), bukan buat resolusi produk aslinya (itu tetap di
/// resolve_or_create_product saat konfirmasi).
async fn is_known_locally(state: &State<'_, AppState>, barcode: &str) -> AppResult<bool> {
    if let Some(remote) = state.remote_config() {
        let existing: Option<ProductWithStock> =
            crate::lan::call(&remote, "find_by_barcode", serde_json::json!({ "barcode": barcode })).await?;
        return Ok(existing.is_some());
    }
    let conn = state.lock()?;
    Ok(db::find_by_barcode(&conn, barcode)?.is_some())
}

/// Daftar baris menu Barang yang masih menunggu di-pull buat toko ini
/// (barcode+qty dikirim sudah keisi di app mobile). App mobile TIDAK connect
/// ke katalog POS — nama barang di sana cuma referensi manual, jadi di sini
/// tiap barcode dicek lokal & ditandai kalau belum dikenal produk POS toko ini.
#[tauri::command]
pub async fn bridge_list_pending(state: State<'_, AppState>) -> AppResult<Vec<crate::pull::PullItem>> {
    let (server_url, api_key) = read_bridge_config(&state)?;
    let mut items = crate::pull::list_pending(&server_url, &api_key).await?;
    for item in &mut items {
        item.known_locally = is_known_locally(&state, &item.barcode).await?;
    }
    Ok(items)
}

/// Konfirmasi sejumlah baris sekaligus: `items` dikirim balik dari frontend
/// dengan qty_dikirim yang SUDAH dikoreksi kasir kalau perlu (beda dari yang
/// diklaim Pengirim). Nulis satu batch stock_movements lokal (kind=in) —
/// tiap baris di-tag `[pull-row:<id>]` per item, baru menandai baris-baris
/// itu `confirmed` di server.
#[tauri::command]
pub async fn bridge_confirm_pull(
    state: State<'_, AppState>,
    items: Vec<crate::pull::PullItem>,
    user_id: Option<String>,
) -> AppResult<crate::models::StockMovementBatchDetail> {
    if items.is_empty() {
        return Err(AppError::Other("Tidak ada baris yang dipilih.".into()));
    }

    {
        let conn = state.lock()?;
        for item in &items {
            let tag = format!("[pull-row:{}]", item.id);
            if db::stock_movement_note_contains(&conn, &tag)? {
                return Err(AppError::Other(format!(
                    "Baris {} sudah pernah ditulis ke stok lokal (kemungkinan konfirmasi ke server sempat \
                     gagal di percobaan sebelumnya). Cek Daftar Item Masuk & hubungi Bos — jangan pull ulang.",
                    item.barcode
                )));
            }
        }
    }

    let mut batch_items = Vec::with_capacity(items.len());
    for item in &items {
        let fallback_name = item.name.clone().unwrap_or_else(|| item.barcode.clone());
        let product_id = resolve_or_create_product(&state, &item.barcode, &fallback_name).await?;
        batch_items.push(crate::models::StockMovementBatchItemInput {
            product_id,
            qty: item.qty_dikirim,
            note: Some(format!("[pull-row:{}] dari app mobile", item.id)),
        });
    }

    let input = crate::models::StockMovementBatchInput {
        kind: "in".into(),
        note: Some("Pull dari app mobile".into()),
        user_id,
        items: batch_items,
        created_at: None,
    };

    let detail = if let Some(remote) = state.remote_config() {
        crate::lan::call(&remote, "create_stock_movement_batch", serde_json::json!({ "input": input })).await?
    } else {
        let mut conn = state.lock()?;
        db::create_stock_movement_batch(&mut conn, input)?
    };

    // Stok lokal sudah aman ke-tulis di atas. Kalau langkah di bawah ini
    // gagal (mis. jaringan putus), baris-baris ini tetap `pending` di server
    // — guard idempotensi di atas mencegah pull berikutnya nulis dobel ke stok.
    let (server_url, api_key) = read_bridge_config(&state)?;
    let confirmed_items: Vec<(String, f64)> = items.iter().map(|i| (i.id.clone(), i.qty_dikirim)).collect();
    crate::pull::confirm(&server_url, &api_key, &confirmed_items).await?;

    Ok(detail)
}

/// Tolak 1 baris (mis. ternyata salah kirim ke toko ini) — TIDAK menulis apa
/// pun ke stok lokal.
#[tauri::command]
pub async fn bridge_reject_pull(state: State<'_, AppState>, row_id: String) -> AppResult<()> {
    let (server_url, api_key) = read_bridge_config(&state)?;
    crate::pull::reject(&server_url, &api_key, &row_id).await?;
    Ok(())
}

// ---------- Server Pusat (LAN) ----------

/// Port tetap untuk Server Pusat (dipakai host maupun ditampilkan ke user
/// saat pairing manual bila diperlukan).
const LAN_SERVER_PORT: u16 = 8899;

#[tauri::command]
pub fn list_servers(state: State<'_, AppState>) -> AppResult<Vec<crate::servers::ServerInfo>> {
    crate::servers::list_servers(&state.data_dir)
}

#[tauri::command]
pub fn current_server(state: State<'_, AppState>) -> AppResult<crate::servers::ServerInfo> {
    crate::servers::current_server(&state.data_dir)
}

/// Cek keterjangkauan + validitas kode pairing sebuah Server Pusat, dipakai
/// layar "+ Tambah Server" sebelum menyimpan.
#[tauri::command]
pub async fn ping_server(host: String, port: u16, token: String) -> AppResult<String> {
    crate::lan::health_check(&host, port, &token).await
}

#[tauri::command]
pub async fn add_server(
    state: State<'_, AppState>,
    name: String,
    host: String,
    port: u16,
    token: String,
) -> AppResult<crate::servers::ServerInfo> {
    // Validasi keterjangkauan + kode pairing sebelum disimpan ke registry.
    crate::lan::health_check(&host, port, &token).await?;
    crate::servers::add_server(&state.data_dir, name, host, port, token)
}

/// Pindah server aktif (lokal <-> salah satu remote tersimpan). Men-set/
/// meng-clear `AppState.remote` supaya command yang di-proxy langsung
/// mengarah ke tempat yang benar setelahnya.
#[tauri::command]
pub fn select_server(state: State<'_, AppState>, id: String) -> AppResult<crate::servers::ServerInfo> {
    let info = crate::servers::set_active(&state.data_dir, &id)?;
    let mut remote = state
        .remote
        .lock()
        .map_err(|_| AppError::Other("gagal mengunci state".into()))?;
    *remote = if info.is_remote() {
        Some(crate::lan::RemoteConfig {
            base_url: format!(
                "http://{}:{}",
                info.host.clone().unwrap_or_default(),
                info.port.unwrap_or(LAN_SERVER_PORT)
            ),
            token: info.token.clone().unwrap_or_default(),
        })
    } else {
        None
    };
    Ok(info)
}

#[tauri::command]
pub fn remove_server(state: State<'_, AppState>, id: String) -> AppResult<()> {
    crate::servers::remove_server(&state.data_dir, &id)
}

#[derive(serde::Serialize)]
pub struct LanServerStatus {
    pub enabled: bool,
    pub port: u16,
    pub token: String,
    pub local_ip: Option<String>,
}

fn lan_status_snapshot(state: &State<'_, AppState>) -> AppResult<LanServerStatus> {
    let conn = state.lock()?;
    let enabled = db::get_setting(&conn, "lan_server_enabled")?.as_deref() == Some("1");
    let token = db::get_setting(&conn, "lan_server_token")?.unwrap_or_default();
    drop(conn);
    let local_ip = local_ip_address::local_ip().ok().map(|ip| ip.to_string());
    Ok(LanServerStatus { enabled, port: LAN_SERVER_PORT, token, local_ip })
}

#[tauri::command]
pub fn lan_server_status(state: State<'_, AppState>) -> AppResult<LanServerStatus> {
    lan_status_snapshot(&state)
}

/// Nyalakan/matikan server host "Server Pusat" pada PC ini. Menghasilkan
/// token pairing baru bila belum ada token tersimpan.
#[tauri::command]
pub fn set_lan_server_enabled(state: State<'_, AppState>, enabled: bool) -> AppResult<LanServerStatus> {
    // Hentikan instance lama (bila ada) sebelum mengubah pengaturan.
    {
        let mut lan = state.lan.lock().map_err(|_| AppError::Other("gagal mengunci state".into()))?;
        if let Some(handle) = lan.take() {
            handle.stop();
        }
    }
    let conn = state.lock()?;
    db::set_setting(&conn, "lan_server_enabled", if enabled { "1" } else { "0" })?;
    let mut token = db::get_setting(&conn, "lan_server_token")?.unwrap_or_default();
    if enabled && token.is_empty() {
        token = crate::lan::generate_token();
        db::set_setting(&conn, "lan_server_token", &token)?;
    }
    drop(conn);
    if enabled {
        let handle = crate::lan::start(state.conn.clone(), LAN_SERVER_PORT, token)?;
        let mut lan = state.lan.lock().map_err(|_| AppError::Other("gagal mengunci state".into()))?;
        *lan = Some(handle);
    }
    lan_status_snapshot(&state)
}

/// Buat token pairing baru (mis. dicurigai bocor) dan restart server bila
/// sedang aktif, supaya token lama langsung tidak berlaku lagi.
#[tauri::command]
pub fn regenerate_lan_token(state: State<'_, AppState>) -> AppResult<LanServerStatus> {
    let new_token = crate::lan::generate_token();
    let conn = state.lock()?;
    db::set_setting(&conn, "lan_server_token", &new_token)?;
    let was_enabled = db::get_setting(&conn, "lan_server_enabled")?.as_deref() == Some("1");
    drop(conn);
    if was_enabled {
        let mut lan = state.lan.lock().map_err(|_| AppError::Other("gagal mengunci state".into()))?;
        if let Some(handle) = lan.take() {
            handle.stop();
        }
        let handle = crate::lan::start(state.conn.clone(), LAN_SERVER_PORT, new_token)?;
        *lan = Some(handle);
    }
    lan_status_snapshot(&state)
}
