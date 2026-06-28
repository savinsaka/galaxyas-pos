import { invoke } from "@tauri-apps/api/core";
import type {
  Brand,
  BrandInput,
  DiscountPeriod,
  DiscountPeriodInput,
  Product,
  ProductInput,
  ProductWithStock,
  SaleInput,
  StockMovement,
  StockMovementInput,
  SyncResult,
  Transaction,
  TransactionDetail,
  User,
  UserInput,
} from "./types";

/** Wrapper terketik untuk semua command Tauri (backend Rust lokal). */
export const api = {
  // Pengaturan
  getSettings: () => invoke<Record<string, string>>("get_settings"),
  updateSetting: (key: string, value: string) =>
    invoke<void>("update_setting", { key, value }),

  // Barang & stok
  listProducts: (search = "", includeInactive = false) =>
    invoke<ProductWithStock[]>("list_products", { search, includeInactive }),
  saveProduct: (input: ProductInput) => invoke<Product>("save_product", { input }),
  toggleProductActive: (id: string, active: boolean) =>
    invoke<void>("toggle_product_active", { id, active }),
  deleteProduct: (id: string) => invoke<void>("delete_product", { id }),
  findByBarcode: (barcode: string) =>
    invoke<ProductWithStock | null>("find_by_barcode", { barcode }),
  adjustStock: (productId: string, delta: number) =>
    invoke<number>("adjust_stock", { productId, delta }),
  setStock: (productId: string, qty: number) =>
    invoke<number>("set_stock", { productId, qty }),

  // Penjualan / kasir
  checkout: (sale: SaleInput) => invoke<TransactionDetail>("checkout", { sale }),
  listTransactions: (limit = 100) =>
    invoke<Transaction[]>("list_transactions", { limit }),
  getTransaction: (id: string) =>
    invoke<TransactionDetail | null>("get_transaction", { id }),
  deleteTransaction: (id: string) => invoke<void>("delete_transaction", { id }),

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

  // Diskon periodik
  listDiscounts: () => invoke<DiscountPeriod[]>("list_discounts"),
  saveDiscount: (input: DiscountPeriodInput) =>
    invoke<DiscountPeriod>("save_discount", { input }),
  deleteDiscount: (id: string) => invoke<void>("delete_discount", { id }),

  // Merek
  listBrands: () => invoke<Brand[]>("list_brands"),
  saveBrand: (input: BrandInput) => invoke<Brand>("save_brand", { input }),
  deleteBrand: (id: string) => invoke<void>("delete_brand", { id }),

  // Sistem: file & printer
  writeTempFile: (fileName: string, bytes: number[]) =>
    invoke<string>("write_temp_file", { fileName, bytes }),
  listPrinters: () => invoke<string[]>("list_printers"),
  printTextTo: (printer: string | null, text: string) =>
    invoke<void>("print_text_to", { printer, text }),

  // Sinkronisasi (manual)
  syncPush: () => invoke<SyncResult>("sync_push"),
  syncPull: () => invoke<SyncResult>("sync_pull"),
  syncAll: () => invoke<SyncResult>("sync_all"),
};
