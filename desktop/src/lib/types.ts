// Tipe sesuai struct serde di Rust (field snake_case agar cocok lewat IPC).

export interface Product {
  id: string;
  name: string;
  barcode: string | null;
  category: string | null;
  brand: string | null;
  unit: string | null;
  sell_price: number;
  cost_price: number;
  default_discount: number;
  is_active: boolean;
  is_deleted: boolean;
  updated_at: string;
}

export interface ProductWithStock extends Product {
  stock_qty: number;
}

export interface ProductPage {
  items: ProductWithStock[];
  total: number;
}

export interface ProductInput {
  id?: string | null;
  name: string;
  barcode?: string | null;
  category?: string | null;
  brand?: string | null;
  unit?: string | null;
  sell_price: number;
  cost_price: number;
  default_discount?: number;
  is_active?: boolean;
}

export interface SaleItemInput {
  product_id: string;
  name: string;
  price: number;
  qty: number;
  discount: number;
}

export interface SaleInput {
  cashier_id: string;
  payment_method: string;
  paid: number;
  items: SaleItemInput[];
  customer_id?: string | null;
  shift_id?: string | null;
  created_at?: string | null;
  /** Hanya diisi kalau payment_method = "Kombinasi". */
  paid_cash?: number | null;
  paid_qris?: number | null;
  /**
   * Kunci idempoten satu percobaan checkout — TETAP SAMA saat dicoba ulang.
   * Kalau transaksi sudah tersimpan di Server Pusat tapi jawabannya hilang di
   * jaringan, percobaan kedua mengembalikan transaksi yang sudah ada, bukan
   * mencatat yang kedua.
   */
  client_ref?: string | null;
}

export interface Transaction {
  id: string;
  invoice_no: string;
  cashier_id: string;
  subtotal: number;
  discount: number;
  total: number;
  paid: number;
  change: number;
  payment_method: string;
  created_at: string;
  customer_id: string | null;
  shift_id: string | null;
  paid_cash?: number | null;
  paid_qris?: number | null;
}

export interface TransactionItem {
  product_id: string;
  name: string;
  price: number;
  qty: number;
  discount: number;
  line_total: number;
}

export interface TransactionDetail extends Transaction {
  items: TransactionItem[];
}

export interface TransactionPage {
  items: Transaction[];
  total: number;
}

export interface SyncLogEntry {
  id: string;
  name: string;
  action: string;
}

export interface SyncResult {
  pushed: number;
  pulled: number;
  skipped: number;
  message: string;
  log: SyncLogEntry[];
}

export type PaymentMethod = "Tunai" | "QRIS" | "Kombinasi" | "Kartu";

export interface Brand {
  id: string;
  name: string;
  updated_at: string;
}

export interface BrandInput {
  id?: string | null;
  name: string;
}

export interface StoreInfo {
  id: string;
  name: string;
  file: string;
  created_at: string;
}

/// Jalur ke Server Pusat. Kasir memilih sendiri tiap membuka app — tidak ada
/// auto-fallback (lihat servers.rs).
export type ServerPath = "lan" | "online";

/// Satu Server Pusat tersimpan. Satu entry bisa menyimpan DUA alamat: wifi
/// (lan_host:lan_port) dan internet (relay_url + store_id lewat relay).
export interface ServerInfo {
  id: string;
  kind: "local" | "remote";
  name: string;
  lan_host: string | null;
  lan_port: number | null;
  relay_url: string | null;
  store_id: string | null;
  device_token: string | null;
}

export interface ActiveServer {
  server: ServerInfo;
  path: ServerPath;
}

/// Isian layar "+ Tambah Server" / "Uji Koneksi".
export interface ServerInput {
  name: string;
  lan_host: string;
  lan_port: number | null;
  relay_url: string;
  store_id: string;
  code: string;
  path: ServerPath;
}

export const hasLanPath = (s: ServerInfo | null | undefined): boolean =>
  !!s?.lan_host?.trim();
export const hasOnlinePath = (s: ServerInfo | null | undefined): boolean =>
  !!s?.relay_url?.trim() && !!s?.store_id?.trim();

export interface LanServerStatus {
  enabled: boolean;
  port: number;
  token: string;
  local_ip: string | null;
}

