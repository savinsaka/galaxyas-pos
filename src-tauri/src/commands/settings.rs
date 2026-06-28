use rusqlite::params;
use serde::Deserialize;
use tauri::State;

use crate::db::models::{now_ms, PrinterSettings, Store};
use crate::db::{enqueue, get_meta, set_meta, write_audit};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[tauri::command]
pub fn get_store(state: State<AppState>) -> AppResult<Store> {
    state.require_session()?;
    let conn = state.pool.get()?;
    let store = conn
        .query_row(
            "SELECT * FROM stores WHERE id = ?1",
            params![state.store_id],
            |r| Ok(Store::from_row(r)),
        )
        .map_err(|_| AppError::NotFound("Toko tidak ditemukan".into()))??;
    Ok(store)
}

#[derive(Debug, Deserialize)]
pub struct StoreInput {
    pub name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub tax_percent: f64,
}

#[tauri::command]
pub fn update_store(state: State<AppState>, input: StoreInput) -> AppResult<Store> {
    let session = state.require_role(&["admin", "supervisor"])?;
    let mut conn = state.pool.get()?;
    let tx = conn.transaction()?;
    let ts = now_ms();
    tx.execute(
        "UPDATE stores SET name=?2, address=?3, phone=?4, tax_percent=?5,
             updated_at=?6, sync_status='pending' WHERE id=?1",
        params![
            state.store_id,
            input.name,
            input.address,
            input.phone,
            input.tax_percent,
            ts
        ],
    )?;
    let store = tx.query_row(
        "SELECT * FROM stores WHERE id = ?1",
        params![state.store_id],
        |r| Ok(Store::from_row(r)),
    )??;
    enqueue(&tx, "store", &store.id, "update", &serde_json::to_value(&store)?, Some(store.updated_at))?;
    write_audit(&tx, &state.store_id, &session.user.id, "update_store", Some("store"), Some(&store.id), None)?;
    tx.commit()?;
    Ok(store)
}

const PRINTER_KEY: &str = "printer_settings";

#[tauri::command]
pub fn get_printer_settings(state: State<AppState>) -> AppResult<PrinterSettings> {
    state.require_session()?;
    let conn = state.pool.get()?;
    let stored = get_meta(&conn, PRINTER_KEY)?;
    let settings = match stored {
        Some(json) => serde_json::from_str(&json)?,
        None => PrinterSettings {
            printer_name: "POS-58".into(),
            paper_width: 58,
            header_text: "GalaxyAS POS".into(),
            footer_text: "Terima kasih".into(),
        },
    };
    Ok(settings)
}

#[tauri::command]
pub fn update_printer_settings(
    state: State<AppState>,
    settings: PrinterSettings,
) -> AppResult<PrinterSettings> {
    state.require_role(&["admin", "supervisor"])?;
    let conn = state.pool.get()?;
    set_meta(&conn, PRINTER_KEY, &serde_json::to_string(&settings)?)?;
    Ok(settings)
}
