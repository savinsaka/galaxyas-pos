use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::models::now_ms;
use crate::error::AppResult;
use crate::sync::{conflict, SyncHandle};

const BATCH_SIZE: i64 = 100;
const BACKOFF_BASE_MS: i64 = 60_000; // 1 minute
const BACKOFF_CAP_MS: i64 = 3_600_000; // 1 hour

#[derive(Debug, Serialize)]
struct PushItem {
    queue_id: String,
    entity_type: String,
    entity_id: String,
    operation: String,
    payload: serde_json::Value,
    base_updated_at: Option<i64>,
}

#[derive(Debug, Serialize)]
struct PushRequest {
    store_id: String,
    changes: Vec<PushItem>,
}

#[derive(Debug, Deserialize)]
struct ConflictItem {
    queue_id: String,
    entity_type: String,
    entity_id: String,
    server_payload: serde_json::Value,
    conflict_field: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PushResponse {
    accepted: Vec<String>,
    #[serde(default)]
    conflicts: Vec<ConflictItem>,
}

/// Read a batch of due queue rows (sync, no await held).
fn take_batch(handle: &SyncHandle) -> AppResult<Vec<PushItem>> {
    let conn = handle.pool.get()?;
    let now = now_ms();
    let mut stmt = conn.prepare(
        "SELECT id, entity_type, entity_id, operation, payload, base_updated_at
         FROM sync_queue
         WHERE status IN ('pending','failed') AND next_retry_at <= ?1
         ORDER BY created_at ASC LIMIT ?2",
    )?;
    let items = stmt
        .query_map(params![now, BATCH_SIZE], |r| {
            let payload_str: String = r.get("payload")?;
            Ok(PushItem {
                queue_id: r.get("id")?,
                entity_type: r.get("entity_type")?,
                entity_id: r.get("entity_id")?,
                operation: r.get("operation")?,
                payload: serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null),
                base_updated_at: r.get("base_updated_at")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

/// Mark the given queue rows acked and their entities synced (one transaction).
fn mark_accepted(handle: &SyncHandle, queue_ids: &[String]) -> AppResult<()> {
    if queue_ids.is_empty() {
        return Ok(());
    }
    let mut conn = handle.pool.get()?;
    let tx = conn.transaction()?;
    for qid in queue_ids {
        // Mark the entity row synced based on the queue entry's mapping.
        if let Ok((entity_type, entity_id)) = tx.query_row(
            "SELECT entity_type, entity_id FROM sync_queue WHERE id = ?1",
            params![qid],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        ) {
            mark_entity_synced(&tx, &entity_type, &entity_id);
        }
        tx.execute(
            "UPDATE sync_queue SET status='acked' WHERE id=?1",
            params![qid],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn mark_entity_synced(conn: &rusqlite::Connection, entity_type: &str, entity_id: &str) {
    let table = match entity_type {
        "item" | "item_stock" => "items",
        "sale" => "sales",
        "stock_transaction" => "stock_transactions",
        "shift" => "shifts",
        "store" => "stores",
        _ => return,
    };
    let _ = conn.execute(
        &format!("UPDATE {table} SET sync_status='synced' WHERE id=?1"),
        params![entity_id],
    );
}

/// Apply backoff to a failed batch so we don't hammer an unavailable server.
fn apply_backoff(handle: &SyncHandle, queue_ids: &[String], err: &str) -> AppResult<()> {
    let mut conn = handle.pool.get()?;
    let tx = conn.transaction()?;
    for qid in queue_ids {
        let retry: i64 = tx
            .query_row(
                "SELECT retry_count FROM sync_queue WHERE id=?1",
                params![qid],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let delay = (BACKOFF_BASE_MS.saturating_mul(1 << retry.min(6))).min(BACKOFF_CAP_MS);
        tx.execute(
            "UPDATE sync_queue SET status='failed', retry_count=retry_count+1,
                 last_error=?2, next_retry_at=?3 WHERE id=?1",
            params![qid, err, now_ms() + delay],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Push pending changes to the server. Idempotent: the server dedupes by UUID.
pub async fn push_pending(handle: &SyncHandle) -> AppResult<()> {
    loop {
        let batch = take_batch(handle)?;
        if batch.is_empty() {
            break;
        }
        let queue_ids: Vec<String> = batch.iter().map(|b| b.queue_id.clone()).collect();

        let req = PushRequest {
            store_id: handle.store_id.clone(),
            changes: batch,
        };
        let url = format!("{}/api/v1/sync/push", handle.server_url);
        let resp = handle.http.post(&url).json(&req).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let body: PushResponse = r.json().await?;
                mark_accepted(handle, &body.accepted)?;
                for c in body.conflicts {
                    conflict::record_conflict(
                        handle,
                        &c.entity_type,
                        &c.entity_id,
                        &c.server_payload,
                        c.conflict_field.as_deref(),
                    )?;
                    // The conflicting queue row is considered handled.
                    mark_accepted(handle, &[c.queue_id])?;
                }
            }
            Ok(r) => {
                let status = r.status();
                apply_backoff(handle, &queue_ids, &format!("HTTP {status}"))?;
                return Err(crate::error::AppError::Other(format!(
                    "push gagal: HTTP {status}"
                )));
            }
            Err(e) => {
                apply_backoff(handle, &queue_ids, &e.to_string())?;
                return Err(e.into());
            }
        }

        if queue_ids.len() < BATCH_SIZE as usize {
            break;
        }
    }
    Ok(())
}
