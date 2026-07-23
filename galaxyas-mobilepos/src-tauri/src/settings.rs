//! Pengaturan per-device sebagai `settings.json` di app data dir — pengganti
//! tabel `settings` SQLite di desktop. Key yang dipakai frontend SAMA persis
//! (store_name, receipt_paper, receipt_printer, theme, dst) supaya modul TS
//! yang dicopy dari desktop (mis. parseReceiptConfig) jalan tanpa perubahan.
//! Catatan: client LAN desktop pun menyimpan pengaturannya per-device, jadi
//! perilaku ini konsisten, bukan regresi.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};

fn settings_path(data_dir: &Path) -> PathBuf {
    data_dir.join("settings.json")
}

pub fn all_settings(data_dir: &Path) -> AppResult<HashMap<String, String>> {
    let path = settings_path(data_dir);
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let bytes = std::fs::read(&path)?;
    serde_json::from_slice(&bytes)
        .map_err(|e| AppError::Other(format!("File pengaturan rusak: {e}")))
}

pub fn set_setting(data_dir: &Path, key: &str, value: &str) -> AppResult<()> {
    let mut map = all_settings(data_dir)?;
    map.insert(key.to_string(), value.to_string());
    let bytes = serde_json::to_vec_pretty(&map)
        .map_err(|e| AppError::Other(format!("Gagal menyimpan pengaturan: {e}")))?;
    std::fs::write(settings_path(data_dir), bytes)?;
    Ok(())
}
