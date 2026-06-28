pub mod conflict;
pub mod pull;
pub mod push;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::db::models::{now_ms, SyncStatusInfo};
use crate::db::pool::DbPool;
use crate::error::AppResult;
use crate::state::AppState;

/// All the cloneable pieces the sync engine needs. Kept separate from
/// `AppState` so the background worker can own a `'static` copy while commands
/// continue to borrow `AppState`.
#[derive(Clone)]
pub struct SyncHandle {
    pub pool: DbPool,
    pub http: reqwest::Client,
    pub server_url: String,
    pub store_id: String,
    pub status: Arc<Mutex<SyncStatusInfo>>,
}

impl SyncHandle {
    pub fn from_state(state: &AppState) -> Self {
        Self {
            pool: state.pool.clone(),
            http: state.http.clone(),
            server_url: state.server_url.clone(),
            store_id: state.store_id.clone(),
            status: state.sync_status.clone(),
        }
    }

    fn set_state(&self, s: &str, error: Option<String>) {
        let mut guard = self.status.lock().unwrap();
        guard.state = s.to_string();
        guard.last_error = error;
    }

    fn mark_synced(&self) {
        let mut guard = self.status.lock().unwrap();
        guard.state = "idle".into();
        guard.last_sync_at = Some(now_ms());
        guard.last_error = None;
    }
}

/// Run one full sync cycle: push the outbound queue, then pull remote changes.
/// Network and DB access never overlap an `.await` while holding a connection,
/// so the UI command path is never blocked by an in-flight request.
pub async fn run_cycle(handle: &SyncHandle) -> AppResult<()> {
    handle.set_state("syncing", None);
    match push::push_pending(handle).await {
        Ok(_) => {}
        Err(e) => {
            // Network errors are expected when offline; surface as offline state.
            handle.set_state("offline", Some(e.to_string()));
            return Err(e);
        }
    }
    pull::pull_changes(handle).await?;
    handle.mark_synced();
    Ok(())
}

/// Used by the `trigger_sync` command (manual sync button).
pub async fn run_once(state: &AppState) -> AppResult<()> {
    let handle = SyncHandle::from_state(state);
    // Manual sync: ignore network errors so the UI can still report status.
    let _ = run_cycle(&handle).await;
    Ok(())
}

/// Spawn the periodic background worker (3-5 minutes; sync also fires on demand).
pub fn spawn_worker(handle: SyncHandle) {
    tokio::spawn(async move {
        // Small initial delay so app startup is not contended.
        tokio::time::sleep(Duration::from_secs(10)).await;
        let mut ticker = tokio::time::interval(Duration::from_secs(180));
        loop {
            ticker.tick().await;
            if let Err(e) = run_cycle(&handle).await {
                tracing::warn!(error = %e, "sync cycle failed (will retry)");
            }
        }
    });
}
