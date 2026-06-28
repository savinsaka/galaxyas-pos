use std::sync::{Arc, Mutex};

use crate::db::models::{Session, SyncStatusInfo};
use crate::db::pool::DbPool;

/// Shared application state injected into every Tauri command via `State<_>`.
pub struct AppState {
    pub pool: DbPool,
    /// The store this installation belongs to (one cashier == one store).
    pub store_id: String,
    /// Currently authenticated session, if any.
    pub session: Mutex<Option<Session>>,
    /// Latest known sync status (shared with the background worker).
    pub sync_status: Arc<Mutex<SyncStatusInfo>>,
    /// HTTP client used by the sync worker (rustls, keep-alive).
    pub http: reqwest::Client,
    /// Base URL of the FastAPI sync server.
    pub server_url: String,
}

impl AppState {
    pub fn require_session(&self) -> Result<Session, crate::error::AppError> {
        self.session
            .lock()
            .unwrap()
            .clone()
            .ok_or(crate::error::AppError::Unauthorized)
    }

    pub fn require_role(&self, roles: &[&str]) -> Result<Session, crate::error::AppError> {
        let session = self.require_session()?;
        if roles.contains(&session.user.role.as_str()) {
            Ok(session)
        } else {
            Err(crate::error::AppError::Forbidden(format!(
                "role '{}' tidak diizinkan",
                session.user.role
            )))
        }
    }
}
