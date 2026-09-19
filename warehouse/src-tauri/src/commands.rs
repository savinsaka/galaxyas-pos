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
    ProductWithStock, SaleInput, Shift, StockMovement, StockMovementInput, StoreInfo, SyncLogEntry,
    SyncResult, TransactionDetail, User, UserInput,
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
    /// Handle agent relay (akses HP dari luar wifi toko), bila diaktifkan.
    pub relay: Mutex<Option<crate::relay::RelayHandle>>,
    /// Status agent relay yang dibaca layar Pengaturan; ikut ditulis oleh loop
    /// agent di background, jadi sengaja dibagi lewat Arc.
    pub relay_status: Arc<Mutex<crate::relay::RelayStatus>>,
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

// ---------- Migrasi toko (berkas .gpos) ----------

/// Letak berkas hasil ekspor + laporannya.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationExport {
    pub path: String,
    pub hasil: crate::migrasi::HasilMigrasi,
}

/// Migrasi selalu bekerja pada **database PC ini**, jadi ia dilarang saat PC ini
/// sedang jadi klien Server Pusat.
///
/// Kalau tidak dijaga, ekspor akan mengambil isi SQLite lokal yang sejak PC ini
/// jadi klien tidak lagi dipakai berjualan — berkasnya jadi kosong atau basi,
/// dan tidak ada satu pun galat yang mengatakannya. Impor lebih buruk: ia
/// menulis ke database yang tidak dibaca siapa pun, lalu terlihat berhasil.
fn hanya_database_pc_ini(state: &State<'_, AppState>) -> AppResult<()> {
    if state.remote_config().is_some() {
        return Err(AppError::Other(
            "PC ini sedang memakai database PC pusat, jadi migrasi harus dijalankan di PC \
             pusat itu — bukan dari sini."
                .into(),
        ));
    }
    Ok(())
}

/// Seluruh isi toko yang sedang aktif, jadi satu berkas `.gpos`.
///
/// Berkasnya ditulis di sini dan bukan di frontend supaya isinya tidak pernah
/// melewati webview: ia berisi seluruh harga modal dan riwayat toko.
#[tauri::command]
pub fn migration_export(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<MigrationExport> {
    use tauri::Manager;

    hanya_database_pc_ini(&state)?;

    let toko = stores::current_store(&state.data_dir)?;
    let (data, mut hasil) = {
        let conn = state.lock()?;
        crate::migrasi::ekspor(&conn, &toko.name)?
    };

    // Folder tetap di Dokumen, bukan folder temp: berkas ini justru dibuat
    // untuk disimpan dan dikirim, dan yang di temp hilang tanpa pemberitahuan.
    let folder = app
        .path()
        .document_dir()
        .unwrap_or_else(|_| state.data_dir.clone())
        .join("GALAXYAS POS Migrasi");
    std::fs::create_dir_all(&folder)?;

    let path = folder.join(crate::migrasi::nama_berkas(&toko.name));
    std::fs::write(&path, &data)?;

    hasil.berkas = path.to_string_lossy().into_owned();
    Ok(MigrationExport { path: hasil.berkas.clone(), hasil })
}

/// Asal-usul berkas **tanpa mengimpornya** — untuk ditampilkan sebelum
/// konfirmasi. Isi berkas tidak didekripsi, hanya kepalanya yang memang terbuka.
#[tauri::command]
pub fn migration_inspect(bytes: Vec<u8>) -> AppResult<crate::migrasi::SumberBerkas> {
    crate::migrasi::periksa(&bytes)
}

/// **Ganti** seluruh isi toko aktif dengan isi berkas. Tidak bisa dibatalkan.
///
/// Yang menahan salah pencet bukan command ini melainkan dua hal lain: kalimat
/// yang harus diketik ulang di layar, dan cadangan database yang dibuat sebelum
/// baris pertama dihapus. Letak cadangannya ikut di jawaban — itu jalan
/// pulangnya.
#[tauri::command]
pub fn migration_import(
    state: State<'_, AppState>,
    bytes: Vec<u8>,
    confirm: String,
) -> AppResult<crate::migrasi::HasilMigrasi> {
    if confirm.trim() != "GANTI SEMUA DATA" {
        return Err(AppError::Other("Konfirmasi tidak sesuai.".into()));
    }
    hanya_database_pc_ini(&state)?;

    let toko = stores::current_store(&state.data_dir)?;
    let path = stores::db_path(&state.data_dir, &toko);
    let mut conn = state.lock()?;
    crate::migrasi::impor(&mut conn, &path, &bytes)
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

/// Opname Spesial: hitung sebagian barang satu merek, sisanya dinolkan.
/// Sengaja satu perintah (bukan loop create_stock_movement dari UI) supaya
/// atomik dan tetap cepat lewat LAN/relay walau mereknya ratusan barang.
#[tauri::command]
pub async fn create_opname_special(
    state: State<'_, AppState>,
    input: crate::models::OpnameSpecialInput,
) -> AppResult<crate::models::OpnameSpecialResult> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "create_opname_special", serde_json::json!({ "input": input })).await;
    }
    let mut conn = state.lock()?;
    db::create_opname_special(&mut conn, input)
}

