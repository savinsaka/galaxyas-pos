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
  LanServerStatus,
  MobileDevice,
  OpenShiftInput,
  OpnameSpecialInput,
  OpnameSpecialResult,
  PairingQr,
  Product,
  ProductInput,
  PullItem,
  ProductPage,
  ProductSalesRow,
  ProductWithStock,
  RelayStatus,
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
  StockFlowDetail,
  StockFlowRow,
  StoreInfo,
  SyncResult,
  TransactionDetail,
  TransactionPage,
  User,
  UserInput,
} from "./types";

/** Wrapper terketik untuk semua command Tauri (backend Rust lokal). */
export const api = {
  // Multi-database (toko)
  listStores: () => invoke<StoreInfo[]>("list_stores"),
  currentStore: () => invoke<StoreInfo>("current_store"),
  createStore: (name: string) => invoke<StoreInfo>("create_store", { name }),
  selectStore: (id: string) => invoke<StoreInfo>("select_store", { id }),

  // Pengaturan
  getSettings: () => invoke<Record<string, string>>("get_settings"),
  updateSetting: (key: string, value: string) =>
    invoke<void>("update_setting", { key, value }),

  // Barang & stok
  listProducts: (search = "", includeInactive = false, limit?: number) =>
    invoke<ProductWithStock[]>("list_products", { search, includeInactive, limit }),
  listProductsPage: (opts: {
    search?: string;
    includeInactive?: boolean;
    brand?: string | null;
    sortBy?: string;
    sortDir?: "asc" | "desc";
    limit: number;
    offset: number;
  }) =>
    invoke<ProductPage>("list_products_page", {
      search: opts.search ?? "",
      includeInactive: opts.includeInactive ?? false,
      brand: opts.brand ?? null,
      sortBy: opts.sortBy ?? "name",
      sortDir: opts.sortDir ?? "asc",
      limit: opts.limit,
      offset: opts.offset,
    }),
  saveProduct: (input: ProductInput) => invoke<Product>("save_product", { input }),
  toggleProductActive: (id: string, active: boolean) =>
    invoke<void>("toggle_product_active", { id, active }),
  deleteProduct: (id: string) => invoke<void>("delete_product", { id }),
  dedupeProducts: () => invoke<DedupeResult>("dedupe_products"),
  resetData: (confirm: string) => invoke<void>("reset_data", { confirm }),
  findByBarcode: (barcode: string) =>
    invoke<ProductWithStock | null>("find_by_barcode", { barcode }),
  adjustStock: (productId: string, delta: number) =>
    invoke<number>("adjust_stock", { productId, delta }),
  setStock: (productId: string, qty: number) =>
    invoke<number>("set_stock", { productId, qty }),

  // Penjualan / kasir
  checkout: (sale: SaleInput) => invoke<TransactionDetail>("checkout", { sale }),
  listTransactions: async (from: string | null = null, to: string | null = null, limit = 100) =>
    (await invoke<TransactionPage>("list_transactions", { from, to, limit, offset: 0 })).items,
  listTransactionsPage: (
    from: string | null = null,
    to: string | null = null,
    limit = 50,
    offset = 0,
    search: string | null = null,
  ) => invoke<TransactionPage>("list_transactions", { from, to, search, limit, offset }),
  getTransaction: (id: string) =>
    invoke<TransactionDetail | null>("get_transaction", { id }),
  deleteTransaction: (id: string) => invoke<void>("delete_transaction", { id }),
  updateTransaction: (id: string, sale: SaleInput) =>
    invoke<TransactionDetail>("update_transaction", { id, input: sale }),

  // Pengguna / hak akses
  login: (username: string, pin: string) =>
    invoke<User | null>("login", { username, pin }),
  listUsers: () => invoke<User[]>("list_users"),
  saveUser: (input: UserInput) => invoke<User>("save_user", { input }),
  deleteUser: (id: string) => invoke<void>("delete_user", { id }),

  // Pergerakan stok
  createStockMovement: (input: StockMovementInput) =>
    invoke<StockMovement>("create_stock_movement", { input }),
  listStockMovements: (
    kind: string | null = null,
    from: string | null = null,
    to: string | null = null,
    limit = 500,
  ) => invoke<StockMovement[]>("list_stock_movements", { kind, from, to, limit }),
  deleteStockMovement: (id: number) =>
    invoke<void>("delete_stock_movement", { id }),

  // Opname Spesial (satu merek: yang dihitung diset, sisanya dinolkan)
  createOpnameSpecial: (input: OpnameSpecialInput) =>
    invoke<OpnameSpecialResult>("create_opname_special", { input }),

  // Batch Item Masuk / Keluar (satu transaksi = banyak barang)
  createStockMovementBatch: (input: StockMovementBatchInput) =>
    invoke<StockMovementBatchDetail>("create_stock_movement_batch", { input }),
  listStockMovementBatches: (
    kind: "in" | "out" | null = null,
    from: string | null = null,
    to: string | null = null,
    limit = 50,
    offset = 0,
  ) => invoke<StockMovementBatchPage>("list_stock_movement_batches", { kind, from, to, limit, offset }),
  getStockMovementBatch: (id: string) =>
    invoke<StockMovementBatchDetail | null>("get_stock_movement_batch", { id }),
  updateStockMovementBatch: (id: string, items: StockMovementBatchItemInput[], note: string | null) =>
    invoke<StockMovementBatchDetail>("update_stock_movement_batch", { id, items, note }),
  deleteStockMovementBatch: (id: string) =>
    invoke<void>("delete_stock_movement_batch", { id }),

  // Diskon periodik
  listDiscounts: () => invoke<DiscountPeriod[]>("list_discounts"),
  saveDiscount: (input: DiscountPeriodInput) =>
    invoke<DiscountPeriod>("save_discount", { input }),
  deleteDiscount: (id: string) => invoke<void>("delete_discount", { id }),

  // Merek
  listBrands: () => invoke<Brand[]>("list_brands"),
  saveBrand: (input: BrandInput) => invoke<Brand>("save_brand", { input }),
  deleteBrand: (id: string) => invoke<void>("delete_brand", { id }),

  // Pelanggan
  listCustomers: (search = "", includeInactive = false) =>
    invoke<Customer[]>("list_customers", { search, includeInactive }),
  saveCustomer: (input: CustomerInput) => invoke<Customer>("save_customer", { input }),
  deleteCustomer: (id: string) => invoke<void>("delete_customer", { id }),

  // Pengeluaran (Kas Keluar)
  listExpenses: (from: string | null = null, to: string | null = null) =>
    invoke<Expense[]>("list_expenses", { from, to }),
  saveExpense: (input: ExpenseInput) => invoke<Expense>("save_expense", { input }),
  deleteExpense: (id: string) => invoke<void>("delete_expense", { id }),

  // Shift kasir (buka/tutup, rekonsiliasi)
  getActiveShift: () => invoke<Shift | null>("get_active_shift"),
  openShift: (input: OpenShiftInput) => invoke<Shift>("open_shift", { input }),
  closeShift: (input: CloseShiftInput) => invoke<Shift>("close_shift", { input }),
  listShifts: (limit = 100) => invoke<Shift[]>("list_shifts", { limit }),

  // Laporan per barang / per merek
  productSalesReport: (from: string, to: string, brands: string[] = []) =>
    invoke<ProductSalesRow[]>("product_sales_report", { from, to, brands }),
  brandSalesReport: (from: string, to: string, brands: string[] = []) =>
    invoke<BrandSalesRow[]>("brand_sales_report", { from, to, brands }),
  salesItemDetailReport: (from: string, to: string, brands: string[] = []) =>
    invoke<SalesItemDetailRow[]>("sales_item_detail_report", { from, to, brands }),
  dailySalesReport: (from: string, to: string, brands: string[] = []) =>
    invoke<DailySalesRow[]>("daily_sales_report", { from, to, brands }),

  // Alur Barang (buku besar stok)
  stockFlowRecap: (from: string, to: string, search: string | null = null) =>
    invoke<StockFlowRow[]>("stock_flow_recap", { from, to, search }),
  stockFlowDetail: (productId: string, from: string, to: string) =>
    invoke<StockFlowDetail>("stock_flow_detail", { productId, from, to }),

  // Sistem: file & printer
  writeTempFile: (fileName: string, bytes: number[]) =>
    invoke<string>("write_temp_file", { fileName, bytes }),
  listPrinters: () => invoke<string[]>("list_printers"),
  printTextTo: (printer: string | null, text: string) =>
    invoke<void>("print_text_to", { printer, text }),
  printEscposTo: (printer: string | null, bytes: Uint8Array) =>
    invoke<void>("print_escpos_to", { printer, bytes: Array.from(bytes) }),
  openPrintWindow: (html: string, css: string, title: string, width: number, height: number) =>
    invoke<void>("open_print_window", { html, css, title, width, height }),

  // Sinkronisasi (manual)
  syncPush: () => invoke<SyncResult>("sync_push"),
  syncPull: () => invoke<SyncResult>("sync_pull"),
  syncAll: () => invoke<SyncResult>("sync_all"),

  // Bridge: Pull dari app mobile (galaxyas-mobile, fase 6) — server & auth
  // TERPISAH dari Sinkronisasi di atas (itu ke server POS, ini ke server mobile).
  bridgeListPending: () => invoke<PullItem[]>("bridge_list_pending"),
  bridgeConfirmPull: (items: PullItem[], userId: string | null) =>
    invoke<StockMovementBatchDetail>("bridge_confirm_pull", { items, userId }),
  bridgeRejectPull: (rowId: string) => invoke<void>("bridge_reject_pull", { rowId }),

  // Server Pusat (multi-kasir LAN)
  listServers: () => invoke<ServerInfo[]>("list_servers"),
  currentServer: () => invoke<ServerInfo>("current_server"),
  pingServer: (host: string, port: number, token: string) =>
    invoke<string>("ping_server", { host, port, token }),
  addServer: (name: string, host: string, port: number, token: string) =>
    invoke<ServerInfo>("add_server", { name, host, port, token }),
  selectServer: (id: string) => invoke<ServerInfo>("select_server", { id }),
  removeServer: (id: string) => invoke<void>("remove_server", { id }),
  lanServerStatus: () => invoke<LanServerStatus>("lan_server_status"),
  setLanServerEnabled: (enabled: boolean) =>
    invoke<LanServerStatus>("set_lan_server_enabled", { enabled }),
  regenerateLanToken: () => invoke<LanServerStatus>("regenerate_lan_token"),

  // Akses Online (relay) + HP yang terdaftar.
  pairingQr: () => invoke<PairingQr>("pairing_qr"),
  listMobileDevices: () => invoke<MobileDevice[]>("list_mobile_devices"),
  revokeMobileDevice: (id: string) => invoke<void>("revoke_mobile_device", { id }),
  relayStatus: () => invoke<RelayStatus>("relay_status"),
  saveRelaySettings: (input: { url: string; store_id: string; agent_key: string }) =>
    invoke<RelayStatus>("save_relay_settings", { input }),
  setRelayEnabled: (enabled: boolean) => invoke<RelayStatus>("set_relay_enabled", { enabled }),
};
