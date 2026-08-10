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

/// Satu halaman hasil `list_products_page`, plus total baris cocok (untuk
/// kontrol paginasi di UI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductPage {
    pub items: Vec<ProductWithStock>,
    pub total: i64,
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
    pub payment_method: String, // Tunai | QRIS | Kombinasi | Kartu
    pub paid: f64,
    pub items: Vec<SaleItemInput>,
    #[serde(default)]
    pub customer_id: Option<String>,
    #[serde(default)]
    pub shift_id: Option<String>,
    /// Override waktu transaksi (ISO-8601), dipakai admin untuk mencatat
    /// transaksi mundur (tanggal kemarin dsb). Kosong = pakai waktu sekarang.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Hanya diisi kalau payment_method = "Kombinasi" — pecahan bayar tunai/QRIS.
    #[serde(default)]
    pub paid_cash: Option<f64>,
    #[serde(default)]
    pub paid_qris: Option<f64>,
    /// Kunci idempoten dari klien: **satu percobaan checkout = satu nilai**, dan
    /// nilainya TETAP SAMA saat kasir mencoba ulang. Gunanya untuk jalur yang
    /// jawabannya bisa hilang di tengah jalan (Server Pusat lewat wifi/internet):
    /// kalau transaksi sudah tersimpan tapi jawaban tidak sampai ke kasir,
    /// percobaan kedua mengembalikan transaksi yang SUDAH ada alih-alih membuat
    /// yang kedua. Kosong (app HP / klien lama) = perilaku persis seperti dulu.
    #[serde(default)]
    pub client_ref: Option<String>,
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
    #[serde(default)]
    pub customer_id: Option<String>,
    #[serde(default)]
    pub shift_id: Option<String>,
    #[serde(default)]
    pub paid_cash: Option<f64>,
    #[serde(default)]
    pub paid_qris: Option<f64>,
}

/// Halaman hasil list_transactions dengan total untuk pagination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionPage {
    pub items: Vec<Transaction>,
    pub total: i64,
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
fn default_one() -> i64 {
    1
}

/// Satu baris log sinkronisasi (produk apa, diapakan) — ditampilkan di UI
/// Sync Center supaya kelihatan barang mana yang berubah, bukan cuma angka.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncLogEntry {
    pub id: String,
    pub name: String,
    /// "Diterapkan" | "Dilewati" (pull) | "Dikirim" (push — server tidak
    /// mengembalikan breakdown per-ID, jadi push cuma bisa melaporkan apa
    /// yang DIKIRIM, bukan status per-item hasil server).
    pub action: String,
}

/// Hasil ringkas operasi sinkronisasi yang dikembalikan ke UI.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncResult {
    pub pushed: i64,
    pub pulled: i64,
    pub skipped: i64,
    pub message: String,
    #[serde(default)]
    pub log: Vec<SyncLogEntry>,
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
    /// Barcode barang (ikut ditampilkan di bawah nama pada riwayat opname).
    #[serde(default)]
    pub barcode: Option<String>,
    pub kind: String, // in | out | opname | sale
    pub qty: f64,
    pub note: Option<String>,
    pub user_id: Option<String>,
    pub created_at: String,
    /// Stok "buku" sesaat sebelum mutasi ini dicatat. `None` hanya untuk baris
    /// lama yang tidak bisa direkonstruksi (barang belum punya mutasi sebelumnya).
    #[serde(default)]
    pub stock_before: Option<f64>,
    pub stock_after: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovementInput {
    pub product_id: String,
    pub kind: String, // in | out | opname
    pub qty: f64,
    pub note: Option<String>,
    pub user_id: Option<String>,
    /// Override waktu pergerakan (ISO-8601), dipakai admin untuk mencatat
    /// mundur (tanggal kemarin dsb). Kosong = pakai waktu sekarang.
    #[serde(default)]
    pub created_at: Option<String>,
}

// ---------- Time Opname (opname yang dicatat mundur ke titik waktu tertentu) ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeOpnameItemInput {
    pub product_id: String,
    /// Hasil hitung fisik pada `created_at` — angka mutlak, bukan selisih.
    pub qty: f64,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeOpnameInput {
    /// Waktu barang itu benar-benar dihitung (RFC3339). Wajib — beda dari menu
    /// opname lain yang boleh mengosongkannya dan jatuh ke waktu sekarang.
    pub created_at: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    pub items: Vec<TimeOpnameItemInput>,
}

