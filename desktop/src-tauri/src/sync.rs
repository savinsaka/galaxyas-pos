use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::models::Product;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct PullResponse {
    pub server_time: String,
    pub count: i64,
    pub products: Vec<Product>,
}

#[derive(Debug, Serialize)]
struct PushRequest<'a> {
    store_id: &'a str,
    products: &'a [Product],
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct PushResponse {
    pub server_time: String,
    pub received: i64,
    pub applied: i64,
    pub skipped: i64,
    #[serde(default)]
    pub results: Vec<PushResultItem>,
}

#[derive(Debug, Deserialize)]
pub struct PushResultItem {
    pub id: String,
    /// "created" | "applied" | "skipped_stale" | "skipped_brand"
    pub status: String,
}

#[derive(Debug, Serialize)]
struct HardPushRequest<'a> {
    store_id: &'a str,
    products: &'a [Product],
    exclude_brands: &'a [String],
}

fn client() -> AppResult<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

/// Tarik master data yang berubah setelah `since` (Delta Sync).
pub async fn pull(
    server_url: &str,
    store_id: &str,
    since: Option<&str>,
) -> AppResult<PullResponse> {
    let url = format!("{}/api/v1/sync/products/pull", server_url.trim_end_matches('/'));
    let mut req = client()?.get(&url).query(&[("store_id", store_id)]);
    if let Some(s) = since {
        if !s.is_empty() {
            req = req.query(&[("since", s)]);
        }
    }
    let resp = req.send().await?.error_for_status()?;
    Ok(resp.json::<PullResponse>().await?)
}

/// Kirim perubahan master data lokal ke server (server menerapkan Last Write Wins).
pub async fn push(
    server_url: &str,
    store_id: &str,
    products: &[Product],
) -> AppResult<PushResponse> {
    let url = format!("{}/api/v1/sync/products/push", server_url.trim_end_matches('/'));
    let body = PushRequest { store_id, products };
    let resp = client()?.post(&url).json(&body).send().await?.error_for_status()?;
    Ok(resp.json::<PushResponse>().await?)
}

/// Hard push: server menimpa SSoT tanpa Last Write Wins. Produk SSoT yang
/// merek-nya ada di `exclude_brands` tidak ditimpa. Server lama (tanpa
/// endpoint ini) akan membalas 404 — sengaja endpoint terpisah supaya tidak
/// diam-diam jatuh ke push LWW biasa.
pub async fn hard_push(
    server_url: &str,
    store_id: &str,
    products: &[Product],
    exclude_brands: &[String],
) -> AppResult<PushResponse> {
    let url = format!("{}/api/v1/sync/products/hard-push", server_url.trim_end_matches('/'));
    let body = HardPushRequest { store_id, products, exclude_brands };
    let resp = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()?
        .post(&url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.json::<PushResponse>().await?)
}
