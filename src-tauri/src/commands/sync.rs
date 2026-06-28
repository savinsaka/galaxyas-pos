use rusqlite::params;
use tauri::State;

use crate::db::models::{now_ms, SyncConflict, SyncStatusInfo};
use crate::db::write_audit;
use crate::error::AppResult;
use crate::state::AppState;
use crate::sync;

/// Compute the live sync status from the local database.
pub fn compute_status(state: &AppState) -> AppResult<SyncStatusInfo> {
    let conn = state.pool.get()?;
    let pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sync_queue WHERE status IN ('pending','failed')",
        [],
        |r| r.get(0),
    )?;
    let conflicts: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sync_conflicts WHERE resolution = 'pending'",
        [],
        |r| r.get(0),
    )?;
    let last = state.sync_status.lock().unwrap().clone();
    Ok(SyncStatusInfo {
        state: last.state,
        pending_count: pending,
        conflict_count: conflicts,
        last_sync_at: last.last_sync_at,
        last_error: last.last_error,
    })
}

#[tauri::command]
pub fn sync_status(state: State<AppState>) -> AppResult<SyncStatusInfo> {
    state.require_session()?;
    compute_status(&state)
}

#[tauri::command]
pub async fn trigger_sync(state: State<'_, AppState>) -> AppResult<SyncStatusInfo> {
    state.require_session()?;
    sync::run_once(&state).await?;
    compute_status(&state)
}

#[tauri::command]
pub fn list_conflicts(state: State<AppState>) -> AppResult<Vec<SyncConflict>> {
    state.require_session()?;
    let conn = state.pool.get()?;
    let mut stmt = conn
        .prepare("SELECT * FROM sync_conflicts ORDER BY created_at DESC LIMIT 200")?;
    let rows = stmt
        .query_map([], |r| Ok(SyncConflict::from_row(r)))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect::<AppResult<Vec<_>>>()?;
    Ok(rows)
}

#[tauri::command]
pub fn resolve_conflict(
    state: State<AppState>,
    id: String,
    resolution: String,
) -> AppResult<()> {
    let session = state.require_role(&["admin", "supervisor"])?;
    let conn = state.pool.get()?;
    let ts = now_ms();
    conn.execute(
        "UPDATE sync_conflicts SET resolution=?2, resolved_by=?3, resolved_at=?4 WHERE id=?1",
        params![id, resolution, session.user.id, ts],
    )?;
    write_audit(&conn, &state.store_id, &session.user.id, "resolve_conflict", Some("sync_conflict"), Some(&id), Some(&resolution))?;
    Ok(())
}
