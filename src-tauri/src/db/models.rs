use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::error::AppResult;

/// Current epoch milliseconds (the canonical timestamp unit across the app).
pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Generate a time-ordered UUID v7 string (better index locality than v4).
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Store {
    pub id: String,
    pub name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub tax_percent: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Store {
    pub fn from_row(r: &Row) -> AppResult<Self> {
        Ok(Self {
            id: r.get("id")?,
            name: r.get("name")?,
            address: r.get("address")?,
            phone: r.get("phone")?,
            tax_percent: r.get("tax_percent")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub store_id: String,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl User {
    pub fn from_row(r: &Row) -> AppResult<Self> {
        Ok(Self {
            id: r.get("id")?,
            store_id: r.get("store_id")?,
            username: r.get("username")?,
            full_name: r.get("full_name")?,
            role: r.get("role")?,
            is_active: r.get::<_, i64>("is_active")? != 0,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub user: User,
    pub token: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub store_id: String,
    pub kode_item: Option<String>,
    pub barcode: Option<String>,
    pub nama_item: String,
    pub jenis: Option<String>,
    pub merek: Option<String>,
    pub satuan_dasar: Option<String>,
    pub harga_beli: f64,
    pub harga_jual: f64,
    pub diskon_persen: f64,
    pub stok: f64,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub sync_status: String,
}

impl Item {
    pub fn from_row(r: &Row) -> AppResult<Self> {
        Ok(Self {
            id: r.get("id")?,
            store_id: r.get("store_id")?,
            kode_item: r.get("kode_item")?,
            barcode: r.get("barcode")?,
            nama_item: r.get("nama_item")?,
            jenis: r.get("jenis")?,
            merek: r.get("merek")?,
            satuan_dasar: r.get("satuan_dasar")?,
            harga_beli: r.get("harga_beli")?,
            harga_jual: r.get("harga_jual")?,
            diskon_persen: r.get("diskon_persen")?,
            stok: r.get("stok")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
            deleted_at: r.get("deleted_at")?,
            sync_status: r.get("sync_status")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemInput {
    pub kode_item: Option<String>,
    pub barcode: Option<String>,
    pub nama_item: String,
    pub jenis: Option<String>,
    pub merek: Option<String>,
    pub satuan_dasar: Option<String>,
    pub harga_beli: f64,
    pub harga_jual: f64,
    pub diskon_persen: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemQuery {
    pub search: Option<String>,
    pub jenis: Option<String>,
    pub merek: Option<String>,
    pub limit: i64,
    pub cursor_name: Option<String>,
    pub cursor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemPage {
    pub items: Vec<Item>,
    pub next_cursor_name: Option<String>,
    pub next_cursor_id: Option<String>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockTransaction {
    pub id: String,
    pub store_id: String,
    pub item_id: String,
    #[serde(rename = "type")]
    pub tx_type: String,
    pub qty: f64,
    pub qty_before: f64,
    pub qty_after: f64,
    pub ref_doc: Option<String>,
    pub note: Option<String>,
    pub user_id: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub sync_status: String,
}

impl StockTransaction {
    pub fn from_row(r: &Row) -> AppResult<Self> {
        Ok(Self {
            id: r.get("id")?,
            store_id: r.get("store_id")?,
            item_id: r.get("item_id")?,
            tx_type: r.get("type")?,
            qty: r.get("qty")?,
            qty_before: r.get("qty_before")?,
            qty_after: r.get("qty_after")?,
            ref_doc: r.get("ref_doc")?,
            note: r.get("note")?,
            user_id: r.get("user_id")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
            sync_status: r.get("sync_status")?,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StockMovementInput {
    pub item_id: String,
    #[serde(rename = "type")]
    pub tx_type: String,
    pub qty: f64,
    pub ref_doc: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sale {
    pub id: String,
    pub store_id: String,
    pub invoice_no: Option<String>,
    pub user_id: String,
    pub shift_id: Option<String>,
    pub subtotal: f64,
    pub diskon: f64,
    pub pajak: f64,
    pub total: f64,
    pub bayar: f64,
    pub kembali: f64,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub sync_status: String,
}

impl Sale {
    pub fn from_row(r: &Row) -> AppResult<Self> {
        Ok(Self {
            id: r.get("id")?,
            store_id: r.get("store_id")?,
            invoice_no: r.get("invoice_no")?,
            user_id: r.get("user_id")?,
            shift_id: r.get("shift_id")?,
            subtotal: r.get("subtotal")?,
            diskon: r.get("diskon")?,
            pajak: r.get("pajak")?,
            total: r.get("total")?,
            bayar: r.get("bayar")?,
            kembali: r.get("kembali")?,
            status: r.get("status")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
            deleted_at: r.get("deleted_at")?,
            sync_status: r.get("sync_status")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleItem {
    pub id: String,
    pub sale_id: String,
    pub item_id: String,
    pub nama_item: String,
    pub qty: f64,
    pub harga: f64,
    pub diskon: f64,
    pub subtotal: f64,
}

impl SaleItem {
    pub fn from_row(r: &Row) -> AppResult<Self> {
        Ok(Self {
            id: r.get("id")?,
            sale_id: r.get("sale_id")?,
            item_id: r.get("item_id")?,
            nama_item: r.get("nama_item")?,
            qty: r.get("qty")?,
            harga: r.get("harga")?,
            diskon: r.get("diskon")?,
            subtotal: r.get("subtotal")?,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaleItemInput {
    pub item_id: String,
    pub nama_item: String,
    pub qty: f64,
    pub harga: f64,
    pub diskon: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaleInput {
    pub shift_id: Option<String>,
    pub status: String,
    pub diskon: f64,
    pub pajak_persen: f64,
    pub bayar: f64,
    pub items: Vec<SaleItemInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaleWithItems {
    pub sale: Sale,
    pub items: Vec<SaleItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shift {
    pub id: String,
    pub store_id: String,
    pub user_id: String,
    pub opening_cash: f64,
    pub closing_cash: Option<f64>,
    pub expected_cash: Option<f64>,
    pub total_sales: f64,
    pub opened_at: i64,
    pub closed_at: Option<i64>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub sync_status: String,
}

impl Shift {
    pub fn from_row(r: &Row) -> AppResult<Self> {
        Ok(Self {
            id: r.get("id")?,
            store_id: r.get("store_id")?,
            user_id: r.get("user_id")?,
            opening_cash: r.get("opening_cash")?,
            closing_cash: r.get("closing_cash")?,
            expected_cash: r.get("expected_cash")?,
            total_sales: r.get("total_sales")?,
            opened_at: r.get("opened_at")?,
            closed_at: r.get("closed_at")?,
            status: r.get("status")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
            sync_status: r.get("sync_status")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditLog {
    pub id: String,
    pub store_id: String,
    pub user_id: String,
    pub action: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub detail: Option<String>,
    pub created_at: i64,
}

impl AuditLog {
    pub fn from_row(r: &Row) -> AppResult<Self> {
        Ok(Self {
            id: r.get("id")?,
            store_id: r.get("store_id")?,
            user_id: r.get("user_id")?,
            action: r.get("action")?,
            entity_type: r.get("entity_type")?,
            entity_id: r.get("entity_id")?,
            detail: r.get("detail")?,
            created_at: r.get("created_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncConflict {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub local_payload: String,
    pub server_payload: String,
    pub conflict_field: Option<String>,
    pub resolution: String,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<i64>,
    pub created_at: i64,
}

impl SyncConflict {
    pub fn from_row(r: &Row) -> AppResult<Self> {
        Ok(Self {
            id: r.get("id")?,
            entity_type: r.get("entity_type")?,
            entity_id: r.get("entity_id")?,
            local_payload: r.get("local_payload")?,
            server_payload: r.get("server_payload")?,
            conflict_field: r.get("conflict_field")?,
            resolution: r.get("resolution")?,
            resolved_by: r.get("resolved_by")?,
            resolved_at: r.get("resolved_at")?,
            created_at: r.get("created_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncStatusInfo {
    pub state: String,
    pub pending_count: i64,
    pub conflict_count: i64,
    pub last_sync_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailySalesReport {
    pub date: String,
    pub total_transactions: i64,
    pub total_revenue: f64,
    pub total_discount: f64,
    pub total_tax: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopSellingProduct {
    pub item_id: String,
    pub nama_item: String,
    pub total_qty: f64,
    pub total_revenue: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StockReportRow {
    pub item_id: String,
    pub kode_item: Option<String>,
    pub nama_item: String,
    pub satuan_dasar: Option<String>,
    pub stok: f64,
    pub harga_jual: f64,
    pub nilai_stok: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterSettings {
    pub printer_name: String,
    pub paper_width: i64,
    pub header_text: String,
    pub footer_text: String,
}
