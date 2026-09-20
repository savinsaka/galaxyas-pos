//! Sisi PENGIRIM (gudang) ke server bridge Gudang (`warehouse-bridge`).
//!
//! Auth pakai **API key mesin** lewat header `X-Warehouse-Key` (tanpa akun/
//! email/password). GPOS Warehouse ambil daftar toko lalu membuat baris kiriman
//! (batch, sekali kirim = atomik di server). Toko menariknya lewat "Pull dari
//! Gudang" (`X-Store-Api-Key`). `native-tls` diwarisi dari Cargo.toml.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Toko tujuan (subset; field lain diabaikan serde).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestStore {
    pub id: String,
    pub code: String,
    pub name: String,
}

/// Satu item kiriman untuk body POST /order-rows.
#[derive(Debug, Serialize)]
pub struct ShipItem<'a> {
    pub barcode: &'a str,
    pub qty: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct ShipRequest<'a> {
    store_id: &'a str,
    items: Vec<ShipItem<'a>>,
}

fn client() -> AppResult<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

fn key_err() -> AppError {
    AppError::Config("Warehouse Key salah / belum benar (cek Pengaturan → Server Pengiriman).".into())
}

/// Daftar toko tujuan.
pub async fn list_stores(server_url: &str, warehouse_key: &str) -> AppResult<Vec<DestStore>> {
    let url = format!("{}/api/v1/stores", server_url.trim_end_matches('/'));
    let resp = client()?
        .get(&url)
        .header("X-Warehouse-Key", warehouse_key)
        .send()
        .await?;
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(key_err());
    }
    let resp = resp.error_for_status()?;
    Ok(resp.json::<Vec<DestStore>>().await?)
}

/// Buat semua baris kiriman untuk satu toko dalam SATU request (atomik di
/// server — tidak ada risiko dobel-separuh saat retry).
pub async fn create_order_rows(
    server_url: &str,
    warehouse_key: &str,
    store_id: &str,
    items: Vec<ShipItem<'_>>,
) -> AppResult<()> {
    let url = format!("{}/api/v1/order-rows", server_url.trim_end_matches('/'));
    let resp = client()?
        .post(&url)
        .header("X-Warehouse-Key", warehouse_key)
        .json(&ShipRequest { store_id, items })
        .send()
        .await?;
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(key_err());
    }
    resp.error_for_status()?;
    Ok(())
}
