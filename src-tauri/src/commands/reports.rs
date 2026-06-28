use rusqlite::params;
use tauri::State;

use crate::db::models::{DailySalesReport, StockReportRow, TopSellingProduct};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub fn daily_sales_report(
    state: State<AppState>,
    from_ms: i64,
    to_ms: i64,
) -> AppResult<Vec<DailySalesReport>> {
    state.require_session()?;
    let conn = state.pool.get()?;
    // SQLite stores epoch-ms; convert to local date for grouping.
    let mut stmt = conn.prepare(
        "SELECT date(created_at / 1000, 'unixepoch', 'localtime') AS d,
                COUNT(*) AS trx,
                COALESCE(SUM(total), 0) AS revenue,
                COALESCE(SUM(diskon), 0) AS discount,
                COALESCE(SUM(pajak), 0) AS tax
         FROM sales
         WHERE status = 'completed' AND created_at BETWEEN ?1 AND ?2
         GROUP BY d ORDER BY d DESC",
    )?;
    let rows = stmt
        .query_map(params![from_ms, to_ms], |r| {
            Ok(DailySalesReport {
                date: r.get("d")?,
                total_transactions: r.get("trx")?,
                total_revenue: r.get("revenue")?,
                total_discount: r.get("discount")?,
                total_tax: r.get("tax")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[tauri::command]
pub fn top_selling_products(
    state: State<AppState>,
    from_ms: i64,
    to_ms: i64,
    limit: i64,
) -> AppResult<Vec<TopSellingProduct>> {
    state.require_session()?;
    let conn = state.pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT si.item_id AS item_id, si.nama_item AS nama_item,
                SUM(si.qty) AS qty, SUM(si.subtotal) AS revenue
         FROM sale_items si
         JOIN sales s ON s.id = si.sale_id
         WHERE s.status = 'completed' AND s.created_at BETWEEN ?1 AND ?2
         GROUP BY si.item_id, si.nama_item
         ORDER BY qty DESC LIMIT ?3",
    )?;
    let rows = stmt
        .query_map(params![from_ms, to_ms, limit], |r| {
            Ok(TopSellingProduct {
                item_id: r.get("item_id")?,
                nama_item: r.get("nama_item")?,
                total_qty: r.get("qty")?,
                total_revenue: r.get("revenue")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[tauri::command]
pub fn stock_report(state: State<AppState>) -> AppResult<Vec<StockReportRow>> {
    state.require_session()?;
    let conn = state.pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, kode_item, nama_item, satuan_dasar, stok, harga_jual
         FROM items WHERE deleted_at IS NULL ORDER BY nama_item",
    )?;
    let rows = stmt
        .query_map([], |r| {
            let stok: f64 = r.get("stok")?;
            let harga: f64 = r.get("harga_jual")?;
            Ok(StockReportRow {
                item_id: r.get("id")?,
                kode_item: r.get("kode_item")?,
                nama_item: r.get("nama_item")?,
                satuan_dasar: r.get("satuan_dasar")?,
                stok,
                harga_jual: harga,
                nilai_stok: stok * harga,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
