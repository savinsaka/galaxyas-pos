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

export interface SyncResult {
  pushed: number;
  pulled: number;
  skipped: number;
  message: string;
}

export type PaymentMethod = "Tunai" | "QRIS" | "Transfer" | "Kartu";

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

export interface ServerInfo {
  id: string;
  kind: "local" | "remote";
  name: string;
  host: string | null;
  port: number | null;
  token: string | null;
}

export interface LanServerStatus {
  enabled: boolean;
  port: number;
  token: string;
  local_ip: string | null;
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

export type ModuleKey = "master" | "penjualan" | "persediaan" | "laporan" | "pengaturan";

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
