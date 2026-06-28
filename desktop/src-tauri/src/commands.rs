use std::collections::HashMap;
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::Connection;
use tauri::State;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{
    DiscountPeriod, DiscountPeriodInput, Product, ProductInput, ProductWithStock, SaleInput,
    StockMovement, StockMovementInput, SyncResult, Transaction, TransactionDetail, User, UserInput,
};
use crate::sync;

/// State global aplikasi: koneksi SQLite tunggal yang dijaga Mutex.
pub struct AppState {
    pub conn: Mutex<Connection>,
}

impl AppState {
    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| AppError::Other("gagal mengunci database".into()))
    }
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
pub fn list_products(
    state: State<'_, AppState>,
    search: Option<String>,
    include_inactive: Option<bool>,
) -> AppResult<Vec<ProductWithStock>> {
    let conn = state.lock()?;
    db::list_products(&conn, search, include_inactive.unwrap_or(false))
}

#[tauri::command]
pub fn save_product(state: State<'_, AppState>, input: ProductInput) -> AppResult<Product> {
    let conn = state.lock()?;
    db::upsert_product(&conn, input)
}

#[tauri::command]
pub fn toggle_product_active(
    state: State<'_, AppState>,
    id: String,
    active: bool,
) -> AppResult<()> {
    let conn = state.lock()?;
    db::set_product_active(&conn, &id, active)
}

#[tauri::command]
pub fn delete_product(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let conn = state.lock()?;
    db::delete_product(&conn, &id)
}

#[tauri::command]
pub fn find_by_barcode(
    state: State<'_, AppState>,
    barcode: String,
) -> AppResult<Option<ProductWithStock>> {
    let conn = state.lock()?;
    db::find_by_barcode(&conn, &barcode)
}

#[tauri::command]
pub fn adjust_stock(state: State<'_, AppState>, product_id: String, delta: f64) -> AppResult<f64> {
    let conn = state.lock()?;
    db::adjust_stock(&conn, &product_id, delta)
}

#[tauri::command]
pub fn set_stock(state: State<'_, AppState>, product_id: String, qty: f64) -> AppResult<f64> {
    let conn = state.lock()?;
    db::set_stock(&conn, &product_id, qty)
}

// ---------- Penjualan / kasir ----------

#[tauri::command]
pub fn checkout(state: State<'_, AppState>, sale: SaleInput) -> AppResult<TransactionDetail> {
    let mut conn = state.lock()?;
    db::create_sale(&mut conn, sale)
}

#[tauri::command]
pub fn list_transactions(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> AppResult<Vec<Transaction>> {
    let conn = state.lock()?;
    db::list_transactions(&conn, limit.unwrap_or(100))
}

#[tauri::command]
pub fn get_transaction(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Option<TransactionDetail>> {
    let conn = state.lock()?;
    db::get_transaction(&conn, &id)
}

#[tauri::command]
pub fn delete_transaction(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let mut conn = state.lock()?;
    db::delete_transaction(&mut conn, &id)
}

// ---------- Pengguna / hak akses ----------

#[tauri::command]
pub fn login(state: State<'_, AppState>, username: String, pin: String) -> AppResult<Option<User>> {
    let conn = state.lock()?;
    db::login(&conn, &username, &pin)
}

#[tauri::command]
pub fn list_users(state: State<'_, AppState>) -> AppResult<Vec<User>> {
    let conn = state.lock()?;
    db::list_users(&conn)
}

#[tauri::command]
pub fn save_user(state: State<'_, AppState>, input: UserInput) -> AppResult<User> {
    let conn = state.lock()?;
    db::save_user(&conn, input)
}

#[tauri::command]
pub fn delete_user(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let conn = state.lock()?;
    db::delete_user(&conn, &id)
}

// ---------- Pergerakan stok ----------

#[tauri::command]
pub fn create_stock_movement(
    state: State<'_, AppState>,
    input: StockMovementInput,
) -> AppResult<StockMovement> {
    let conn = state.lock()?;
    db::create_stock_movement(&conn, input)
}

#[tauri::command]
pub fn list_stock_movements(
    state: State<'_, AppState>,
    kind: Option<String>,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
) -> AppResult<Vec<StockMovement>> {
    let conn = state.lock()?;
    db::list_stock_movements(&conn, kind, from, to, limit.unwrap_or(500))
}

#[tauri::command]
pub fn delete_stock_movement(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    let mut conn = state.lock()?;
    db::delete_stock_movement(&mut conn, id)
}

// ---------- Diskon periodik ----------

#[tauri::command]
pub fn list_discounts(state: State<'_, AppState>) -> AppResult<Vec<DiscountPeriod>> {
    let conn = state.lock()?;
    db::list_discounts(&conn)
}

#[tauri::command]
pub fn save_discount(
    state: State<'_, AppState>,
    input: DiscountPeriodInput,
) -> AppResult<DiscountPeriod> {
    let conn = state.lock()?;
    db::save_discount(&conn, input)
}

#[tauri::command]
pub fn delete_discount(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let conn = state.lock()?;
    db::delete_discount(&conn, &id)
}

// ---------- Merek (brand) ----------

#[tauri::command]
pub fn list_brands(state: State<'_, AppState>) -> AppResult<Vec<crate::models::Brand>> {
    let conn = state.lock()?;
    db::list_brands(&conn)
}

#[tauri::command]
pub fn save_brand(
    state: State<'_, AppState>,
    input: crate::models::BrandInput,
) -> AppResult<crate::models::Brand> {
    let conn = state.lock()?;
    db::save_brand(&conn, input)
}

#[tauri::command]
pub fn delete_brand(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let conn = state.lock()?;
    db::delete_brand(&conn, &id)
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

        let ps = match printer {
            Some(p) if !p.trim().is_empty() => format!(
                "Get-Content -Raw -LiteralPath '{}' | Out-Printer -Name '{}'",
                path.display(),
                p.replace('\'', "''")
            ),
            _ => format!("Get-Content -Raw -LiteralPath '{}' | Out-Printer", path.display()),
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