/// Time Opname: catat hasil hitung pada titik waktu tertentu di masa lalu.
/// Pratinjau dulu (murni baca) supaya pemakai lihat dampaknya ke stok hari ini
/// sebelum menyimpan — opname tidak bisa dibatalkan.
#[tauri::command]
pub async fn preview_time_opname(
    state: State<'_, AppState>,
    input: crate::models::TimeOpnameInput,
) -> AppResult<Vec<crate::models::TimeOpnameRow>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "preview_time_opname", serde_json::json!({ "input": input })).await;
    }
    let conn = state.lock()?;
    db::preview_time_opname(&conn, input)
}

#[tauri::command]
pub async fn create_time_opname(
    state: State<'_, AppState>,
    input: crate::models::TimeOpnameInput,
) -> AppResult<crate::models::TimeOpnameResult> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "create_time_opname", serde_json::json!({ "input": input })).await;
    }
    let mut conn = state.lock()?;
    db::create_time_opname(&mut conn, input)
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

// ---------- Alur Barang (buku besar stok) ----------

#[tauri::command]
pub async fn stock_flow_recap(
    state: State<'_, AppState>,
    from: String,
    to: String,
    search: Option<String>,
) -> AppResult<Vec<crate::models::StockFlowRow>> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "stock_flow_recap", serde_json::json!({
            "from": from, "to": to, "search": search
        })).await;
    }
    let conn = state.lock()?;
    db::stock_flow_recap(&conn, &from, &to, search)
}

#[tauri::command]
pub async fn stock_flow_detail(
    state: State<'_, AppState>,
    product_id: String,
    from: String,
    to: String,
) -> AppResult<crate::models::StockFlowDetail> {
    if let Some(remote) = state.remote_config() {
        return crate::lan::call(&remote, "stock_flow_detail", serde_json::json!({
            "product_id": product_id, "from": from, "to": to
        })).await;
    }
    let conn = state.lock()?;
    db::stock_flow_detail(&conn, &product_id, &from, &to)
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

    // Server tidak mengembalikan breakdown per-ID (cuma agregat applied/skipped),
    // jadi log push cuma bisa melaporkan apa yang DIKIRIM, bukan status per-item.
    let log: Vec<SyncLogEntry> = dirty
        .iter()
        .map(|p| SyncLogEntry { id: p.id.clone(), name: p.name.clone(), action: "Dikirim".into() })
        .collect();

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
        log,
    })
}

/// Ambil Update dari Server (Download): tarik master data terbaru (Delta Sync).
#[tauri::command]
pub async fn sync_pull(state: State<'_, AppState>) -> AppResult<SyncResult> {
    let (server_url, store_id, last_pull) = read_sync_config(&state)?;

    let since = if last_pull.is_empty() { None } else { Some(last_pull.as_str()) };
    let resp = sync::pull(&server_url, &store_id, since).await?;

    let (applied, skipped, log) = {
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
        log,
    })
}

