mod commands;
mod db;
mod error;
mod lan;
mod models;
mod pull;
mod servers;
mod stores;
mod sync;
#[cfg(target_os = "windows")]
mod winprint;

use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tauri::Manager;

use commands::{AppState, PrintPayloadState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // Tandai shutdown BERSIH (lawan dari crash/force-close) hanya untuk window
        // utama — window cetak (label "print-*") dibuka/ditutup berkali-kali dan
        // tidak boleh ikut meng-clear penanda ini. Lihat db::recover_stale_shift.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if window.label() == "main" {
                    let state = window.app_handle().state::<AppState>();
                    let lock_result = state.conn.lock();
                    if let Ok(conn) = lock_result {
                        let _ = db::set_setting(&conn, "app_running", "0");
                    }
                }
            }
        })
        .setup(|app| {
            // Database lokal disimpan di app data dir (per instalasi). Bisa berisi
            // sampai 3 toko terpisah (lihat `stores.rs`); toko aktif ditentukan
            // oleh registry `stores.json`, dengan migrasi otomatis dari
            // `galaxyas.sqlite` lama sebagai "Toko 1" bila registry belum ada.
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("gagal menentukan app data dir");
            std::fs::create_dir_all(&data_dir).expect("gagal membuat app data dir");

            let active = stores::current_store(&data_dir).expect("gagal membaca registry toko");
            let db_path = stores::db_path(&data_dir, &active);

            let conn = Connection::open(&db_path).expect("gagal membuka database SQLite");
            db::init_schema(&conn).expect("gagal inisialisasi skema");
            db::seed_defaults(&conn).expect("gagal seeding data awal");
            // Pulihkan shift yang tertinggal kalau sesi sebelumnya berhenti tidak
            // wajar (crash/force close/mati listrik) — jangan gagalkan startup
            // kalau ini error, cukup catat.
            if let Err(e) = db::recover_stale_shift(&conn) {
                eprintln!("gagal memulihkan shift yang tertinggal: {e}");
            }

            // Baca pengaturan Server Pusat (host) dari koneksi lokal SEBELUM
            // dipindah ke Arc<Mutex<>>, supaya auto-start di bawah bisa pakai
            // token yang sudah pasti tersimpan. Robust terhadap setting yang
            // belum ada (instalasi lama / fitur belum pernah dipakai).
            let lan_enabled =
                db::get_setting(&conn, "lan_server_enabled").ok().flatten().as_deref() == Some("1");
            let mut lan_token = db::get_setting(&conn, "lan_server_token")
                .ok()
                .flatten()
                .unwrap_or_default();
            if lan_enabled && lan_token.is_empty() {
                lan_token = lan::generate_token();
                let _ = db::set_setting(&conn, "lan_server_token", &lan_token);
            }

            let conn = Arc::new(Mutex::new(conn));

            app.manage(AppState {
                conn: conn.clone(),
                data_dir: data_dir.clone(),
                remote: Mutex::new(None),
                lan: Mutex::new(None),
            });
            app.manage(PrintPayloadState::default());

            // Bila server aktif tersimpan di registry adalah "remote", set
            // AppState.remote supaya command yang di-proxy langsung memanggil
            // host tsb. Registry yang belum ada/rusak dianggap mode lokal.
            if let Ok(active_server) = servers::current_server(&data_dir) {
                if active_server.is_remote() {
                    let remote_cfg = lan::RemoteConfig {
                        base_url: format!(
                            "http://{}:{}",
                            active_server.host.clone().unwrap_or_default(),
                            active_server.port.unwrap_or(8899)
                        ),
                        token: active_server.token.clone().unwrap_or_default(),
                    };
                    let state = app.state::<AppState>();
                    let lock_result = state.remote.lock();
                    if let Ok(mut remote) = lock_result {
                        *remote = Some(remote_cfg);
                    }
                }
            }

            // Auto-start Server Pusat (host) bila sebelumnya diaktifkan dari
            // PC ini. Kegagalan (mis. port dipakai) tidak boleh menggagalkan
            // startup aplikasi — cukup dicatat ke stderr.
            if lan_enabled {
                match lan::start(conn.clone(), 8899, lan_token) {
                    Ok(handle) => {
                        let state = app.state::<AppState>();
                        let lock_result = state.lan.lock();
                        if let Ok(mut lan_guard) = lock_result {
                            *lan_guard = Some(handle);
                        }
                    }
                    Err(e) => {
                        eprintln!("gagal memulai Server Pusat otomatis: {e}");
                    }
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_stores,
            commands::current_store,
            commands::create_store,
            commands::select_store,
            commands::get_settings,
            commands::update_setting,
            commands::list_products,
            commands::list_products_page,
            commands::save_product,
            commands::toggle_product_active,
            commands::delete_product,
            commands::dedupe_products,
            commands::reset_data,
            commands::find_by_barcode,
            commands::adjust_stock,
            commands::set_stock,
            commands::checkout,
            commands::list_transactions,
            commands::get_transaction,
            commands::delete_transaction,
            commands::update_transaction,
            commands::login,
            commands::list_users,
            commands::save_user,
            commands::delete_user,
            commands::create_stock_movement,
            commands::list_stock_movements,
            commands::delete_stock_movement,
            commands::create_stock_movement_batch,
            commands::list_stock_movement_batches,
            commands::get_stock_movement_batch,
            commands::update_stock_movement_batch,
            commands::delete_stock_movement_batch,
            commands::list_discounts,
            commands::save_discount,
            commands::delete_discount,
            commands::list_brands,
            commands::save_brand,
            commands::delete_brand,
            commands::list_customers,
            commands::save_customer,
            commands::delete_customer,
            commands::list_expenses,
            commands::save_expense,
            commands::delete_expense,
            commands::get_active_shift,
            commands::open_shift,
            commands::close_shift,
            commands::list_shifts,
            commands::product_sales_report,
            commands::brand_sales_report,
            commands::sales_item_detail_report,
            commands::daily_sales_report,
            commands::write_temp_file,
            commands::list_printers,
            commands::print_text_to,
            commands::print_escpos_to,
            commands::open_print_window,
            commands::take_print_payload,
            commands::sync_push,
            commands::sync_pull,
            commands::sync_all,
            commands::bridge_list_pending,
            commands::bridge_confirm_pull,
            commands::bridge_reject_pull,
            commands::list_servers,
            commands::current_server,
            commands::ping_server,
            commands::add_server,
            commands::select_server,
            commands::remove_server,
            commands::lan_server_status,
            commands::set_lan_server_enabled,
            commands::regenerate_lan_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
