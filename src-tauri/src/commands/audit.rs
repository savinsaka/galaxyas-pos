use rusqlite::params;
use tauri::State;

use crate::db::models::AuditLog;
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub fn list_audit_logs(state: State<AppState>, limit: i64) -> AppResult<Vec<AuditLog>> {
    state.require_role(&["admin", "supervisor"])?;
    let conn = state.pool.get()?;
    let mut stmt = conn
        .prepare("SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT ?1")?;
    let rows = stmt
        .query_map(params![limit], |r| Ok(AuditLog::from_row(r)))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect::<AppResult<Vec<_>>>()?;
    Ok(rows)
}
