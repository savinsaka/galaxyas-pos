use rusqlite::params;
use serde::Deserialize;
use tauri::State;

use crate::db::models::{new_id, now_ms, StockMovementInput, StockTransaction};
use crate::db::{enqueue, write_audit};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Apply a stock movement and write the resulting transaction. `in`/`out` are
/// relative; `adjustment`/`opname` set the absolute on-hand quantity.
fn apply_movement(
    tx: &rusqlite::Connection,
    store_id: &str,
    user_id: &str,
    input: &StockMovementInput,
) -> AppResult<StockTransaction> {
    let before: f64 = tx
        .query_row(
            "SELECT stok FROM items WHERE id = ?1 AND deleted_at IS NULL",
            params![input.item_id],
            |r| r.get(0),
        )
        .map_err(|_| AppError::NotFound("Item tidak ditemukan".into()))?;

    let after = match input.tx_type.as_str() {
        "in" => before + input.qty,
        "out" => before - input.qty,
        "adjustment" | "opname" => input.qty,
        other => return Err(AppError::Validation(format!("tipe tidak valid: {other}"))),
    };

    let ts = now_ms();
    tx.execute(
        "UPDATE items SET stok = ?2, updated_at = ?3, sync_status = 'pending' WHERE id = ?1",
        params![input.item_id, after, ts],
    )?;

    let id = new_id();
    tx.execute(
        "INSERT INTO stock_transactions
            (id, store_id, item_id, type, qty, qty_before, qty_after, ref_doc, note,
             user_id, created_at, updated_at, sync_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, 'pending')",
        params![
            id,
            store_id,
            input.item_id,
            input.tx_type,
            input.qty,
            before,
            after,
            input.ref_doc,
            input.note,
            user_id,
            ts,
        ],
    )?;

    let stx = tx.query_row(
        "SELECT * FROM stock_transactions WHERE id = ?1",
        params![id],
        |r| Ok(StockTransaction::from_row(r)),
    )??;

    enqueue(
        tx,
        "stock_transaction",
        &id,
        "insert",
        &serde_json::to_value(&stx)?,
        None,
    )?;
    // The item's new stock also needs to sync.
    enqueue(tx, "item_stock", &input.item_id, "update", &serde_json::json!({ "stok": after }), None)?;
    Ok(stx)
}

#[tauri::command]
pub fn record_stock_movement(
    state: State<AppState>,
    input: StockMovementInput,
) -> AppResult<StockTransaction> {
    let session = state.require_role(&["admin", "supervisor", "kasir"])?;
    if input.qty == 0.0 {
        return Err(AppError::Validation("Qty tidak boleh 0".into()));
    }
    let mut conn = state.pool.get()?;
    let tx = conn.transaction()?;
    let stx = apply_movement(&tx, &state.store_id, &session.user.id, &input)?;
    write_audit(
        &tx,
        &state.store_id,
        &session.user.id,
        "record_stock_movement",
        Some("stock_transaction"),
        Some(&stx.id),
        Some(&input.tx_type),
    )?;
    tx.commit()?;
    Ok(stx)
}

#[tauri::command]
pub fn list_stock_transactions(
    state: State<AppState>,
    item_id: Option<String>,
    limit: i64,
) -> AppResult<Vec<StockTransaction>> {
    state.require_session()?;
    let conn = state.pool.get()?;
    let (sql, want_filter) = match &item_id {
        Some(_) => (
            "SELECT * FROM stock_transactions WHERE item_id = ?1 ORDER BY created_at DESC LIMIT ?2",
            true,
        ),
        None => (
            "SELECT * FROM stock_transactions ORDER BY created_at DESC LIMIT ?1",
            false,
        ),
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = if want_filter {
        stmt.query_map(params![item_id.unwrap(), limit], |r| {
            Ok(StockTransaction::from_row(r))
        })?
        .collect::<Result<Vec<_>, _>>()?
    } else {
        stmt.query_map(params![limit], |r| Ok(StockTransaction::from_row(r)))?
            .collect::<Result<Vec<_>, _>>()?
    };
    rows.into_iter().collect::<AppResult<Vec<_>>>()
}

#[derive(Debug, Deserialize)]
pub struct OpnameRow {
    pub item_id: String,
    pub counted_qty: f64,
}

#[tauri::command]
pub fn apply_stock_opname(
    state: State<AppState>,
    rows: Vec<OpnameRow>,
    ref_doc: String,
) -> AppResult<i64> {
    let session = state.require_role(&["admin", "supervisor"])?;
    let mut conn = state.pool.get()?;
    let tx = conn.transaction()?;
    let mut count = 0;
    for row in rows {
        let input = StockMovementInput {
            item_id: row.item_id,
            tx_type: "opname".into(),
            qty: row.counted_qty,
            ref_doc: Some(ref_doc.clone()),
            note: Some("Stock opname".into()),
        };
        apply_movement(&tx, &state.store_id, &session.user.id, &input)?;
        count += 1;
    }
    write_audit(&tx, &state.store_id, &session.user.id, "apply_stock_opname", Some("stock_transaction"), None, Some(&ref_doc))?;
    tx.commit()?;
    Ok(count)
}
