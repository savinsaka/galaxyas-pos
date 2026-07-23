import { invoke } from "@tauri-apps/api/core";
import type {
  Brand,
  BrandInput,
  BrandSalesRow,
  CloseShiftInput,
  Customer,
  CustomerInput,
  DailySalesRow,
  DedupeResult,
  DiscountPeriod,
  DiscountPeriodInput,
  Expense,
  ExpenseInput,
  OpenShiftInput,
  Product,
  ProductInput,
  ProductPage,
  ProductSalesRow,
  ProductWithStock,
  SaleInput,
  SalesItemDetailRow,
  ServerInfo,
  Shift,
  StockMovement,
  StockMovementInput,
  StockMovementBatch,
  StockMovementBatchDetail,
  StockMovementBatchInput,
  StockMovementBatchItemInput,
  StockMovementBatchPage,
  TransactionDetail,
  TransactionPage,
  User,
  UserInput,
} from "./types";

/**
 * Wrapper terketik untuk semua command — SALINAN desktop/src/lib/api.ts dengan
 * satu perbedaan: semua command DATA di-proxy ke Server Pusat lewat satu
 * command Rust generik `rpc(name, args)` (HP tidak punya database sendiri).
 *
 * Host `/rpc/<name>` mengharapkan key argumen level-atas snake_case (persis
 * nama parameter command Rust-nya) — di desktop konversi camelCase→snake_case
 * itu dilakukan otomatis oleh Tauri saat invoke; di sini kita lakukan sendiri
 * karena args diteruskan sebagai JSON mentah. Field NESTED (ProductInput,
 * SaleInput, dst) sudah snake_case di types.ts, tidak perlu dikonversi.
 */
