use rusqlite::params;
use serde::Deserialize;

use crate::db::models::Item;
use crate::db::{get_meta, set_meta};
use crate::error::AppResult;
use crate::sync::SyncHandle;

#[derive(Debug, Deserialize)]
struct PullChange {
    entity_type: String,
    entity_id: String,
    payload: serde_json::Value,
    server_updated_at: i64,
    #[serde(default)]
    deleted: bool,
}

#[derive(Debug, Deserialize)]
struct PullResponse {
    changes: Vec<PullChange>,
    server_time: i64,
}

/// Pull remote changes (master data, prices) that other stores have pushed.
/// Applies Last-Write-Wins by comparing `server_updated_at` to the local
/// `updated_at`. Sales are store-owned and are not pulled back.
pub async fn pull_changes(handle: &SyncHandle) -> AppResult<()> {
    let since: i64 = {
        let conn = handle.pool.get()?;
        get_meta(&conn, "last_pull_at")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    };

    let url = format!(
        "{}/api/v1/sync/pull?since={}&store_id={}",
        handle.server_url, since, handle.store_id
    );
    let resp = handle.http.get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(crate::error::AppError::Other(format!(
            "pull gagal: HTTP {}",
            resp.status()
        )));
    }
    let body: PullResponse = resp.json().await?;

    // Apply merges in a single transaction (sync, no await held).
    let mut conn = handle.pool.get()?;
    let tx = conn.transaction()?;
    for change in &body.changes {
        match change.entity_type.as_str() {
            "item" => merge_item(&tx, change)?,
            // Other master-data entities can be added here as needed.
            _ => {}
        }
    }
    set_meta(&tx, "last_pull_at", &body.server_time.to_string())?;
    tx.commit()?;
    Ok(())
}

fn merge_item(tx: &rusqlite::Connection, change: &PullChange) -> AppResult<()> {
    let local_updated: Option<i64> = tx
        .query_row(
            "SELECT updated_at FROM items WHERE id = ?1",
            params![change.entity_id],
            |r| r.get(0),
        )
        .ok();

    // LWW: skip if our local copy is newer or equal.
    if let Some(local) = local_updated {
        if local >= change.server_updated_at {
            return Ok(());
        }
    }

    let incoming: Item = match serde_json::from_value(change.payload.clone()) {
        Ok(i) => i,
        Err(_) => return Ok(()),
    };
    let deleted_at = if change.deleted {
        Some(change.server_updated_at)
    } else {
        incoming.deleted_at
    };

    tx.execute(
        "INSERT INTO items
            (id, store_id, kode_item, barcode, nama_item, jenis, merek, satuan_dasar,
             harga_beli, harga_jual, diskon_persen, stok, created_at, updated_at, deleted_at, sync_status)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,'synced')
         ON CONFLICT(id) DO UPDATE SET
            kode_item=excluded.kode_item, barcode=excluded.barcode, nama_item=excluded.nama_item,
            jenis=excluded.jenis, merek=excluded.merek, satuan_dasar=excluded.satuan_dasar,
            harga_beli=excluded.harga_beli, harga_jual=excluded.harga_jual,
            diskon_persen=excluded.diskon_persen, updated_at=excluded.updated_at,
            deleted_at=excluded.deleted_at, sync_status='synced'",
        params![
            incoming.id,
            incoming.store_id,
            incoming.kode_item,
            incoming.barcode,
            incoming.nama_item,
            incoming.jenis,
            incoming.merek,
            incoming.satuan_dasar,
            incoming.harga_beli,
            incoming.harga_jual,
            incoming.diskon_persen,
            incoming.stok,
            incoming.created_at,
            change.server_updated_at,
            deleted_at,
        ],
    )?;
    Ok(())
}