/// Satu barang dalam pratinjau/hasil time opname — cukup untuk menjelaskan ke
/// pemakai apa yang berubah, di titik waktu itu maupun pada stok hari ini.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeOpnameRow {
    pub product_id: String,
    pub product_name: String,
    pub barcode: Option<String>,
    /// Stok buku tepat sebelum titik waktu opname.
    pub book: f64,
    pub counted: f64,
    /// `counted - book` — selisih pada titik waktu itu, bukan terhadap stok hari ini.
    pub diff: f64,
    pub stock_now_before: f64,
    /// Stok setelah semua mutasi sesudah titik waktu itu diputar ulang.
    pub stock_now_after: f64,
    /// Ada opname lain SETELAH titik waktu ini, jadi stok hari ini ditentukan
    /// opname yang lebih baru itu — hitungan di sini tidak menggesernya.
    pub overridden_by_later_opname: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeOpnameResult {
    pub created_at: String,
    pub rows: Vec<TimeOpnameRow>,
}

// ---------- Batch Item Masuk / Keluar ----------

/// Satu baris barang di dalam batch (input saat simpan/edit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovementBatchItemInput {
    pub product_id: String,
    pub qty: f64,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovementBatchInput {
    pub kind: String, // in | out
    pub note: Option<String>,
    pub user_id: Option<String>,
    pub items: Vec<StockMovementBatchItemInput>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Barang dalam batch, versi tersimpan (ikut nama barang untuk ditampilkan).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovementBatchItem {
    pub product_id: String,
    pub product_name: String,
    pub qty: f64,
    pub note: Option<String>,
}

/// Ringkasan batch untuk daftar (Daftar Item Masuk/Keluar) — agregat, bukan per barang.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovementBatch {
    pub id: String,
    pub no: String,
    pub kind: String,
    pub note: Option<String>,
    pub user_id: Option<String>,
    pub created_at: String,
    pub item_count: i64,
    pub total_qty: f64,
}

/// Halaman hasil list_stock_movement_batches dengan total untuk pagination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovementBatchPage {
    pub items: Vec<StockMovementBatch>,
    pub total: i64,
}

/// Detail batch lengkap dengan seluruh barangnya, untuk lihat/edit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovementBatchDetail {
    pub id: String,
    pub no: String,
    pub kind: String,
    pub note: Option<String>,
    pub user_id: Option<String>,
    pub created_at: String,
    pub items: Vec<StockMovementBatchItem>,
}

// ---------- Opname Spesial (satu merek disisir habis) ----------

/// Satu barang yang benar-benar dihitung fisiknya saat Opname Spesial.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpnameSpecialItemInput {
    pub product_id: String,
    /// Hasil hitung fisik (stok absolut, bukan selisih).
    pub qty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpnameSpecialInput {
    pub brand: String,
    pub note: Option<String>,
    pub user_id: Option<String>,
    pub items: Vec<OpnameSpecialItemInput>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Ringkasan hasil Opname Spesial untuk ditampilkan ke kasir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpnameSpecialResult {
    pub brand: String,
    /// Jumlah barang yang dihitung (ada di tabel opname).
    pub counted: i64,
    /// Jumlah barang merek ini yang stoknya dinolkan karena tidak dihitung.
    pub zeroed: i64,
    pub created_at: String,
}

// ---------- Diskon periodik ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountPeriod {
    pub id: String,
    #[serde(default)]
    pub code: String,             // kode grup diskon, mis. "PROMO-LEBARAN"
    pub scope: String,            // item | brand
    pub target: String,           // product_id atau nama brand
    pub target_label: Option<String>,
    pub discount_type: String,    // amount | percent
    pub value: f64,
    pub days: String,             // "everyday" atau csv hari
    pub is_active: bool,
    #[serde(default = "default_one")]
    pub priority: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountPeriodInput {
    pub id: Option<String>,
    #[serde(default)]
    pub code: String,
    pub scope: String,
    pub target: String,
    pub target_label: Option<String>,
    pub discount_type: String,
    pub value: f64,
    pub days: String,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default = "default_one")]
    pub priority: i64,
}

// ---------- Barang duplikat (dedupe barcode) ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupeDetail {
    pub barcode: String,
    pub kept_name: String,
    pub removed_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DedupeResult {
    pub groups: i64,
    pub removed: i64,
    pub details: Vec<DedupeDetail>,
}

