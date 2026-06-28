use rusqlite::params;
use tauri::State;

use crate::db::models::{new_id, now_ms, Shift};
use crate::db::{enqueue, write_audit};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[tauri::command]
pub fn current_shift(state: State<AppState>) -> AppResult<Option<Shift>> {
    state.require_session()?;
    let conn = state.pool.get()?;
    let shift = conn
        .query_row(
            "SELECT * FROM shifts WHERE status = 'open' ORDER BY opened_at DESC LIMIT 1",
            [],
            |r| Ok(Shift::from_row(r)),
        )
        .ok()
        .transpose()?;
    Ok(shift)
}

#[tauri::command]
pub fn open_shift(state: State<AppState>, opening_cash: f64) -> AppResult<Shift> {
    let session = state.require_session()?;
    let mut conn = state.pool.get()?;

    let already_open: bool = conn.query_row(
        "SELECT COUNT(*) FROM shifts WHERE status = 'open'",
        [],
        |r| r.get::<_, i64>(0),
    )? > 0;
    if already_open {
        return Err(AppError::Validation("Masih ada shift yang terbuka".into()));
    }

    let tx = conn.transaction()?;
    let id = new_id();
    let ts = now_ms();
    tx.execute(
        "INSERT INTO shifts
            (id, store_id, user_id, opening_cash, total_sales, opened_at, status, created_at, updated_at, sync_status)
         VALUES (?1, ?2, ?3, ?4, 0, ?5, 'open', ?5, ?5, 'pending')",
        params![id, state.store_id, session.user.id, opening_cash, ts],
    )?;
    let shift = tx.query_row("SELECT * FROM shifts WHERE id = ?1", params![id], |r| {
        Ok(Shift::from_row(r))
    })??;
    enqueue(&tx, "shift", &id, "insert", &serde_json::to_value(&shift)?, None)?;
    write_audit(&tx, &state.store_id, &session.user.id, "open_shift", Some("shift"), Some(&id), None)?;
    tx.commit()?;
    Ok(shift)
}

#[tauri::command]
pub fn close_shift(state: State<AppState>, closing_cash: f64) -> AppResult<Shift> {
    let session = state.require_session()?;
    let mut conn = state.pool.get()?;
    let tx = conn.transaction()?;

    let shift = tx
        .query_row(
            "SELECT * FROM shifts WHERE status = 'open' ORDER BY opened_at DESC LIMIT 1",
            [],
            |r| Ok(Shift::from_row(r)),
        )
        .map_err(|_| AppError::Validation("Tidak ada shift yang terbuka".into()))??;

    let total_sales: f64 = tx.query_row(
        "SELECT COALESCE(SUM(total), 0) FROM sales
         WHERE status = 'completed' AND shift_id = ?1",
        params![shift.id],
        |r| r.get(0),
    )?;
    let expected = shift.opening_cash + total_sales;
    let ts = now_ms();
    tx.execute(
        "UPDATE shifts SET closing_cash=?2, expected_cash=?3, total_sales=?4,
             closed_at=?5, status='closed', updated_at=?5, sync_status='pending'
         WHERE id=?1",
        params![shift.id, closing_cash, expected, total_sales, ts],
    )?;
    let updated = tx.query_row("SELECT * FROM shifts WHERE id = ?1", params![shift.id], |r| {
        Ok(Shift::from_row(r))
    })??;
    enqueue(&tx, "shift", &shift.id, "update", &serde_json::to_value(&updated)?, Some(shift.updated_at))?;
    write_audit(&tx, &state.store_id, &session.user.id, "close_shift", Some("shift"), Some(&shift.id), None)?;
    tx.commit()?;
    Ok(updated)
}