/// Sinkronisasi penuh: upload dulu, lalu download.
#[tauri::command]
pub async fn sync_all(state: State<'_, AppState>) -> AppResult<SyncResult> {
    let push_res = sync_push(state.clone()).await?;
    let pull_res = sync_pull(state).await?;
    let mut log = push_res.log;
    log.extend(pull_res.log);
    Ok(SyncResult {
        pushed: push_res.pushed,
        pulled: pull_res.pulled,
        skipped: push_res.skipped + pull_res.skipped,
        message: format!("{} {}", push_res.message, pull_res.message),
        log,
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

// ---------- Server Pusat (wifi & internet) ----------

use crate::servers::{ServerInfo, ServerPath};

/// Port tetap untuk Server Pusat (dipakai host maupun ditampilkan ke user
/// saat pairing manual bila diperlukan).
const LAN_SERVER_PORT: u16 = crate::servers::DEFAULT_LAN_PORT;

/// Isian layar "+ Tambah Server" di PC klien. Satu entry bisa membawa kedua
/// alamat sekaligus (wifi & internet) — mana yang dipakai ditentukan `path`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerInput {
    pub name: String,
    #[serde(default)]
    pub lan_host: String,
    #[serde(default)]
    pub lan_port: Option<u16>,
    #[serde(default)]
    pub relay_url: String,
    #[serde(default)]
    pub store_id: String,
    /// Kode pairing 6 karakter dari Pengaturan PC pusat.
    #[serde(default)]
    pub code: String,
    /// "lan" | "online".
    #[serde(default)]
    pub path: String,
}

fn trimmed_opt(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

impl ServerInput {
    fn path(&self) -> ServerPath {
        ServerPath::parse(&self.path)
    }

    /// Entry calon (token menyusul dari pairing) — dipakai untuk menyusun base
    /// URL lewat satu-satunya sumber kebenaran, `ServerInfo::base_url`.
    fn draft(&self, device_token: String) -> ServerInfo {
        ServerInfo::new_remote(
            trimmed_opt(&self.name).unwrap_or_else(|| "Server Pusat".to_string()),
            trimmed_opt(&self.lan_host),
            Some(self.lan_port.unwrap_or(LAN_SERVER_PORT)),
            trimmed_opt(&self.relay_url),
            trimmed_opt(&self.store_id),
            device_token,
        )
    }
}

fn path_label(path: ServerPath) -> &'static str {
    if path.is_online() {
        "internet"
    } else {
        "wifi"
    }
}

fn missing_address_err(path: ServerPath) -> AppError {
    AppError::Other(match path {
        ServerPath::Lan => "IP Server Pusat wajib diisi untuk jalur wifi.".to_string(),
        ServerPath::Online => {
            "Alamat Relay dan Store ID wajib diisi untuk jalur internet.".to_string()
        }
    })
}

/// Nama PC ini, untuk daftar "Perangkat Terhubung" di PC pusat — supaya PC
/// klien bisa dibedakan dari HP dan dicabut satu-satu.
fn client_device_name() -> String {
    let host = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default();
    let host = host.trim().to_string();
    if host.is_empty() {
        "PC Kasir".to_string()
    } else {
        format!("PC {host}")
    }
}

#[tauri::command]
pub fn list_servers(state: State<'_, AppState>) -> AppResult<Vec<ServerInfo>> {
    crate::servers::list_servers(&state.data_dir)
}

#[tauri::command]
pub fn current_server(state: State<'_, AppState>) -> AppResult<crate::servers::ActiveServer> {
    crate::servers::current_active(&state.data_dir)
}

/// Cek keterjangkauan Server Pusat lewat jalur terpilih, dipakai tombol
/// "Uji Koneksi" di layar "+ Tambah Server" sebelum menyimpan apa pun.
#[tauri::command]
pub async fn ping_server(input: ServerInput) -> AppResult<String> {
    let path = input.path();
    let base = input
        .draft(String::new())
        .base_url(path)
        .ok_or_else(|| missing_address_err(path))?;
    crate::lan::health_check(&base, input.code.trim(), path.is_online()).await
}

/// Daftarkan PC ini ke sebuah Server Pusat: tukar kode pairing 6 karakter
/// dengan token perangkat 64-hex, lalu simpan entry berisi KEDUA alamat.
/// Token panjang itu wajib untuk jalur internet (kode pendek ditolak di sana,
/// lihat `lan::authenticate`) dan tetap sah di jalur wifi, jadi satu kali
/// pendaftaran cukup untuk dua jalur.
#[tauri::command]
pub async fn add_server(
    state: State<'_, AppState>,
    input: ServerInput,
) -> AppResult<ServerInfo> {
    let path = input.path();
    let draft = input.draft(String::new());
    if !draft.supports(path) {
        return Err(missing_address_err(path));
    }
    if input.code.trim().is_empty() {
        return Err(AppError::Other("Kode Pairing wajib diisi.".into()));
    }

    let device_name = client_device_name();
    let base = draft.base_url(path).ok_or_else(|| missing_address_err(path))?;
    let paired =
        match crate::lan::pair_client(&base, &input.code, &device_name, path.is_online()).await {
            Ok(result) => result,
            Err(first) => {
                // Alamat jalur lain ikut diisi? coba juga — kasir yang salah
                // memilih jalur tidak perlu mengisi form dari awal.
                let other = if path.is_online() { ServerPath::Lan } else { ServerPath::Online };
                match draft.base_url(other) {
                    Some(other_base) => crate::lan::pair_client(
                        &other_base,
                        &input.code,
                        &device_name,
                        other.is_online(),
                    )
                    .await
                    .map_err(|second| {
                        AppError::Other(format!(
                            "Gagal mendaftar lewat {}: {first} Lewat {} juga gagal: {second}",
                            path_label(path),
                            path_label(other)
                        ))
                    })?,
                    None => return Err(first),
                }
            }
        };

    // Nama kosong = pakai nama toko yang dikirim PC pusat saat pairing.
    let mut info = input.draft(paired.device_token);
    if trimmed_opt(&input.name).is_none() && !paired.store_name.trim().is_empty() {
        info.name = paired.store_name.trim().to_string();
    }
    crate::servers::add_server(&state.data_dir, info)
}

/// Pindah server aktif (lokal <-> salah satu remote tersimpan) sekaligus jalur
/// yang dipakai. Men-set/meng-clear `AppState.remote` supaya command yang
/// di-proxy langsung mengarah ke tempat yang benar setelahnya.
///
/// `path = None` mempertahankan jalur tersimpan (dipakai tombol "Putuskan
/// Koneksi" yang balik ke Server Lokal).
#[tauri::command]
pub fn select_server(
    state: State<'_, AppState>,
    id: String,
    path: Option<String>,
) -> AppResult<crate::servers::ActiveServer> {
    let active =
        crate::servers::set_active(&state.data_dir, &id, path.as_deref().map(ServerPath::parse))?;
    let mut remote = state
        .remote
        .lock()
        .map_err(|_| AppError::Other("gagal mengunci state".into()))?;
    *remote = active.server.remote_config(active.path);
    Ok(active)
}

/// Kode Setup untuk PC klien (dijalankan di PC pusat): isi QR pairing yang sama
/// dikemas jadi satu baris teks yang bisa disalin — PC tidak punya kamera.
#[derive(Debug, serde::Serialize)]
pub struct SetupCode {
    pub code: String,
    pub payload: crate::lan::PairingPayload,
}

#[tauri::command]
pub fn client_setup_code(state: State<'_, AppState>) -> AppResult<SetupCode> {
    let local_ip = local_ip_address::local_ip().ok().map(|ip| ip.to_string());
    let conn = state.lock()?;
    let payload = crate::lan::pairing_payload(&conn, local_ip, LAN_SERVER_PORT)?;
    drop(conn);
    let code = crate::lan::setup_code(&payload)?;
    Ok(SetupCode { code, payload })
}

/// Baca Kode Setup yang ditempel di PC klien, untuk mengisi form otomatis.
#[tauri::command]
pub fn decode_setup_code(code: String) -> AppResult<crate::lan::PairingPayload> {
    crate::lan::decode_setup_code(&code)
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
        let handle = crate::lan::start(state.conn.clone(), LAN_SERVER_PORT)?;
        let mut lan = state.lan.lock().map_err(|_| AppError::Other("gagal mengunci state".into()))?;
        *lan = Some(handle);
    }
    lan_status_snapshot(&state)
}

/// Buat kode pairing baru (mis. dicurigai bocor). Tidak perlu restart server:
/// `lan::authenticate` membaca kode dari settings setiap permintaan, jadi kode
/// lama langsung mati. HP yang sudah terdaftar TIDAK ikut terputus — mereka
/// memakai token perangkat masing-masing; untuk memutus HP tertentu pakai
/// `revoke_mobile_device`.
#[tauri::command]
pub fn regenerate_lan_token(state: State<'_, AppState>) -> AppResult<LanServerStatus> {
    let new_token = crate::lan::generate_token();
    let conn = state.lock()?;
    db::set_setting(&conn, "lan_server_token", &new_token)?;
    drop(conn);
    lan_status_snapshot(&state)
}

// ---------- Akses Online (relay) ----------

/// Pengaturan relay yang dikirim frontend. `agent_key` kosong = jangan diubah,
/// supaya layar Pengaturan tidak perlu menampilkan kunci yang sudah tersimpan.
#[derive(Debug, serde::Deserialize)]
pub struct RelaySettingsInput {
    pub url: String,
    pub store_id: String,
    pub agent_key: String,
}

fn relay_status_snapshot(state: &State<'_, AppState>) -> AppResult<crate::relay::RelayStatus> {
    let mut status = state
        .relay_status
        .lock()
        .map_err(|_| AppError::Other("gagal mengunci state".into()))?
        .clone();
    // URL & store id ditampilkan apa adanya dari settings supaya isian form
    // tetap terisi walau agent sedang mati.
    let conn = state.lock()?;
    status.url = db::get_setting(&conn, "relay_url")?.unwrap_or_default();
    status.store_id = db::get_setting(&conn, "relay_store_id")?.unwrap_or_default();
    Ok(status)
}

#[tauri::command]
pub fn relay_status(state: State<'_, AppState>) -> AppResult<crate::relay::RelayStatus> {
    relay_status_snapshot(&state)
}

/// Simpan pengaturan relay tanpa mengubah nyala/matinya agent.
#[tauri::command]
pub fn save_relay_settings(
    state: State<'_, AppState>,
    input: RelaySettingsInput,
) -> AppResult<crate::relay::RelayStatus> {
    {
        let conn = state.lock()?;
        db::set_setting(&conn, "relay_url", input.url.trim())?;
        db::set_setting(&conn, "relay_store_id", input.store_id.trim())?;
        if !input.agent_key.trim().is_empty() {
            db::set_setting(&conn, "relay_agent_key", input.agent_key.trim())?;
        }
    }
    // Sedang aktif? sambung ulang supaya pengaturan baru langsung dipakai.
    let enabled = {
        let conn = state.lock()?;
        db::get_setting(&conn, "relay_enabled")?.as_deref() == Some("1")
    };
    if enabled {
        set_relay_enabled(state, true)
    } else {
        relay_status_snapshot(&state)
    }
}

/// Nyalakan/matikan agent relay pada PC ini.
#[tauri::command]
pub fn set_relay_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> AppResult<crate::relay::RelayStatus> {
    {
        let mut guard = state
            .relay
            .lock()
            .map_err(|_| AppError::Other("gagal mengunci state".into()))?;
        if let Some(handle) = guard.take() {
            handle.stop();
        }
    }
    {
        let conn = state.lock()?;
        db::set_setting(&conn, "relay_enabled", if enabled { "1" } else { "0" })?;
    }

    if !enabled {
        if let Ok(mut s) = state.relay_status.lock() {
            s.enabled = false;
            s.connected = false;
            s.connected_since = None;
            s.last_error = None;
        }
        return relay_status_snapshot(&state);
    }

    let config = {
        let conn = state.lock()?;
        crate::relay::RelayConfig::load(&conn)?
    }
    .ok_or_else(|| {
        AppError::Config("URL relay, Store ID, dan Agent Key wajib diisi dulu.".into())
    })?;

    let handle = crate::relay::start(state.conn.clone(), config, state.relay_status.clone());
    let mut guard = state
        .relay
        .lock()
        .map_err(|_| AppError::Other("gagal mengunci state".into()))?;
    *guard = Some(handle);
    drop(guard);
    relay_status_snapshot(&state)
}

/// QR pairing siap tampil: SVG + isinya (isinya ikut dikirim supaya layar
/// Pengaturan bisa menampilkan nilainya sebagai cadangan kalau kamera HP rewel).
#[derive(Debug, serde::Serialize)]
pub struct PairingQr {
    pub svg: String,
    pub payload: crate::lan::PairingPayload,
}

#[tauri::command]
pub fn pairing_qr(state: State<'_, AppState>) -> AppResult<PairingQr> {
    let local_ip = local_ip_address::local_ip().ok().map(|ip| ip.to_string());
    let conn = state.lock()?;
    let payload = crate::lan::pairing_payload(&conn, local_ip, LAN_SERVER_PORT)?;
    drop(conn);

    let json = serde_json::to_string(&payload)
        .map_err(|e| AppError::Other(format!("gagal menyusun isi QR: {e}")))?;
    let code = qrcode::QrCode::new(json.as_bytes())
        .map_err(|e| AppError::Other(format!("gagal membuat QR: {e}")))?;
    let svg = code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(220, 220)
        .quiet_zone(true)
        .build();
    Ok(PairingQr { svg, payload })
}

/// Daftar HP yang sudah didaftarkan ke Server Pusat ini (layar Pengaturan).
#[tauri::command]
pub fn list_mobile_devices(state: State<'_, AppState>) -> AppResult<Vec<crate::models::MobileDevice>> {
    let conn = state.lock()?;
    db::list_mobile_devices(&conn)
}

/// Cabut akses satu HP. Berlaku seketika untuk jalur LAN maupun relay.
#[tauri::command]
pub fn revoke_mobile_device(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let conn = state.lock()?;
    db::revoke_mobile_device(&conn, &id)
}
