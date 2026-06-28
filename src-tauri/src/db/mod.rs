pub mod models;
pub mod pool;

use rusqlite::{params, Connection};

use crate::db::models::{new_id, now_ms};
use crate::error::AppResult;

/// Append a change to the local `sync_queue`. MUST be called inside the same
/// transaction that mutated the entity so data and its outbound queue entry are
/// committed atomically (no record can ever escape synchronization).
pub fn enqueue(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
    operation: &str,
    payload: &serde_json::Value,
    base_updated_at: Option<i64>,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO sync_queue
            (id, entity_type, entity_id, operation, payload, base_updated_at, status, retry_count, created_at, next_retry_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', 0, ?7, 0)",
        params![
            new_id(),
            entity_type,
            entity_id,
            operation,
            payload.to_string(),
            base_updated_at,
            now_ms(),
        ],
    )?;
    Ok(())
}

/// Write an audit-log entry. Audit logs are local-only history and are not part
/// of the sync queue.
pub fn write_audit(
    conn: &Connection,
    store_id: &str,
    user_id: &str,
    action: &str,
    entity_type: Option<&str>,
    entity_id: Option<&str>,
    detail: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO audit_logs
            (id, store_id, user_id, action, entity_type, entity_id, detail, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            new_id(),
            store_id,
            user_id,
            action,
            entity_type,
            entity_id,
            detail,
            now_ms(),
        ],
    )?;
    Ok(())
}

/// Read a key from `sync_meta` (e.g. `last_pull_at`).
pub fn get_meta(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    let value = conn
        .query_row(
            "SELECT value FROM sync_meta WHERE key = ?1",
            params![key],
            |r| r.get::<_, String>(0),
        )
        .ok();
    Ok(value)
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO sync_meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}
