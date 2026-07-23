//! Registry "Server Pusat" versi mobile: daftar server remote (host:port +
//! token pairing) yang pernah dipasangkan, disimpan di `servers.json` di app
//! data dir. Beda dengan desktop (desktop/src-tauri/src/servers.rs): TIDAK ada
//! entry implisit "Server Lokal" — HP selalu client, tanpa database sendiri.
//! Belum ada server aktif = frontend menampilkan onboarding pairing.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub id: String,
    /// Selalu "remote" di mobile; field dipertahankan agar bentuk JSON sama
    /// dengan desktop (frontend ServerPicker dipakai lintas app).
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Registry {
    active_id: String,
    servers: Vec<ServerInfo>,
}

fn registry_path(data_dir: &Path) -> PathBuf {
    data_dir.join("servers.json")
}

fn default_registry() -> Registry {
    Registry { active_id: String::new(), servers: Vec::new() }
}

fn load_registry(data_dir: &Path) -> AppResult<Registry> {
    let path = registry_path(data_dir);
    if !path.exists() {
        return Ok(default_registry());
    }
    let bytes = std::fs::read(&path)?;
    serde_json::from_slice(&bytes).map_err(|e| AppError::Other(format!("Registry server rusak: {e}")))
}

fn save_registry(data_dir: &Path, reg: &Registry) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(reg)
        .map_err(|e| AppError::Other(format!("Gagal menyimpan registry server: {e}")))?;
    std::fs::write(registry_path(data_dir), bytes)?;
    Ok(())
}

pub fn list_servers(data_dir: &Path) -> AppResult<Vec<ServerInfo>> {
    Ok(load_registry(data_dir)?.servers)
}

/// Server aktif, atau None bila belum pernah pairing (onboarding).
pub fn current_server(data_dir: &Path) -> AppResult<Option<ServerInfo>> {
    let reg = load_registry(data_dir)?;
    Ok(reg.servers.iter().find(|s| s.id == reg.active_id).cloned())
}

/// Simpan server remote baru (setelah pairing berhasil divalidasi lewat `/health`).
pub fn add_server(
    data_dir: &Path,
    name: String,
    host: String,
    port: u16,
    token: String,
) -> AppResult<ServerInfo> {
    let mut reg = load_registry(data_dir)?;
    let info = ServerInfo {
        id: Uuid::new_v4().to_string(),
        kind: "remote".to_string(),
        name,
        host: Some(host),
        port: Some(port),
        token: Some(token),
    };
    reg.servers.push(info.clone());
    // Server pertama yang ditambahkan langsung jadi aktif.
    if reg.active_id.is_empty() {
        reg.active_id = info.id.clone();
    }
    save_registry(data_dir, &reg)?;
    Ok(info)
}

/// Tandai `id` sebagai server aktif (dipanggil sebelum swap RemoteConfig di AppState).
pub fn set_active(data_dir: &Path, id: &str) -> AppResult<ServerInfo> {
    let mut reg = load_registry(data_dir)?;
    let info = reg
        .servers
        .iter()
        .find(|s| s.id == id)
        .cloned()
        .ok_or_else(|| AppError::Other("Server tidak ditemukan.".into()))?;
    reg.active_id = id.to_string();
    save_registry(data_dir, &reg)?;
    Ok(info)
}

pub fn remove_server(data_dir: &Path, id: &str) -> AppResult<()> {
    let mut reg = load_registry(data_dir)?;
    reg.servers.retain(|s| s.id != id);
    if reg.active_id == id {
        reg.active_id = reg.servers.first().map(|s| s.id.clone()).unwrap_or_default();
    }
    save_registry(data_dir, &reg)?;
    Ok(())
}
