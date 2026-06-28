mod auth;
mod commands;
mod db;
mod error;
mod state;
mod sync;

use std::sync::{Arc, Mutex};

use rusqlite::params;
use tauri::Manager;

use crate::db::models::{new_id, now_ms, SyncStatusInfo};
use crate::db::pool::{init_pool, DbPool};
use crate::error::AppResult;
use crate::state::AppState;
use crate::sync::SyncHandle;

const DEFAULT_STORE_ID: &str = "store-001";

/// Seed a default store, admin/cashier users and store row on first launch.
fn seed_defaults(pool: &DbPool, store_id: &str) -> AppResult<()> {
    let conn = pool.get()?;
    let store_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM stores", [], |r| r.get(0))?;
    if store_count > 0 {
        return Ok(());
    }

    let ts = now_ms();
    conn.execute(
        "INSERT INTO stores (id, name, address, phone, tax_percent, created_at, updated_at, sync_status)
         VALUES (?1, 'GalaxyAS Toko Pusat', 'Jl. Merdeka No. 1', '021-0000000', 11, ?2, ?2, 'synced')",
        params![store_id, ts],
    )?;

    let admin_hash = auth::hash_password("admin123")?;
    let kasir_hash = auth::hash_password("kasir123")?;
    conn.execute(
        "INSERT INTO users (id, store_id, username, full_name, password_hash, role, is_active, created_at, updated_at, sync_status)
         VALUES (?1, ?2, 'admin', 'Administrator', ?3, 'admin', 1, ?4, ?4, 'synced')",
        params![new_id(), store_id, admin_hash, ts],
    )?;
    conn.execute(
        "INSERT INTO users (id, store_id, username, full_name, password_hash, role, is_active, created_at, updated_at, sync_status)
         VALUES (?1, ?2, 'kasir', 'Kasir Satu', ?3, 'kasir', 1, ?4, ?4, 'synced')",
        params![new_id(), store_id, kasir_hash, ts],
    )?;

    tracing::info!("seeded default store and users");
    Ok(())
}

fn init_logging(log_dir: &std::path::Path) {
    let file_appender = tracing_appender::rolling::daily(log_dir, "galaxyas-pos.log");
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,galaxyas_pos_lib=debug".into()),
        )
        .with_writer(file_appender)
        .with_ansi(false)
        .try_init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&data_dir).ok();
            init_logging(&data_dir);

            let db_path = data_dir.join("galaxyas.sqlite");
            let pool = init_pool(&db_path).expect("failed to initialize database");

            let store_id = std::env::var("GALAXYAS_STORE_ID")
                .unwrap_or_else(|_| DEFAULT_STORE_ID.to_string());
            seed_defaults(&pool, &store_id).expect("failed to seed defaults");

            let server_url = std::env::var("GALAXYAS_SERVER_URL")
                .unwrap_or_else(|_| "http://localhost:8000".to_string());

            let http = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build http client");

            let sync_status = Arc::new(Mutex::new(SyncStatusInfo {
                state: "idle".into(),
                pending_count: 0,
                conflict_count: 0,
                last_sync_at: None,
                last_error: None,
            }));

            let app_state = AppState {
                pool: pool.clone(),
                store_id: store_id.clone(),
                session: Mutex::new(None),
                sync_status: sync_status.clone(),
                http: http.clone(),
                server_url: server_url.clone(),
            };

            // Spawn the periodic background sync worker.
            sync::spawn_worker(SyncHandle {
                pool,
                http,
                server_url,
                store_id,
                status: sync_status,
            });

            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::login,
            commands::auth::logout,
            commands::auth::current_session,
            commands::auth::list_users,
            commands::settings::get_store,
            commands::settings::update_store,
            commands::settings::get_printer_settings,
            commands::settings::update_printer_settings,
            commands::items::list_items,
            commands::items::get_item,
            commands::items::find_item_by_barcode,
            commands::items::create_item,
            commands::items::update_item,
            commands::items::delete_item,
            commands::items::duplicate_item,
            commands::items::bulk_upsert_items,
            commands::items::mass_update_items,
            commands::inventory::record_stock_movement,
            commands::inventory::list_stock_transactions,
            commands::inventory::apply_stock_opname,
            commands::sales::create_sale,
            commands::sales::get_sale,
            commands::sales::list_held_sales,
            commands::sales::void_sale,
            commands::shifts::open_shift,
            commands::shifts::close_shift,
            commands::shifts::current_shift,
            commands::audit::list_audit_logs,
            commands::reports::daily_sales_report,
            commands::reports::top_selling_products,
            commands::reports::stock_report,
            commands::sync::sync_status,
            commands::sync::trigger_sync,
            commands::sync::list_conflicts,
            commands::sync::resolve_conflict,
            commands::print::print_receipt,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GalaxyAS POS");
}