/// Status agent relay (Akses Online) — PC ini menyambung keluar ke VPS supaya
/// HP kasir bisa dipakai dari luar wifi toko.
export interface RelayStatus {
  enabled: boolean;
  connected: boolean;
  url: string;
  store_id: string;
  last_error: string | null;
  connected_since: string | null;
}

/// Isi QR pairing — semua yang perlu diisi di HP, jadi satu.
export interface PairingPayload {
  v: number;
  name: string;
  host: string;
  port: number;
  relay: string;
  store_id: string;
  code: string;
}

export interface PairingQr {
  svg: string;
  payload: PairingPayload;
}

/// Kode Setup untuk PC kasir klien: isi QR yang sama, dikemas jadi satu baris
/// teks yang bisa disalin (PC tidak punya kamera untuk scan QR).
export interface SetupCode {
  code: string;
  payload: PairingPayload;
}

/// HP kasir yang sudah terdaftar di Server Pusat ini.
export interface MobileDevice {
  id: string;
  name: string;
  created_at: string;
  last_seen_at: string | null;
  revoked: boolean;
}

export interface DedupeDetail {
  barcode: string;
  kept_name: string;
  removed_count: number;
}

export interface DedupeResult {
  groups: number;
  removed: number;
  details: DedupeDetail[];
}

export type ModuleKey =
  | "master"
  | "penjualan"
  | "persediaan"
  | "laporan"
  | "pengaturan"
  | "cek-harga";

export interface User {
  id: string;
  username: string;
  name: string;
  role: string;
  permissions: ModuleKey[];
}

export interface UserInput {
  id?: string | null;
  username: string;
  name: string;
  role: string;
  permissions: ModuleKey[];
  pin?: string | null;
}

export type StockKind = "in" | "out" | "opname" | "sale";

export interface StockMovement {
  id: number;
  product_id: string;
  product_name: string;
  kind: StockKind;
  qty: number;
  note: string | null;
  user_id: string | null;
  created_at: string;
  stock_after: number;
}

export interface StockMovementInput {
  product_id: string;
  kind: StockKind;
  qty: number;
  note?: string | null;
  user_id?: string | null;
  created_at?: string | null;
}

/** Opname Spesial: barang yang benar-benar dihitung (qty = stok fisik absolut). */
export interface OpnameSpecialItemInput {
  product_id: string;
  qty: number;
}

export interface OpnameSpecialInput {
  brand: string;
  note?: string | null;
  user_id?: string | null;
  items: OpnameSpecialItemInput[];
  created_at?: string | null;
}

export interface OpnameSpecialResult {
  brand: string;
  counted: number;
  zeroed: number;
  created_at: string;
}

export interface StockMovementBatchItemInput {
  product_id: string;
  qty: number;
  note?: string | null;
}

export interface StockMovementBatchInput {
  kind: "in" | "out";
  note?: string | null;
  user_id?: string | null;
  items: StockMovementBatchItemInput[];
  created_at?: string | null;
}

export interface StockMovementBatchItem {
  product_id: string;
  product_name: string;
  qty: number;
  note: string | null;
}

export interface StockMovementBatch {
  id: string;
  no: string;
  kind: "in" | "out";
  note: string | null;
  user_id: string | null;
  created_at: string;
  item_count: number;
  total_qty: number;
}

export interface StockMovementBatchDetail {
  id: string;
  no: string;
  kind: "in" | "out";
  note: string | null;
  user_id: string | null;
  created_at: string;
  items: StockMovementBatchItem[];
}

export interface StockMovementBatchPage {
  items: StockMovementBatch[];
  total: number;
}

// ---------- Bridge: Pull dari app mobile (galaxyas-mobile, fase 6) ----------
// Satu baris menu Barang yang barcode+qty dikirimnya sudah keisi (siap di-pull).

export interface PullItem {
  id: string;
  barcode: string;
  name: string | null;
  qty_dikirim: number;
  /** false = barcode ini belum match produk apa pun di database toko ini. */
  known_locally: boolean;
}

export interface DiscountPeriod {
  id: string;
  code: string;
  scope: "item" | "brand";
  target: string;
  target_label: string | null;
  discount_type: "amount" | "percent";
  value: number;
  days: string;
  is_active: boolean;
  priority: number;
  updated_at: string;
}

