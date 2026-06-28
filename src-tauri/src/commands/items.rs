use rusqlite::params;
use serde::Deserialize;
use tauri::State;

use crate::db::models::{new_id, now_ms, Item, ItemInput, ItemPage, ItemQuery};
use crate::db::{enqueue, write_audit};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

fn item_payload(item: &Item) -> serde_json::Value {
    serde_json::to_value(item).unwrap_or(serde_json::Value::Null)
}

fn load_item(conn: &rusqlite::Connection, id: &str) -> AppResult<Item> {
    conn.query_row(
        "SELECT * FROM items WHERE id = ?1",
        params![id],
        |r| Ok(Item::from_row(r)),
    )
    .map_err(|_| AppError::NotFound("Item tidak ditemukan".into()))?
}

#[tauri::command]
pub fn list_items(state: State<AppState>, query: ItemQuery) -> AppResult<ItemPage> {
    state.require_session()?;
    let conn = state.pool.get()?;

    let mut where_clauses = vec!["deleted_at IS NULL".to_string()];
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(search) = query.search.as_ref().filter(|s| !s.is_empty()) {
        where_clauses
            .push("(nama_item LIKE ?  OR barcode LIKE ? OR kode_item LIKE ?)".to_string());
        let pattern = format!("%{search}%");
        args.push(Box::new(pattern.clone()));
        args.push(Box::new(pattern.clone()));
        args.push(Box::new(pattern));
    }
    if let Some(jenis) = query.jenis.as_ref().filter(|s| !s.is_empty()) {
        where_clauses.push("jenis = ?".to_string());
        args.push(Box::new(jenis.clone()));
    }
    if let Some(merek) = query.merek.as_ref().filter(|s| !s.is_empty()) {
        where_clauses.push("merek = ?".to_string());
        args.push(Box::new(merek.clone()));
    }

    let base_where = where_clauses.join(" AND ");

    // Total count (without keyset cursor) for the header.
    let total: i64 = {
        let sql = format!("SELECT COUNT(*) FROM items WHERE {base_where}");
        let params_ref: Vec<&dyn rusqlite::ToSql> =
            args.iter().map(|b| b.as_ref()).collect();
        conn.query_row(&sql, params_ref.as_slice(), |r| r.get(0))?
    };

    // Keyset pagination on (nama_item, id).
    let mut page_where = base_where.clone();
    if let (Some(cn), Some(ci)) = (&query.cursor_name, &query.cursor_id) {
        page_where.push_str(" AND (nama_item > ? OR (nama_item = ? AND id > ?))");
        args.push(Box::new(cn.clone()));
        args.push(Box::new(cn.clone()));
        args.push(Box::new(ci.clone()));
    }

    let sql = format!(
        "SELECT * FROM items WHERE {page_where}
         ORDER BY nama_item ASC, id ASC LIMIT {}",
        query.limit
    );
    let params_ref: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(params_ref.as_slice(), |r| Ok(Item::from_row(r)))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect::<AppResult<Vec<_>>>()?;

    let (next_name, next_id) = if items.len() as i64 == query.limit {
        items
            .last()
            .map(|i| (Some(i.nama_item.clone()), Some(i.id.clone())))
            .unwrap_or((None, None))
    } else {
        (None, None)
    };

    Ok(ItemPage {
        items,
        next_cursor_name: next_name,
        next_cursor_id: next_id,
        total,
    })
}

#[tauri::command]
pub fn get_item(state: State<AppState>, id: String) -> AppResult<Item> {
    state.require_session()?;
    let conn = state.pool.get()?;
    load_item(&conn, &id)
}

#[tauri::command]
pub fn find_item_by_barcode(
    state: State<AppState>,
    barcode: String,
) -> AppResult<Option<Item>> {
    state.require_session()?;
    let conn = state.pool.get()?;
    let item = conn
        .query_row(
            "SELECT * FROM items WHERE barcode = ?1 AND deleted_at IS NULL",
            params![barcode],
            |r| Ok(Item::from_row(r)),
        )
        .ok()
        .transpose()?;
    Ok(item)
}

