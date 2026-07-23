mod commands;
mod error;
mod lan;
mod servers;
mod settings;

use std::sync::Mutex;

use tauri::Manager;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("gagal menentukan app data dir");
            std::fs::create_dir_all(&data_dir).expect("gagal membuat app data dir");

            // Pulihkan Server Pusat aktif dari servers.json bila sudah pernah
            // pairing; belum ada = frontend menampilkan onboarding pairing.
            let remote = servers::current_server(&data_dir)
                .ok()
                .flatten()
                .map(|info| lan::RemoteConfig {
                    base_url: format!(
                        "http://{}:{}",
                        info.host.clone().unwrap_or_default(),
                        info.port.unwrap_or(8899)
                    ),
                    token: info.token.unwrap_or_default(),
                });

            app.manage(AppState { data_dir, remote: Mutex::new(remote) });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::rpc,
            commands::get_settings,
            commands::update_setting,
            commands::list_servers,
            commands::current_server,
            commands::ping_server,
            commands::add_server,
            commands::select_server,
            commands::remove_server,
            commands::write_temp_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
