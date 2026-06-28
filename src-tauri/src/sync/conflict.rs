use rusqlite::params;

use crate::db::models::{new_id, now_ms};
use crate::error::AppResult;
use crate::sync::SyncHandle;

/// Persist a conflict reported by the server (e.g. barcode collision) and flag
/// the local entity so the UI can surface it for manual review.
pub fn record_conflict(
    handle: &SyncHandle,
    entity_type: &str,
    entity_id: &str,
    server_payload: &serde_json::Value,
    conflict_field: Option<&str>,
) -> AppResult<()> {
    let conn = handle.pool.get()?;
    let local_payload: String = conn
        .query_row(
            "SELECT payload FROM sync_queue WHERE entity_id = ?1 ORDER BY created_at DESC LIMIT 1",
            params![entity_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "{}".to_string());

    conn.execute(
        "INSERT INTO sync_conflicts
            (id, entity_type, entity_id, local_payload, server_payload, conflict_field,
             resolution, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7)",
        params![
            new_id(),
            entity_type,
            entity_id,
            local_payload,
            server_payload.to_string(),
            conflict_field,
            now_ms(),
        ],
    )?;

    let table = entity_table(entity_type);
    if let Some(table) = table {
        let _ = conn.execute(
            &format!("UPDATE {table} SET sync_status='conflict' WHERE id=?1"),
            params![entity_id],
        );
    }
    tracing::warn!(entity_type, entity_id, "sync conflict recorded");
    Ok(())
}

pub fn entity_table(entity_type: &str) -> Option<&'static str> {
    match entity_type {
        "item" | "item_stock" => Some("items"),
        "store" => Some("stores"),
        "sale" => Some("sales"),
        "stock_transaction" => Some("stock_transactions"),
        "shift" => Some("shifts"),
        _ => None,
    }
}
