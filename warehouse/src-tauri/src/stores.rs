//! Registry multi-toko: sampai 3 database SQLite terpisah dalam satu app data
//! dir, dipilih lewat layar "Pilih Toko" sebelum login. Registry disimpan di
//! `stores.json`; toko pertama otomatis mengarah ke `galaxyas.sqlite` yang
//! sudah ada agar instalasi lama tidak perlu migrasi apa pun.

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::StoreInfo;

pub const MAX_STORES: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Registry {
    active_id: String,
    stores: Vec<StoreInfo>,
}

fn registry_path(data_dir: &Path) -> PathBuf {
    data_dir.join("stores.json")
}

fn default_registry() -> Registry {
    let id = Uuid::new_v4().to_string();
    Registry {
        active_id: id.clone(),
        stores: vec![StoreInfo {
            id,
            name: "Toko 1".into(),
            file: "galaxyas.sqlite".into(),
            created_at: Utc::now().to_rfc3339(),
        }],
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
    let reg: Registry = serde_json::from_slice(&bytes)
        .map_err(|e| AppError::Other(format!("Registry toko rusak: {e}")))?;
    Ok(reg)
}

fn save_registry(data_dir: &Path, reg: &Registry) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(reg)
        .map_err(|e| AppError::Other(format!("Gagal menyimpan registry toko: {e}")))?;
    std::fs::write(registry_path(data_dir), bytes)?;
    Ok(())
}

pub fn list_stores(data_dir: &Path) -> AppResult<Vec<StoreInfo>> {
    Ok(load_registry(data_dir)?.stores)
}

pub fn current_store(data_dir: &Path) -> AppResult<StoreInfo> {
    let reg = load_registry(data_dir)?;
    reg.stores
        .iter()
        .find(|s| s.id == reg.active_id)
        .or_else(|| reg.stores.first())
        .cloned()
        .ok_or_else(|| AppError::Other("Tidak ada toko terdaftar.".into()))
}

pub fn create_store(data_dir: &Path, name: String) -> AppResult<StoreInfo> {
    let mut reg = load_registry(data_dir)?;
    if reg.stores.len() >= MAX_STORES {
        return Err(AppError::Other(format!("Maksimum {MAX_STORES} toko.")));
    }
    let id = Uuid::new_v4().to_string();
    let info = StoreInfo {
        file: format!("store-{id}.sqlite"),
        id,
        name,
        created_at: Utc::now().to_rfc3339(),
    };
    reg.stores.push(info.clone());
    save_registry(data_dir, &reg)?;
    Ok(info)
}

/// Tandai `id` sebagai toko aktif di registry (dipanggil sebelum swap koneksi).
pub fn set_active(data_dir: &Path, id: &str) -> AppResult<StoreInfo> {
    let mut reg = load_registry(data_dir)?;
    let info = reg
        .stores
        .iter()
        .find(|s| s.id == id)
        .cloned()
        .ok_or_else(|| AppError::Other("Toko tidak ditemukan.".into()))?;
    reg.active_id = id.to_string();
    save_registry(data_dir, &reg)?;
    Ok(info)
}

pub fn db_path(data_dir: &Path, store: &StoreInfo) -> PathBuf {
    data_dir.join(&store.file)
}
