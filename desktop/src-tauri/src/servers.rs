//! Registry "Server Pusat": daftar server yang bisa dipilih di layar awal
//! sebelum toko. Selalu ada entry implisit "Server Lokal" (kind="local",
//! perilaku persis seperti sekarang — SQLite in-process), ditambah entry
//! "remote" (host:port + token pairing) untuk konek ke PC lain di LAN yang
//! mengaktifkan Server Pusat. Disimpan di `servers.json` di app data dir,
//! pola sama seperti registry toko di `stores.rs`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

pub const LOCAL_SERVER_ID: &str = "local";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub id: String,
    /// "local" | "remote"
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub token: Option<String>,
}

impl ServerInfo {
    pub fn local() -> Self {
        ServerInfo {
            id: LOCAL_SERVER_ID.to_string(),
            kind: "local".to_string(),
            name: "Server Lokal".to_string(),
            host: None,
            port: None,
            token: None,
        }
    }

    pub fn is_remote(&self) -> bool {
        self.kind == "remote"
    }
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
    Registry {
        active_id: LOCAL_SERVER_ID.to_string(),
        servers: vec![ServerInfo::local()],
    }
}

fn load_registry(data_dir: &Path) -> AppResult<Registry> {
    let path = registry_path(data_dir);
    if !path.exists() {
        let reg = default_registry();
        save_registry(data_dir, &reg)?;
        return Ok(reg);
    }
    let bytes = std::fs::read(&path)?;
    let mut reg: Registry = serde_json::from_slice(&bytes)
        .map_err(|e| AppError::Other(format!("Registry server rusak: {e}")))?;
    // Registry lama (sebelum fitur ini ada) mungkin belum punya entry lokal.
    if !reg.servers.iter().any(|s| s.id == LOCAL_SERVER_ID) {
        reg.servers.insert(0, ServerInfo::local());
        save_registry(data_dir, &reg)?;
    }
    Ok(reg)
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

pub fn current_server(data_dir: &Path) -> AppResult<ServerInfo> {
    let reg = load_registry(data_dir)?;
    Ok(reg
        .servers
        .iter()
        .find(|s| s.id == reg.active_id)
        .cloned()
        .unwrap_or_else(ServerInfo::local))
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
    save_registry(data_dir, &reg)?;
    Ok(info)
}

/// Tandai `id` sebagai server aktif (dipanggil sebelum swap mode di AppState).
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
    if id == LOCAL_SERVER_ID {
        return Err(AppError::Other("Server Lokal tidak bisa dihapus.".into()));
    }
    let mut reg = load_registry(data_dir)?;
    reg.servers.retain(|s| s.id != id);
    if reg.active_id == id {
        reg.active_id = LOCAL_SERVER_ID.to_string();
    }
    save_registry(data_dir, &reg)?;
    Ok(())
}