#[tauri::command]
pub fn create_item(state: State<AppState>, input: ItemInput) -> AppResult<Item> {
    let session = state.require_role(&["admin", "supervisor"])?;
    if input.nama_item.trim().is_empty() {
        return Err(AppError::Validation("Nama item wajib diisi".into()));
    }
    let mut conn = state.pool.get()?;
    let tx = conn.transaction()?;
    let id = new_id();
    let ts = now_ms();
    tx.execute(
        "INSERT INTO items
            (id, store_id, kode_item, barcode, nama_item, jenis, merek, satuan_dasar,
             harga_beli, harga_jual, diskon_persen, stok, created_at, updated_at, sync_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, ?12, ?12, 'pending')",
        params![
            id,
            state.store_id,
            input.kode_item,
            input.barcode,
            input.nama_item,
            input.jenis,
            input.merek,
            input.satuan_dasar,
            input.harga_beli,
            input.harga_jual,
            input.diskon_persen,
            ts,
        ],
    )?;
    let item = load_item(&tx, &id)?;
    enqueue(&tx, "item", &id, "insert", &item_payload(&item), None)?;
    write_audit(&tx, &state.store_id, &session.user.id, "create_item", Some("item"), Some(&id), None)?;
    tx.commit()?;
    Ok(item)
}

#[tauri::command]
pub fn update_item(
    state: State<AppState>,
    id: String,
    input: ItemInput,
) -> AppResult<Item> {
    let session = state.require_role(&["admin", "supervisor"])?;
    let mut conn = state.pool.get()?;
    let tx = conn.transaction()?;
    let prev = load_item(&tx, &id)?;
    let ts = now_ms();
    tx.execute(
        "UPDATE items SET kode_item=?2, barcode=?3, nama_item=?4, jenis=?5, merek=?6,
             satuan_dasar=?7, harga_beli=?8, harga_jual=?9, diskon_persen=?10,
             updated_at=?11, sync_status='pending'
         WHERE id=?1",
        params![
            id,
            input.kode_item,
            input.barcode,
            input.nama_item,
            input.jenis,
            input.merek,
            input.satuan_dasar,
            input.harga_beli,
            input.harga_jual,
            input.diskon_persen,
            ts,
        ],
    )?;
    let item = load_item(&tx, &id)?;
    enqueue(&tx, "item", &id, "update", &item_payload(&item), Some(prev.updated_at))?;
    write_audit(&tx, &state.store_id, &session.user.id, "update_item", Some("item"), Some(&id), None)?;
    tx.commit()?;
    Ok(item)
}

#[tauri::command]
pub fn delete_item(state: State<AppState>, id: String) -> AppResult<()> {
    let session = state.require_role(&["admin", "supervisor"])?;
    let mut conn = state.pool.get()?;
    let tx = conn.transaction()?;
    let prev = load_item(&tx, &id)?;
    let ts = now_ms();
    tx.execute(
        "UPDATE items SET deleted_at=?2, updated_at=?2, sync_status='pending' WHERE id=?1",
        params![id, ts],
    )?;
    let item = load_item(&tx, &id)?;
    enqueue(&tx, "item", &id, "delete", &item_payload(&item), Some(prev.updated_at))?;
    write_audit(&tx, &state.store_id, &session.user.id, "delete_item", Some("item"), Some(&id), None)?;
    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn duplicate_item(state: State<AppState>, id: String) -> AppResult<Item> {
    let session = state.require_role(&["admin", "supervisor"])?;
    let mut conn = state.pool.get()?;
    let tx = conn.transaction()?;
    let src = load_item(&tx, &id)?;
    let new = new_id();
    let ts = now_ms();
    tx.execute(
        "INSERT INTO items
            (id, store_id, kode_item, barcode, nama_item, jenis, merek, satuan_dasar,
             harga_beli, harga_jual, diskon_persen, stok, created_at, updated_at, sync_status)
         VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11, ?11, 'pending')",
        params![
            new,
            state.store_id,
            src.kode_item.map(|k| format!("{k}-COPY")),
            format!("{} (Copy)", src.nama_item),
            src.jenis,
            src.merek,
            src.satuan_dasar,
            src.harga_beli,
            src.harga_jual,
            src.diskon_persen,
            ts,
        ],
    )?;
    let item = load_item(&tx, &new)?;
    enqueue(&tx, "item", &new, "insert", &item_payload(&item), None)?;
    write_audit(&tx, &state.store_id, &session.user.id, "duplicate_item", Some("item"), Some(&new), None)?;
    tx.commit()?;
    Ok(item)
}

#[derive(Debug, serde::Serialize)]
pub struct BulkResult {
    pub inserted: i64,
    pub updated: i64,
    pub errors: Vec<String>,
}