// ---------- Pelanggan ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub note: Option<String>,
    pub is_active: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerInput {
    pub id: Option<String>,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub note: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

// ---------- Pengeluaran (Kas Keluar) ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expense {
    pub id: String,
    pub date: String, // YYYY-MM-DD
    pub category: String,
    pub amount: f64,
    pub note: Option<String>,
    pub user_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpenseInput {
    pub id: Option<String>,
    pub date: String,
    pub category: String,
    pub amount: f64,
    pub note: Option<String>,
    pub user_id: Option<String>,
}

// ---------- Shift kasir (buka/tutup, rekonsiliasi) ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shift {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub opening_cash: f64,
    pub closing_cash: Option<f64>,
    pub expected_cash: Option<f64>,
    pub difference: Option<f64>,
    pub note: Option<String>,
    pub opened_at: String,
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenShiftInput {
    pub user_id: String,
    pub user_name: String,
    pub opening_cash: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseShiftInput {
    pub id: String,
    pub closing_cash: f64,
    pub note: Option<String>,
}

/// Ringkasan rekonsiliasi shift: total transaksi + tunai selama shift, dipakai
/// untuk hitung `expected_cash` sebelum shift ditutup.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShiftSummary {
    pub transaction_count: i64,
    pub total_sales: f64,
    pub cash_sales: f64,
    pub non_cash_sales: f64,
}

// ---------- Laporan per barang / per merek ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSalesRow {
    pub product_id: String,
    pub name: String,
    pub brand: Option<String>,
    pub qty: f64,
    pub gross: f64,
    pub discount: f64,
    pub net: f64,
    pub cogs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandSalesRow {
    pub brand: String,
    pub qty: f64,
    pub gross: f64,
    pub discount: f64,
    pub net: f64,
}

/// Satu baris item terjual (Laporan Item Detail) — satu baris per transaction_item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesItemDetailRow {
    pub invoice_no: String,
    pub created_at: String,
    pub cashier_id: String,
    pub product_id: String,
    pub name: String,
    pub barcode: Option<String>,
    pub brand: Option<String>,
    pub qty: f64,
    pub price: f64,
    pub discount: f64,
    pub net: f64,
}

/// Rekap penjualan item per hari (Laporan Item Per Hari).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySalesRow {
    pub day: String,
    pub qty: f64,
    pub gross: f64,
    pub discount: f64,
    pub net: f64,
}

// ---------- Alur Barang (buku besar stok) ----------

/// Satu baris rekap alur stok per barang dalam satu rentang tanggal.
/// `adjustment` = selisih yang tidak dijelaskan masuk/keluar/terjual — praktisnya
/// hasil koreksi opname (opname menyetel stok absolut, bukan delta).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockFlowRow {
    pub product_id: String,
    pub name: String,
    pub barcode: Option<String>,
    pub brand: Option<String>,
    pub opening: f64,
    pub masuk: f64,
    pub keluar: f64,
    pub terjual: f64,
    pub adjustment: f64,
    pub closing: f64,
    pub current_stock: f64,
}

/// Buku besar stok satu barang: header barang + stok awal + semua mutasi
/// (masuk / keluar / terjual / opname) urut kronologis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockFlowDetail {
    pub product_id: String,
    pub name: String,
    pub barcode: Option<String>,
    pub brand: Option<String>,
    pub unit: Option<String>,
    pub current_stock: f64,
    pub opening: f64,
    pub rows: Vec<StockMovement>,
}

// ---------- Multi-database (toko) ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreInfo {
    pub id: String,
    pub name: String,
    pub file: String,
    pub created_at: String,
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

// ---------- Perangkat mobile (HP kasir) ----------

/// Satu HP yang sudah mendaftar ke Server Pusat ini. Token mentahnya hanya
/// pernah ada sekali (saat `/pair`) dan tidak pernah disimpan — yang tersimpan
/// hash-nya, jadi daftar ini aman ditampilkan di layar Pengaturan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileDevice {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub last_seen_at: Option<String>,
    pub revoked: bool,
}

/// Balasan `POST /pair` — satu-satunya saat token mentah dikirim ke HP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairResult {
    pub device_id: String,
    pub device_token: String,
    pub store_name: String,
}
