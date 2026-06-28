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
}

export interface DiscountPeriod {
  id: string;
  scope: "item" | "brand";
  target: string;
  target_label: string | null;
  discount_type: "amount" | "percent";
  value: number;
  days: string;
  is_active: boolean;
  updated_at: string;
}

export interface DiscountPeriodInput {
  id?: string | null;
  scope: string; // "item" | "brand"
  target: string;
  target_label?: string | null;
  discount_type: string; // "amount" | "percent"
  value: number;
  days: string;
  is_active?: boolean;
}
