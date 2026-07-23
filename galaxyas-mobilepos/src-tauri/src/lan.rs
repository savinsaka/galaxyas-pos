//! Client transport ke "Server Pusat" (LAN central-server di PC Windows).
//!
//! Salinan sisi-client dari desktop/src-tauri/src/lan.rs (tanpa bagian host
//! tiny_http — mobile tidak pernah jadi Server Pusat). Semua command data
//! frontend di-proxy lewat `call(...)` ke `POST http://<host>:<port>/rpc/<nama>`
//! dengan header token pairing, sehingga HP ini memakai database SQLite milik
//! PC pusat secara langsung.

use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{AppError, AppResult};

const TOKEN_HEADER: &str = "X-Galaxyas-Token";
const CONNECT_ERR_MSG: &str =
    "Tidak dapat terhubung ke Server Pusat — periksa koneksi wifi atau apakah PC pusat masih menyala.";

#[derive(Clone, Debug)]
pub struct RemoteConfig {
    pub base_url: String,
    pub token: String,
}

fn client() -> AppResult<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?)
}

/// Panggil satu "command" di Server Pusat lewat HTTP, meniru pemanggilan
/// Tauri command lokal (nama command + args JSON dengan field yang persis
/// sama namanya seperti parameter command aslinya).
pub async fn call<T: Serialize, R: DeserializeOwned>(
    remote: &RemoteConfig,
    name: &str,
    args: T,
) -> AppResult<R> {
    let url = format!("{}/rpc/{}", remote.base_url.trim_end_matches('/'), name);
    let resp = client()?
        .post(&url)
        .header(TOKEN_HEADER, &remote.token)
        .json(&args)
        .send()
        .await
        .map_err(|_| AppError::Other(CONNECT_ERR_MSG.into()))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        let msg = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| "Server Pusat menolak permintaan".to_string());
        return Err(AppError::Other(msg));
    }

    let body = resp.text().await.map_err(|_| AppError::Other(CONNECT_ERR_MSG.into()))?;
    // Body kosong dianggap `null` (untuk command yang me-return `()`).
    let value: serde_json::Value = if body.trim().is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_str(&body)
            .map_err(|e| AppError::Other(format!("respons Server Pusat tidak valid: {e}")))?
    };
    serde_json::from_value(value)
        .map_err(|e| AppError::Other(format!("respons Server Pusat tidak valid: {e}")))
}

/// Cek keterjangkauan host (dipakai layar pairing "+ Tambah Server" sebelum
/// disimpan). Tidak butuh data toko apa pun, cuma liveness + validasi token.
pub async fn health_check(host: &str, port: u16, token: &str) -> AppResult<String> {
    let url = format!("http://{host}:{port}/health");
    let resp = client()?
        .get(&url)
        .header(TOKEN_HEADER, token)
        .send()
        .await
        .map_err(|_| AppError::Other(CONNECT_ERR_MSG.into()))?;
    if !resp.status().is_success() {
        return Err(AppError::Other(CONNECT_ERR_MSG.into()));
    }
    Ok("ok".to_string())
}