#[tauri::command]
pub fn bulk_upsert_items(
    state: State<AppState>,
    items: Vec<ItemInput>,
) -> AppResult<BulkResult> {
    let session = state.require_role(&["admin", "supervisor"])?;
    let mut conn = state.pool.get()?;
    let tx = conn.transaction()?;
    let mut inserted = 0;
    let mut updated = 0;
    let mut errors = Vec::new();

    for (idx, input) in items.into_iter().enumerate() {
        if input.nama_item.trim().is_empty() {
            errors.push(format!("Baris {}: nama_item kosong", idx + 2));
            continue;
        }
        let ts = now_ms();
        let existing: Option<String> = match &input.barcode {
            Some(b) if !b.is_empty() => conn_existing(&tx, b)?,
            _ => None,
        };
        if let Some(existing_id) = existing {
            tx.execute(
                "UPDATE items SET kode_item=?2, nama_item=?3, jenis=?4, merek=?5,
                     satuan_dasar=?6, harga_beli=?7, harga_jual=?8, diskon_persen=?9,
                     updated_at=?10, sync_status='pending' WHERE id=?1",
                params![
                    existing_id,
                    input.kode_item,
                    input.nama_item,
                    input.jenis,
                    input.merek,
                    input.satuan_dasar,
                    input.harga_beli,
                    input.harga_jual,
                    input.diskon_persen,
                    ts,
                ],
            )?;
            let item = load_item(&tx, &existing_id)?;
            enqueue(&tx, "item", &existing_id, "update", &item_payload(&item), Some(item.updated_at))?;
            updated += 1;
        } else {
            let id = new_id();
            tx.execute(
                "INSERT INTO items
                    (id, store_id, kode_item, barcode, nama_item, jenis, merek, satuan_dasar,
                     harga_beli, harga_jual, diskon_persen, stok, created_at, updated_at, sync_status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, ?12, ?12, 'pending')",
                params![
                    id,
                    state.store_id,
                    input.kode_item,
                    input.barcode,
                    input.nama_item,
                    input.jenis,
                    input.merek,
                    input.satuan_dasar,
                    input.harga_beli,
                    input.harga_jual,
                    input.diskon_persen,
                    ts,
                ],
            )?;
            let item = load_item(&tx, &id)?;
            enqueue(&tx, "item", &id, "insert", &item_payload(&item), None)?;
            inserted += 1;
        }
    }
    write_audit(&tx, &state.store_id, &session.user.id, "bulk_upsert_items", Some("item"), None, None)?;
    tx.commit()?;
    Ok(BulkResult { inserted, updated, errors })
}

fn conn_existing(conn: &rusqlite::Connection, barcode: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM items WHERE barcode = ?1 AND deleted_at IS NULL",
            params![barcode],
            |r| r.get::<_, String>(0),
        )
        .ok())
}

#[derive(Debug, Deserialize)]
pub struct MassUpdateRow {
    pub id: String,
    pub harga_beli: Option<f64>,
    pub harga_jual: Option<f64>,
    pub diskon_persen: Option<f64>,
    pub jenis: Option<String>,
    pub merek: Option<String>,
}

#[tauri::command]
pub fn mass_update_items(
    state: State<AppState>,
    items: Vec<MassUpdateRow>,
) -> AppResult<i64> {
    let session = state.require_role(&["admin", "supervisor"])?;
    let mut conn = state.pool.get()?;
    let tx = conn.transaction()?;
    let mut count = 0;
    for row in items {
        let prev = load_item(&tx, &row.id)?;
        let ts = now_ms();
        tx.execute(
            "UPDATE items SET
                harga_beli = COALESCE(?2, harga_beli),
                harga_jual = COALESCE(?3, harga_jual),
                diskon_persen = COALESCE(?4, diskon_persen),
                jenis = COALESCE(?5, jenis),
                merek = COALESCE(?6, merek),
                updated_at = ?7, sync_status = 'pending'
             WHERE id = ?1",
            params![
                row.id,
                row.harga_beli,
                row.harga_jual,
                row.diskon_persen,
                row.jenis,
                row.merek,
                ts,
            ],
        )?;
        let item = load_item(&tx, &row.id)?;
        enqueue(&tx, "item", &row.id, "update", &item_payload(&item), Some(prev.updated_at))?;
        count += 1;
    }
    write_audit(&tx, &state.store_id, &session.user.id, "mass_update_items", Some("item"), None, None)?;
    tx.commit()?;
    Ok(count)
}