export interface DiscountPeriodInput {
  id?: string | null;
  code: string;
  scope: string; // "item" | "brand"
  target: string;
  target_label?: string | null;
  discount_type: string; // "amount" | "percent"
  value: number;
  days: string;
  is_active?: boolean;
  priority?: number;
}

export interface Customer {
  id: string;
  name: string;
  phone: string | null;
  email: string | null;
  address: string | null;
  note: string | null;
  is_active: boolean;
  updated_at: string;
}

export interface CustomerInput {
  id?: string | null;
  name: string;
  phone?: string | null;
  email?: string | null;
  address?: string | null;
  note?: string | null;
  is_active?: boolean;
}

export interface Expense {
  id: string;
  date: string;
  category: string;
  amount: number;
  note: string | null;
  user_id: string | null;
  created_at: string;
}

export interface ExpenseInput {
  id?: string | null;
  date: string;
  category: string;
  amount: number;
  note?: string | null;
  user_id?: string | null;
}

export interface Shift {
  id: string;
  user_id: string;
  user_name: string;
  opening_cash: number;
  closing_cash: number | null;
  expected_cash: number | null;
  difference: number | null;
  note: string | null;
  opened_at: string;
  closed_at: string | null;
}

export interface OpenShiftInput {
  user_id: string;
  user_name: string;
  opening_cash: number;
}

export interface CloseShiftInput {
  id: string;
  closing_cash: number;
  note?: string | null;
}

export interface ProductSalesRow {
  product_id: string;
  name: string;
  brand: string | null;
  qty: number;
  gross: number;
  discount: number;
  net: number;
  cogs: number;
}

export interface BrandSalesRow {
  brand: string;
  qty: number;
  gross: number;
  discount: number;
  net: number;
}

export interface SalesItemDetailRow {
  invoice_no: string;
  created_at: string;
  cashier_id: string;
  product_id: string;
  name: string;
  barcode: string | null;
  brand: string | null;
  qty: number;
  price: number;
  discount: number;
  net: number;
}

export interface DailySalesRow {
  day: string;
  qty: number;
  gross: number;
  discount: number;
  net: number;
}

/** Satu baris rekap Alur Barang (semua barang) untuk satu rentang tanggal. */
export interface StockFlowRow {
  product_id: string;
  name: string;
  barcode: string | null;
  brand: string | null;
  opening: number;
  masuk: number;
  keluar: number;
  terjual: number;
  /** Selisih yang tidak dijelaskan masuk/keluar/terjual — hasil koreksi opname. */
  adjustment: number;
  closing: number;
  current_stock: number;
}

/** Buku besar stok satu barang: header + stok awal + mutasi kronologis. */
export interface StockFlowDetail {
  product_id: string;
  name: string;
  barcode: string | null;
  brand: string | null;
  unit: string | null;
  current_stock: number;
  opening: number;
  rows: StockMovement[];
}

// --- Migrasi toko (berkas .gpos) -------------------------------------------
// Bentuknya dikunci `contracts/MIGRASI.md` di repo Gpos2 dan dipakai dua
// aplikasi. Yang berubah di sini berubah juga di `src-tauri/src/migrasi.rs`
// dan di sisi Python — bukan di sini saja.

/** Asal-usul satu berkas `.gpos`, dibaca dari kepalanya yang terbuka. */
export interface MigrationSource {
  /** `gpos1` (aplikasi ini) atau `gpos2`. */
  app: string;
  versiApp: string;
  versiBundel: number;
  dibuat: string;
  tokoId: string;
  tokoNama: string;
  jumlah: Record<string, number>;
  omzetTotal: number;
  penjualanPertama: string | null;
  penjualanTerakhir: string | null;
  /** Kalimat dari aplikasi pembuat: apa yang akan hilang kalau diimpor ke sini. */
  catatan: string[];
}

export interface MigrationResult {
  berkas: string;
  ukuran: number;
  sumber: MigrationSource;
  baris: Record<string, number>;
  dilewati: Record<string, number>;
  peringatan: string[];
  /** Letak salinan database sebelum diganti — jalan pulang kalau salah berkas. */
  cadangan: string | null;
}

export interface MigrationExport {
  path: string;
  hasil: MigrationResult;
}
