//! Sisi PENGIRIM (gudang) ke server GALAXYAS Mobile — kebalikan dari `pull.rs`.
//!
//! GPOS Warehouse login sebagai user (role pengirim/supervisor/bos) lalu bikin
//! OrderRow (barcode + qty_dikirim) untuk toko tujuan. Toko lalu menariknya
//! lewat bridge Pull (`X-Store-Api-Key`) yang sudah ada di POS (`pull.rs`).
//!
//! Auth di sini pakai JWT Bearer (email + password) — BEDA dari `X-Store-Api-Key`
//! yang dipakai sisi pull. `native-tls` diwarisi dari Cargo.toml (server jjapps
//! menolak TLS renegotiation ala rustls — lihat catatan di `pull.rs`/Cargo.toml).

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    // refresh_token diabaikan: Warehouse login ulang tiap operasi (sederhana,
    // operasinya jarang & manual).
}

/// Toko tujuan (subset dari StoreOut server; field lain diabaikan serde).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestStore {
    pub id: String,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
struct LoginBody<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Debug, Serialize)]
struct OrderRowBody<'a> {
    store_id: &'a str,
    order_date: &'a str,
    barcode: &'a str,
    qty_dikirim: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
}

fn client() -> AppResult<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

/// Login sebagai Pengirim, kembalikan access token (dipakai sebagai Bearer).
pub async fn login(server_url: &str, email: &str, password: &str) -> AppResult<String> {
    let url = format!("{}/api/v1/auth/login", server_url.trim_end_matches('/'));
    let resp = client()?
        .post(&url)
        .json(&LoginBody { email, password })
        .send()
        .await?;
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(AppError::Config(
            "Email atau password pengirim salah (cek menu Pengaturan → Pengiriman).".into(),
        ));
    }
    if resp.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(AppError::Config(
            "Akun pengirim belum aktif / role-nya tidak boleh mengirim. Hubungi Bos.".into(),
        ));
    }
    let resp = resp.error_for_status()?;
    Ok(resp.json::<TokenResponse>().await?.access_token)
}

/// Daftar toko tujuan (butuh Bearer).
pub async fn list_stores(server_url: &str, token: &str) -> AppResult<Vec<DestStore>> {
    let url = format!("{}/api/v1/stores", server_url.trim_end_matches('/'));
    let resp = client()?
        .get(&url)
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.json::<Vec<DestStore>>().await?)
}

/// Buat satu OrderRow (barcode + qty_dikirim) untuk `store_id`. Karena hanya
/// qty_dikirim yang diisi (bukan qty_minta), server menandainya origin
/// "pengirim" dan pull_status "pending" — siap ditarik toko.
pub async fn create_order_row(
    server_url: &str,
    token: &str,
    store_id: &str,
    order_date: &str,
    barcode: &str,
    qty: f64,
    name: Option<&str>,
) -> AppResult<()> {
    let url = format!("{}/api/v1/order-rows", server_url.trim_end_matches('/'));
    client()?
        .post(&url)
        .bearer_auth(token)
        .json(&OrderRowBody {
            store_id,
            order_date,
            barcode,
            qty_dikirim: qty,
            name,
        })
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
