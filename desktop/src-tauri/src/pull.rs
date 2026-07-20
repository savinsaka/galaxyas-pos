//! Bridge ke server GALAXYAS Mobile (galaxyas-mobile/backend) — TERPISAH dari
//! `sync.rs` (yang urusannya sync master data produk ke server POS/`jjapps.net`).
//!
//! Auth di sini pakai header `X-Store-Api-Key` (kunci per-toko), BUKAN token
//! sync yang dipakai `sync.rs`. Dipanggil dari menu Item Masuk lewat tombol
//! "Pull" — kasir tarik baris menu Barang yang siap di-pull (barcode+qty
//! dikirim udah keisi dari app mobile), koreksi qty kalau perlu, lalu
//! konfirmasi (baru itu ditulis ke stock_movements lokal lewat
//! `db::create_stock_movement_batch`).

use serde::{Deserialize, Serialize};

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullItem {
    pub id: String,
    pub barcode: String,
    pub name: Option<String>,
    pub qty_dikirim: f64,
    /// Diisi LOKAL di commands.rs:bridge_list_pending (server mobile gak tau
    /// katalog POS) — true kalau barcode ini match ke produk yang sudah ada
    /// di database toko ini. Kalau false, kasir sebaiknya cek dulu apa
    /// barcode-nya salah/typo sebelum konfirmasi (nama cuma referensi buat
    /// itu, bukan sumber kebenaran).
    #[serde(default)]
    pub known_locally: bool,
}

#[derive(Debug, Serialize)]
struct ConfirmItem<'a> {
    id: &'a str,
    qty_confirmed: f64,
}

#[derive(Debug, Serialize)]
struct ConfirmRequest<'a> {
    items: Vec<ConfirmItem<'a>>,
}

fn client() -> AppResult<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

/// Daftar baris menu Barang yang siap di-pull buat toko ini (toko ditentukan
/// server dari API key, bukan dikirim dari sini).
pub async fn list_pending(server_url: &str, api_key: &str) -> AppResult<Vec<PullItem>> {
    let url = format!("{}/api/v1/bridge/order-rows", server_url.trim_end_matches('/'));
    let resp = client()?
        .get(&url)
        .header("X-Store-Api-Key", api_key)
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.json::<Vec<PullItem>>().await?)
}

/// Konfirmasi sejumlah baris sekaligus (dengan qty final, boleh beda dari
/// yang diklaim Pengirim kalau kasir ketemu lebih/kurang saat cek fisik).
pub async fn confirm(server_url: &str, api_key: &str, items: &[(String, f64)]) -> AppResult<Vec<PullItem>> {
    let url = format!("{}/api/v1/bridge/order-rows/confirm", server_url.trim_end_matches('/'));
    let body = ConfirmRequest {
        items: items.iter().map(|(id, qty)| ConfirmItem { id, qty_confirmed: *qty }).collect(),
    };
    let resp = client()?
        .post(&url)
        .header("X-Store-Api-Key", api_key)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.json::<Vec<PullItem>>().await?)
}

/// Tolak 1 baris (mis. barang gak jadi dikirim / salah kirim ke toko ini).
pub async fn reject(server_url: &str, api_key: &str, row_id: &str) -> AppResult<PullItem> {
    let url = format!(
        "{}/api/v1/bridge/order-rows/{}/reject",
        server_url.trim_end_matches('/'),
        row_id
    );
    let resp = client()?
        .post(&url)
        .header("X-Store-Api-Key", api_key)
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.json::<PullItem>().await?)
}
