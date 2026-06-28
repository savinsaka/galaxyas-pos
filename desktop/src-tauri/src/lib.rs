mod commands;
mod db;
mod error;
mod models;
mod sync;

use std::sync::Mutex;

use rusqlite::Connection;
use tauri::Manager;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Database lokal disimpan di app data dir (per instalasi/toko).
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("gagal menentukan app data dir");
            std::fs::create_dir_all(&data_dir).expect("gagal membuat app data dir");
            let db_path = data_dir.join("galaxyas.sqlite");

            let conn = Connection::open(&db_path).expect("gagal membuka database SQLite");
            db::init_schema(&conn).expect("gagal inisialisasi skema");
            db::seed_defaults(&conn).expect("gagal seeding data awal");

            app.manage(AppState { conn: Mutex::new(conn) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::update_setting,
            commands::list_products,
            commands::save_product,
            commands::toggle_product_active,
            commands::delete_product,
            commands::find_by_barcode,
            commands::adjust_stock,
            commands::set_stock,
            commands::checkout,
            commands::list_transactions,
            commands::get_transaction,
            commands::delete_transaction,
            commands::login,
            commands::list_users,
            commands::save_user,
            commands::delete_user,
            commands::create_stock_movement,
            commands::list_stock_movements,
            commands::delete_stock_movement,
            commands::list_discounts,
            commands::save_discount,
            commands::delete_discount,
            commands::list_brands,
            commands::save_brand,
            commands::delete_brand,
            commands::write_temp_file,
            commands::list_printers,
            commands::print_text_to,
            commands::sync_push,
            commands::sync_pull,
            commands::sync_all,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
