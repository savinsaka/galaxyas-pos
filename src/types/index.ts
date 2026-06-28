/**
 * Shared TypeScript types mirroring the Rust models in `src-tauri/src/db/models.rs`.
 *
 * In a production setup these would be generated automatically via `specta` /
 * `tauri-specta`. They are kept in sync manually here so the frontend has a
 * single source of truth for the IPC contract.
 */

export type UserRole = "admin" | "supervisor" | "kasir";

export type SyncStatus = "pending" | "synced" | "conflict";

export interface User {
  id: string;
  store_id: string;
  username: string;
  full_name: string;
  role: UserRole;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export interface Store {
  id: string;
  name: string;
  address: string | null;
  phone: string | null;
  tax_percent: number;
  created_at: number;
  updated_at: number;
}

export interface Session {
  user: User;
  token: string;
  expires_at: number;
}

export interface Item {
  id: string;
  store_id: string;
  kode_item: string | null;
  barcode: string | null;
  nama_item: string;
  jenis: string | null;
  merek: string | null;
  satuan_dasar: string | null;
  harga_beli: number;
  harga_jual: number;
  diskon_persen: number;
  stok: number;
  created_at: number;
  updated_at: number;
  deleted_at: number | null;
  sync_status: SyncStatus;
}

export interface ItemInput {
  kode_item?: string | null;
  barcode?: string | null;
  nama_item: string;
  jenis?: string | null;
  merek?: string | null;
  satuan_dasar?: string | null;
  harga_beli: number;
  harga_jual: number;
  diskon_persen: number;
}

export interface ItemQuery {
  search?: string | null;
  jenis?: string | null;
  merek?: string | null;
  limit: number;
  /** Keyset pagination cursor: last seen (nama_item, id). */
  cursor_name?: string | null;
  cursor_id?: string | null;
}

export interface ItemPage {
  items: Item[];
  next_cursor_name: string | null;
  next_cursor_id: string | null;
  total: number;
}

export type StockTxType = "in" | "out" | "adjustment" | "opname";

export interface StockTransaction {
  id: string;
  store_id: string;
  item_id: string;
  type: StockTxType;
  qty: number;
  qty_before: number;
  qty_after: number;
  ref_doc: string | null;
  note: string | null;
  user_id: string;
  created_at: number;
  updated_at: number;
  sync_status: SyncStatus;
}

export interface StockMovementInput {
  item_id: string;
  type: StockTxType;
  qty: number;
  ref_doc?: string | null;
  note?: string | null;
}

export type SaleStatus = "held" | "completed" | "void";

export interface SaleItem {
  id: string;
  sale_id: string;
  item_id: string;
  nama_item: string;
  qty: number;
  harga: number;
  diskon: number;
  subtotal: number;
}

export interface Sale {
  id: string;
  store_id: string;
  invoice_no: string | null;
  user_id: string;
  shift_id: string | null;
  subtotal: number;
  diskon: number;
  pajak: number;
  total: number;
  bayar: number;
  kembali: number;
  status: SaleStatus;
  created_at: number;
  updated_at: number;
  deleted_at: number | null;
  sync_status: SyncStatus;
}

export interface SaleItemInput {
  item_id: string;
  nama_item: string;
  qty: number;
  harga: number;
  diskon: number;
}

export interface SaleInput {
  shift_id?: string | null;
  status: SaleStatus;
  diskon: number;
  pajak_persen: number;
  bayar: number;
  items: SaleItemInput[];
}

export interface SaleWithItems {
  sale: Sale;
  items: SaleItem[];
}

export interface Shift {
  id: string;
  store_id: string;
  user_id: string;
  opening_cash: number;
  closing_cash: number | null;
  expected_cash: number | null;
  total_sales: number;
  opened_at: number;
  closed_at: number | null;
  status: "open" | "closed";
  created_at: number;
  updated_at: number;
  sync_status: SyncStatus;
}

export interface AuditLog {
  id: string;
  store_id: string;
  user_id: string;
  action: string;
  entity_type: string | null;
  entity_id: string | null;
  detail: string | null;
  created_at: number;
}

export interface SyncConflict {
  id: string;
  entity_type: string;
  entity_id: string;
  local_payload: string;
  server_payload: string;
  conflict_field: string | null;
  resolution: "pending" | "kept_local" | "kept_server";
  resolved_by: string | null;
  resolved_at: number | null;
  created_at: number;
}

export type SyncState =
  | "idle"
  | "syncing"
  | "offline"
  | "error";

export interface SyncStatusInfo {
  state: SyncState;
  pending_count: number;
  conflict_count: number;
  last_sync_at: number | null;
  last_error: string | null;
}

export interface DailySalesReport {
  date: string;
  total_transactions: number;
  total_revenue: number;
  total_discount: number;
  total_tax: number;
}

export interface TopSellingProduct {
  item_id: string;
  nama_item: string;
  total_qty: number;
  total_revenue: number;
}

export interface StockReportRow {
  item_id: string;
  kode_item: string | null;
  nama_item: string;
  satuan_dasar: string | null;
  stok: number;
  harga_jual: number;
  nilai_stok: number;
}

export interface PrinterSettings {
  printer_name: string;
  paper_width: number;
  header_text: string;
  footer_text: string;
}
