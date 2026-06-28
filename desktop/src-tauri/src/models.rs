use serde::{Deserialize, Serialize};

/// Master data barang. Di-mirror dari server, plus metadata sync lokal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub barcode: Option<String>,
    pub category: Option<String>,
    pub brand: Option<String>,
    pub unit: Option<String>,
    pub sell_price: f64,
    pub cost_price: f64,
    pub default_discount: f64,
    pub is_active: bool,
    #[serde(default)]
    pub is_deleted: bool,
    /// ISO-8601 UTC, dipakai untuk Delta Sync & Last Write Wins.
    pub updated_at: String,
}

/// Produk + info stok (untuk tampilan kasir/persediaan). Stok TIDAK di-sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductWithStock {
    #[serde(flatten)]
    pub product: Product,
    pub stock_qty: f64,
}

/// Payload satu item dalam transaksi penjualan dari frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleItemInput {
    pub product_id: String,
    pub name: String,
    pub price: f64,
    pub qty: f64,
    #[serde(default)]
    pub discount: f64,
}

/// Payload transaksi penjualan dari frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleInput {
    pub cashier_id: String,
    pub payment_method: String, // Tunai | QRIS | Transfer | Kartu
    pub paid: f64,
    pub items: Vec<SaleItemInput>,
}

/// Ringkasan transaksi tersimpan (untuk struk & history).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub invoice_no: String,
    pub cashier_id: String,
    pub subtotal: f64,
    pub discount: f64,
    pub total: f64,
    pub paid: f64,
    pub change: f64,
    pub payment_method: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionItem {
    pub product_id: String,
    pub name: String,
    pub price: f64,
    pub qty: f64,
    pub discount: f64,
    pub line_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDetail {
    #[serde(flatten)]
    pub header: Transaction,
    pub items: Vec<TransactionItem>,
}

/// Input upsert barang dari UI manajemen barang.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductInput {
    pub id: Option<String>,
    pub name: String,
    pub barcode: Option<String>,
    pub category: Option<String>,
    pub brand: Option<String>,
    pub unit: Option<String>,
    pub sell_price: f64,
    pub cost_price: f64,
    #[serde(default)]
    pub default_discount: f64,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

/// Hasil ringkas operasi sinkronisasi yang dikembalikan ke UI.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncResult {
    pub pushed: i64,
    pub pulled: i64,
    pub skipped: i64,
    pub message: String,
}

// ---------- Pengguna / hak akses ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub name: String,
    pub role: String,
    /// Daftar modul yang boleh diakses, mis. ["master","penjualan",...].
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInput {
    pub id: Option<String>,
    pub username: String,
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Kosong saat edit = PIN tidak diubah.
    pub pin: Option<String>,
}

// ---------- Pergerakan stok (Item Masuk / Keluar / Opname) ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovement {
    pub id: i64,
    pub product_id: String,
    pub product_name: String,
    pub kind: String, // in | out | opname | sale
    pub qty: f64,
    pub note: Option<String>,
    pub user_id: Option<String>,
    pub created_at: String,
    pub stock_after: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovementInput {
    pub product_id: String,
    pub kind: String, // in | out | opname
    pub qty: f64,
    pub note: Option<String>,
    pub user_id: Option<String>,
}

// ---------- Diskon periodik ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountPeriod {
    pub id: String,
    pub scope: String,            // item | brand
    pub target: String,           // product_id atau nama brand
    pub target_label: Option<String>,
    pub discount_type: String,    // amount | percent
    pub value: f64,
    pub days: String,             // "everyday" atau csv "mon,tue,..."
    pub is_active: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountPeriodInput {
    pub id: Option<String>,
    pub scope: String,
    pub target: String,
    pub target_label: Option<String>,
    pub discount_type: String,
    pub value: f64,
    pub days: String,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

// ---------- Merek (brand) ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brand {
    pub id: String,
    pub name: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandInput {
    pub id: Option<String>,
    pub name: String,
}
