use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::{Product, ProductInput};

/// Server SSoT resmi. Tidak bisa diganti dari UI (pengaturan URL diganti
/// login anggota); env `GPOS_SYNC_SERVER` hanya untuk tes lokal developer.
pub const DEFAULT_SERVER: &str = "https://jjapps.net";

pub fn default_server() -> String {
    std::env::var("GPOS_SYNC_SERVER")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_SERVER.to_string())
}

/// Koneksi ke server sync untuk akun yang sedang login di PC ini.
#[derive(Debug, Clone)]
pub struct Server {
    pub url: String,
    pub store_id: String,
    pub token: String,
}

impl Server {
    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.url.trim_end_matches('/'), path)
    }
}

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
}

fn client_with_timeout(secs: u64) -> AppResult<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(secs))
        .build()?)
}

fn client() -> AppResult<reqwest::Client> {
    client_with_timeout(30)
}

/// Status HTTP gagal → pesan yang bisa dibaca kasir. FastAPI mengirim alasan
/// di `detail` (login salah, sesi dicabut, barcode dobel, …); 404 tanpa detail
/// berarti server belum punya endpoint itu.
async fn api_error(resp: reqwest::Response) -> AppError {
    let status = resp.status();
    let detail = resp
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|v| v.get("detail").and_then(|d| d.as_str()).map(str::to_string));
    match (status.as_u16(), detail) {
        (_, Some(d)) if d != "Not Found" => AppError::Other(d),
        (401, _) => AppError::Other("Sesi server habis atau dicabut. Login ulang di Sync → Akun Server.".into()),
        (404, _) => AppError::Other("Server sync belum diperbarui untuk fitur ini.".into()),
        (code, _) => AppError::Other(format!("Server sync membalas status {code}.")),
    }
}

async fn send<T: serde::de::DeserializeOwned>(req: reqwest::RequestBuilder) -> AppResult<T> {
    let resp = req.send().await?;
    if !resp.status().is_success() {
        return Err(api_error(resp).await);
    }
    Ok(resp.json::<T>().await?)
}

/// Tarik master data yang berubah setelah `since` (Delta Sync).
pub async fn pull(srv: &Server, since: Option<&str>) -> AppResult<PullResponse> {
    let mut req = client()?
        .get(srv.endpoint("/api/v1/sync/products/pull"))
        .bearer_auth(&srv.token)
        .query(&[("store_id", srv.store_id.as_str())]);
    if let Some(s) = since {
        if !s.is_empty() {
            req = req.query(&[("since", s)]);
        }
    }
    send(req).await
}

/// Kirim perubahan master data lokal ke server (server menerapkan Last Write Wins).
pub async fn push(srv: &Server, products: &[Product]) -> AppResult<PushResponse> {
    let body = PushRequest { store_id: &srv.store_id, products };
    send(client()?.post(srv.endpoint("/api/v1/sync/products/push")).bearer_auth(&srv.token).json(&body)).await
}

// ---------- Layar SSoT (lihat & ubah isi server langsung) ----------

#[derive(Debug, Serialize)]
struct SsotSaveRequest<'a> {
    store_id: &'a str,
    product: &'a ProductInput,
}

pub async fn ssot_list(srv: &Server) -> AppResult<Vec<Product>> {
    send(client()?.get(srv.endpoint("/api/v1/ssot/products")).bearer_auth(&srv.token)).await
}

pub async fn ssot_save(srv: &Server, product: &ProductInput) -> AppResult<Product> {
    let body = SsotSaveRequest { store_id: &srv.store_id, product };
    send(client()?.post(srv.endpoint("/api/v1/ssot/products")).bearer_auth(&srv.token).json(&body)).await
}

// ---------- Login anggota ----------

#[derive(Debug, Serialize)]
struct LoginRequest<'a> {
    username: &'a str,
    password: &'a str,
    device: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
    pub name: String,
}

pub async fn login(server_url: &str, username: &str, password: &str, device: &str) -> AppResult<LoginResponse> {
    let url = format!("{}/api/v1/auth/login", server_url.trim_end_matches('/'));
    let body = LoginRequest { username, password, device };
    send(client()?.post(url).json(&body)).await
}

/// Cabut token di server. Gagal (offline/token sudah dicabut) diabaikan
/// pemanggil — token lokal tetap dihapus.
pub async fn logout(srv: &Server) -> AppResult<()> {
    let _: serde_json::Value =
        send(client_with_timeout(10)?.post(srv.endpoint("/api/v1/auth/logout")).bearer_auth(&srv.token)).await?;
    Ok(())
}

// ---------- Kebijakan wajib update ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPolicy {
    pub min_version: String,
    pub deadline: Option<String>,
    pub message: String,
}

/// Tanpa login (dibaca saat app dibuka). Timeout pendek: server tidak
/// terjangkau berarti tidak ada kebijakan, app tidak boleh tertahan.
pub async fn app_policy(server_url: &str, app: &str) -> AppResult<AppPolicy> {
    let url = format!("{}/api/v1/app/policy", server_url.trim_end_matches('/'));
    send(client_with_timeout(6)?.get(url).query(&[("app", app)])).await
}
