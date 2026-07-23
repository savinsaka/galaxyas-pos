use std::path::PathBuf;
use std::sync::Mutex;

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::lan::{self, RemoteConfig};
use crate::servers::{self, ServerInfo};
use crate::settings;

/// State global aplikasi mobile: hanya data dir + konfigurasi Server Pusat
/// aktif. Tidak ada koneksi SQLite — semua data hidup di PC pusat.
pub struct AppState {
    pub data_dir: PathBuf,
    /// Terisi bila sudah pairing: semua command data lewat `rpc` memakai ini.
    pub remote: Mutex<Option<RemoteConfig>>,
}

impl AppState {
    pub fn remote_config(&self) -> Option<RemoteConfig> {
        self.remote.lock().ok().and_then(|g| g.clone())
    }
}

fn remote_config_from(info: &ServerInfo) -> RemoteConfig {
    RemoteConfig {
        base_url: format!(
            "http://{}:{}",
            info.host.clone().unwrap_or_default(),
            info.port.unwrap_or(8899)
        ),
        token: info.token.clone().unwrap_or_default(),
    }
}

// ---------- Proxy generik ke Server Pusat ----------

/// Satu-satunya jalur data: teruskan command apa pun ke host lewat
/// `POST /rpc/<name>`. Menggantikan ~44 proxy command desktop — command host
/// baru di masa depan otomatis terlayani tanpa perubahan Rust di sini.
#[tauri::command]
pub async fn rpc(
    state: State<'_, AppState>,
    name: String,
    args: serde_json::Value,
) -> AppResult<serde_json::Value> {
    let remote = state
        .remote_config()
        .ok_or_else(|| AppError::Other("Belum terhubung ke Server Pusat.".into()))?;
    lan::call(&remote, &name, args).await
}

// ---------- Pengaturan (per-device, settings.json) ----------

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<std::collections::HashMap<String, String>> {
    settings::all_settings(&state.data_dir)
}

#[tauri::command]
pub fn update_setting(state: State<'_, AppState>, key: String, value: String) -> AppResult<()> {
    settings::set_setting(&state.data_dir, &key, &value)
}

// ---------- Registry Server Pusat ----------

#[tauri::command]
pub fn list_servers(state: State<'_, AppState>) -> AppResult<Vec<ServerInfo>> {
    servers::list_servers(&state.data_dir)
}

#[tauri::command]
pub fn current_server(state: State<'_, AppState>) -> AppResult<Option<ServerInfo>> {
    servers::current_server(&state.data_dir)
}

/// Validasi host+token lewat `/health` SEBELUM disimpan (alur "+ Tambah Server").
#[tauri::command]
pub async fn ping_server(host: String, port: u16, token: String) -> AppResult<String> {
    lan::health_check(&host, port, &token).await
}

#[tauri::command]
pub async fn add_server(
    state: State<'_, AppState>,
    name: String,
    host: String,
    port: u16,
    token: String,
) -> AppResult<ServerInfo> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Other("Nama server wajib diisi.".into()));
    }
    lan::health_check(&host, port, &token).await?;
    let info = servers::add_server(&state.data_dir, name.to_string(), host, port, token)?;
    // Server pertama langsung aktif (lihat servers::add_server) — sinkronkan state.
    if servers::current_server(&state.data_dir)?.map(|s| s.id) == Some(info.id.clone()) {
        if let Ok(mut remote) = state.remote.lock() {
            *remote = Some(remote_config_from(&info));
        }
    }
    Ok(info)
}

#[tauri::command]
pub fn select_server(state: State<'_, AppState>, id: String) -> AppResult<ServerInfo> {
    let info = servers::set_active(&state.data_dir, &id)?;
    if let Ok(mut remote) = state.remote.lock() {
        *remote = Some(remote_config_from(&info));
    }
    Ok(info)
}

#[tauri::command]
pub fn remove_server(state: State<'_, AppState>, id: String) -> AppResult<()> {
    servers::remove_server(&state.data_dir, &id)?;
    let now_active = servers::current_server(&state.data_dir)?;
    if let Ok(mut remote) = state.remote.lock() {
        *remote = now_active.as_ref().map(remote_config_from);
    }
    Ok(())
}

// ---------- File sementara (share PDF laporan, Phase 4) ----------

/// Tulis bytes ke file sementara dan kembalikan path-nya (dipakai alur share
/// PDF via plugin opener). Salinan pola desktop `write_temp_file`.
#[tauri::command]
pub fn write_temp_file(file_name: String, contents: Vec<u8>) -> AppResult<String> {
    let dir = std::env::temp_dir().join("galaxyas");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(file_name);
    std::fs::write(&path, contents)?;
    Ok(path.to_string_lossy().to_string())
}
