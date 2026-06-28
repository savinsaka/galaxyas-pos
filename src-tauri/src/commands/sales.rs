use rusqlite::params;
use tauri::State;

use crate::db::models::{new_id, now_ms, Sale, SaleInput, SaleItem, SaleWithItems};
use crate::db::{enqueue, write_audit};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

fn next_invoice_no(conn: &rusqlite::Connection) -> AppResult<String> {
    let today = chrono::Utc::now().format("%Y%m%d").to_string();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sales WHERE invoice_no LIKE ?1",
        params![format!("INV-{today}-%")],
        |r| r.get(0),
    )?;
    Ok(format!("INV-{today}-{:04}", count + 1))
}

#[tauri::command]
pub fn create_sale(state: State<AppState>, input: SaleInput) -> AppResult<Sale> {
    let session = state.require_role(&["admin", "supervisor", "kasir"])?;
    if input.items.is_empty() {
        return Err(AppError::Validation("Keranjang masih kosong".into()));
    }

    let mut conn = state.pool.get()?;
    let tx = conn.transaction()?;
    let ts = now_ms();
    let sale_id = new_id();

    let subtotal: f64 = input
        .items
        .iter()
        .map(|l| l.harga * l.qty - l.diskon)
        .sum();
    let after_diskon = (subtotal - input.diskon).max(0.0);
    let pajak = (after_diskon * input.pajak_persen / 100.0).round();
    let total = after_diskon + pajak;
    let completed = input.status == "completed";
    let kembali = if completed {
        (input.bayar - total).max(0.0)
    } else {
        0.0
    };
    if completed && input.bayar < total {
        return Err(AppError::Validation("Pembayaran kurang dari total".into()));
    }

    let invoice = if completed {
        Some(next_invoice_no(&tx)?)
    } else {
        None
    };

    tx.execute(
        "INSERT INTO sales
            (id, store_id, invoice_no, user_id, shift_id, subtotal, diskon, pajak, total,
             bayar, kembali, status, created_at, updated_at, sync_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13, 'pending')",
        params![
            sale_id,
            state.store_id,
            invoice,
            session.user.id,
            input.shift_id,
            subtotal,
            input.diskon,
            pajak,
            total,
            input.bayar,
            kembali,
            input.status,
            ts,
        ],
    )?;

    for line in &input.items {
        let line_id = new_id();
        let line_subtotal = line.harga * line.qty - line.diskon;
        tx.execute(
            "INSERT INTO sale_items
                (id, sale_id, item_id, nama_item, qty, harga, diskon, subtotal,
                 created_at, updated_at, sync_status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, 'pending')",
            params![
                line_id,
                sale_id,
                line.item_id,
                line.nama_item,
                line.qty,
                line.harga,
                line.diskon,
                line_subtotal,
                ts,
            ],
        )?;
        // Completed sales decrement stock immediately.
        if completed {
            tx.execute(
                "UPDATE items SET stok = stok - ?2, updated_at = ?3, sync_status='pending'
                 WHERE id = ?1",
                params![line.item_id, line.qty, ts],
            )?;
        }
    }

    let sale = tx.query_row("SELECT * FROM sales WHERE id = ?1", params![sale_id], |r| {
        Ok(Sale::from_row(r))
    })??;

    enqueue(&tx, "sale", &sale_id, "insert", &serde_json::to_value(&sale)?, None)?;
    write_audit(
        &tx,
        &state.store_id,
        &session.user.id,
        if completed { "create_sale" } else { "hold_sale" },
        Some("sale"),
        Some(&sale_id),
        None,
    )?;
    tx.commit()?;
    Ok(sale)
}

#[tauri::command]
pub fn get_sale(state: State<AppState>, id: String) -> AppResult<SaleWithItems> {
    state.require_session()?;
    let conn = state.pool.get()?;
    let sale = conn
        .query_row("SELECT * FROM sales WHERE id = ?1", params![id], |r| {
            Ok(Sale::from_row(r))
        })
        .map_err(|_| AppError::NotFound("Transaksi tidak ditemukan".into()))??;
    let mut stmt = conn.prepare("SELECT * FROM sale_items WHERE sale_id = ?1")?;
    let items = stmt
        .query_map(params![id], |r| Ok(SaleItem::from_row(r)))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect::<AppResult<Vec<_>>>()?;
    Ok(SaleWithItems { sale, items })
}

#[tauri::command]
pub fn list_held_sales(state: State<AppState>) -> AppResult<Vec<Sale>> {
    state.require_session()?;
    let conn = state.pool.get()?;
    let mut stmt = conn
        .prepare("SELECT * FROM sales WHERE status = 'held' ORDER BY created_at DESC")?;
    let rows = stmt
        .query_map([], |r| Ok(Sale::from_row(r)))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect::<AppResult<Vec<_>>>()?;
    Ok(rows)
}

#[tauri::command]
pub fn void_sale(state: State<AppState>, id: String, reason: String) -> AppResult<()> {
    let session = state.require_role(&["admin", "supervisor"])?;
    let mut conn = state.pool.get()?;
    let tx = conn.transaction()?;
    let ts = now_ms();
    tx.execute(
        "UPDATE sales SET status='void', updated_at=?2, sync_status='pending' WHERE id=?1",
        params![id, ts],
    )?;
    let sale = tx.query_row("SELECT * FROM sales WHERE id = ?1", params![id], |r| {
        Ok(Sale::from_row(r))
    })??;
    enqueue(&tx, "sale", &id, "update", &serde_json::to_value(&sale)?, Some(sale.updated_at))?;
    write_audit(&tx, &state.store_id, &session.user.id, "void_sale", Some("sale"), Some(&id), Some(&reason))?;
    tx.commit()?;
    Ok(())
}
