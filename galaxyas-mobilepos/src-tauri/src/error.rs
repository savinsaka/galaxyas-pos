use serde::Serialize;
use std::error::Error as StdError;
use std::fmt::Write as _;

/// Error aplikasi yang bisa di-serialize dan dikembalikan ke frontend.
/// (Salinan dari desktop/src-tauri/src/error.rs minus varian rusqlite —
/// mobile tidak punya database lokal.)
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

/// `reqwest::Error`'s `Display` sering cuma nampilin pesan generik ("error
/// sending request for url ...") sementara penyebab aslinya (DNS, TLS
/// handshake gagal, koneksi ditolak, dst) ada di `.source()` yang berlapis —
/// dirangkai di sini supaya pesan error ke user benar-benar actionable.
fn format_with_source_chain(err: &(dyn StdError + 'static)) -> String {
    let mut out = err.to_string();
    let mut cause = err.source();
    while let Some(c) = cause {
        let _ = write!(out, " — penyebab: {c}");
        cause = c.source();
    }
    out
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let msg = match self {
            AppError::Http(e) => format!("network error: {}", format_with_source_chain(e)),
            other => other.to_string(),
        };
        serializer.serialize_str(&msg)
    }
}

pub type AppResult<T> = Result<T, AppError>;