const toSnake = (s: string) => s.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`);

function rpc<T>(name: string, args: Record<string, unknown> = {}): Promise<T> {
  const converted: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(args)) converted[toSnake(k)] = v;
  return invoke<T>("rpc", { name, args: converted });
}

export const api = {
  // Pengaturan (per-device — settings.json lokal, bukan dari Server Pusat)
  getSettings: () => invoke<Record<string, string>>("get_settings"),
  updateSetting: (key: string, value: string) =>
    invoke<void>("update_setting", { key, value }),

  // Barang & stok
  listProducts: (search = "", includeInactive = false, limit?: number) =>
    rpc<ProductWithStock[]>("list_products", { search, includeInactive, limit }),
  listProductsPage: (opts: {
    search?: string;
    includeInactive?: boolean;
    brand?: string | null;
    sortBy?: string;
    sortDir?: "asc" | "desc";
    limit: number;
    offset: number;
  }) =>
    rpc<ProductPage>("list_products_page", {
      search: opts.search ?? "",
      includeInactive: opts.includeInactive ?? false,
      brand: opts.brand ?? null,
      sortBy: opts.sortBy ?? "name",
      sortDir: opts.sortDir ?? "asc",
      limit: opts.limit,
      offset: opts.offset,
    }),
  saveProduct: (input: ProductInput) => rpc<Product>("save_product", { input }),
  toggleProductActive: (id: string, active: boolean) =>
    rpc<void>("toggle_product_active", { id, active }),
  deleteProduct: (id: string) => rpc<void>("delete_product", { id }),
  dedupeProducts: () => rpc<DedupeResult>("dedupe_products"),
  findByBarcode: (barcode: string) =>
    rpc<ProductWithStock | null>("find_by_barcode", { barcode }),
  adjustStock: (productId: string, delta: number) =>
    rpc<number>("adjust_stock", { productId, delta }),
  setStock: (productId: string, qty: number) =>
    rpc<number>("set_stock", { productId, qty }),

  // Penjualan / kasir
  checkout: (sale: SaleInput) => rpc<TransactionDetail>("checkout", { sale }),
  listTransactions: async (from: string | null = null, to: string | null = null, limit = 100) =>
    (await rpc<TransactionPage>("list_transactions", { from, to, limit, offset: 0 })).items,
  listTransactionsPage: (
    from: string | null = null,
    to: string | null = null,
    limit = 50,
    offset = 0,
    search: string | null = null,
  ) => rpc<TransactionPage>("list_transactions", { from, to, search, limit, offset }),
  getTransaction: (id: string) =>
    rpc<TransactionDetail | null>("get_transaction", { id }),
  deleteTransaction: (id: string) => rpc<void>("delete_transaction", { id }),
  updateTransaction: (id: string, sale: SaleInput) =>
    rpc<TransactionDetail>("update_transaction", { id, input: sale }),

  // Pengguna / hak akses
  login: (username: string, pin: string) =>
    rpc<User | null>("login", { username, pin }),
  listUsers: () => rpc<User[]>("list_users"),
  saveUser: (input: UserInput) => rpc<User>("save_user", { input }),
  deleteUser: (id: string) => rpc<void>("delete_user", { id }),

  // Pergerakan stok
  createStockMovement: (input: StockMovementInput) =>
    rpc<StockMovement>("create_stock_movement", { input }),
  listStockMovements: (
    kind: string | null = null,
    from: string | null = null,
    to: string | null = null,
    limit = 500,
  ) => rpc<StockMovement[]>("list_stock_movements", { kind, from, to, limit }),
  deleteStockMovement: (id: number) =>
    rpc<void>("delete_stock_movement", { id }),

  // Batch Item Masuk / Keluar (satu transaksi = banyak barang)
  createStockMovementBatch: (input: StockMovementBatchInput) =>
    rpc<StockMovementBatchDetail>("create_stock_movement_batch", { input }),
  listStockMovementBatches: (
    kind: "in" | "out" | null = null,
    from: string | null = null,
    to: string | null = null,
    limit = 50,
    offset = 0,
  ) => rpc<StockMovementBatchPage>("list_stock_movement_batches", { kind, from, to, limit, offset }),
  getStockMovementBatch: (id: string) =>
    rpc<StockMovementBatchDetail | null>("get_stock_movement_batch", { id }),
  updateStockMovementBatch: (id: string, items: StockMovementBatchItemInput[], note: string | null) =>
    rpc<StockMovementBatchDetail>("update_stock_movement_batch", { id, items, note }),
  deleteStockMovementBatch: (id: string) =>
    rpc<void>("delete_stock_movement_batch", { id }),

  // Diskon periodik
  listDiscounts: () => rpc<DiscountPeriod[]>("list_discounts"),
  saveDiscount: (input: DiscountPeriodInput) =>
    rpc<DiscountPeriod>("save_discount", { input }),
  deleteDiscount: (id: string) => rpc<void>("delete_discount", { id }),

  // Merek
  listBrands: () => rpc<Brand[]>("list_brands"),
  saveBrand: (input: BrandInput) => rpc<Brand>("save_brand", { input }),
  deleteBrand: (id: string) => rpc<void>("delete_brand", { id }),

  // Pelanggan
  listCustomers: (search = "", includeInactive = false) =>
    rpc<Customer[]>("list_customers", { search, includeInactive }),
  saveCustomer: (input: CustomerInput) => rpc<Customer>("save_customer", { input }),
  deleteCustomer: (id: string) => rpc<void>("delete_customer", { id }),

  // Pengeluaran (Kas Keluar)
  listExpenses: (from: string | null = null, to: string | null = null) =>
    rpc<Expense[]>("list_expenses", { from, to }),
  saveExpense: (input: ExpenseInput) => rpc<Expense>("save_expense", { input }),
  deleteExpense: (id: string) => rpc<void>("delete_expense", { id }),

  // Shift kasir (buka/tutup, rekonsiliasi)
  getActiveShift: () => rpc<Shift | null>("get_active_shift"),
  openShift: (input: OpenShiftInput) => rpc<Shift>("open_shift", { input }),
  closeShift: (input: CloseShiftInput) => rpc<Shift>("close_shift", { input }),
  listShifts: (limit = 100) => rpc<Shift[]>("list_shifts", { limit }),

  // Laporan per barang / per merek
  productSalesReport: (from: string, to: string, brands: string[] = []) =>
    rpc<ProductSalesRow[]>("product_sales_report", { from, to, brands }),
  brandSalesReport: (from: string, to: string, brands: string[] = []) =>
    rpc<BrandSalesRow[]>("brand_sales_report", { from, to, brands }),
  salesItemDetailReport: (from: string, to: string, brands: string[] = []) =>
    rpc<SalesItemDetailRow[]>("sales_item_detail_report", { from, to, brands }),
  dailySalesReport: (from: string, to: string, brands: string[] = []) =>
    rpc<DailySalesRow[]>("daily_sales_report", { from, to, brands }),

  // Sistem: file sementara (share PDF laporan)
  writeTempFile: (fileName: string, bytes: number[]) =>
    invoke<string>("write_temp_file", { fileName, bytes }),

  // Printer thermal Bluetooth — diisi plugin btprinter di Phase 2.
  // Bentuk API dipertahankan sama dengan desktop supaya call site port mulus.
  listPrinters: (): Promise<string[]> =>
    Promise.reject(new Error("Printer Bluetooth belum tersedia (Phase 2).")),
  printEscposTo: (_printer: string | null, _bytes: Uint8Array): Promise<void> =>
    Promise.reject(new Error("Printer Bluetooth belum tersedia (Phase 2).")),

  // Server Pusat (registry pairing — lokal di HP)
  listServers: () => invoke<ServerInfo[]>("list_servers"),
  currentServer: () => invoke<ServerInfo | null>("current_server"),
  pingServer: (host: string, port: number, token: string) =>
    invoke<string>("ping_server", { host, port, token }),
  addServer: (name: string, host: string, port: number, token: string) =>
    invoke<ServerInfo>("add_server", { name, host, port, token }),
  selectServer: (id: string) => invoke<ServerInfo>("select_server", { id }),
  removeServer: (id: string) => invoke<void>("remove_server", { id }),
};
