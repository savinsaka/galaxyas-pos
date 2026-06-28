import { ipcInvoke } from "./client";
import type {
  AuditLog,
  DailySalesReport,
  Item,
  ItemInput,
  ItemPage,
  ItemQuery,
  PrinterSettings,
  Sale,
  SaleInput,
  SaleWithItems,
  Session,
  Shift,
  StockMovementInput,
  StockReportRow,
  StockTransaction,
  Store,
  SyncConflict,
  SyncStatusInfo,
  TopSellingProduct,
} from "@/types";

/**
 * Typed command catalogue. Names match the `#[tauri::command]` functions
 * registered in `src-tauri/src/lib.rs`.
 */
export const ipc = {
  // ---- Auth ----
  login: (username: string, password: string) =>
    ipcInvoke<Session>("login", { username, password }),
  logout: () => ipcInvoke<void>("logout"),
  currentSession: () => ipcInvoke<Session | null>("current_session"),
  listUsers: () => ipcInvoke<Session["user"][]>("list_users"),

  // ---- Store / settings ----
  getStore: () => ipcInvoke<Store>("get_store"),
  updateStore: (store: Partial<Store>) =>
    ipcInvoke<Store>("update_store", { input: store }),
  getPrinterSettings: () =>
    ipcInvoke<PrinterSettings>("get_printer_settings"),
  updatePrinterSettings: (settings: PrinterSettings) =>
    ipcInvoke<PrinterSettings>("update_printer_settings", { settings }),

  // ---- Items ----
  listItems: (query: ItemQuery) =>
    ipcInvoke<ItemPage>("list_items", { query }),
  getItem: (id: string) => ipcInvoke<Item>("get_item", { id }),
  findItemByBarcode: (barcode: string) =>
    ipcInvoke<Item | null>("find_item_by_barcode", { barcode }),
  createItem: (input: ItemInput) =>
    ipcInvoke<Item>("create_item", { input }),
  updateItem: (id: string, input: ItemInput) =>
    ipcInvoke<Item>("update_item", { id, input }),
  deleteItem: (id: string) => ipcInvoke<void>("delete_item", { id }),
  duplicateItem: (id: string) => ipcInvoke<Item>("duplicate_item", { id }),
  bulkUpsertItems: (items: ItemInput[]) =>
    ipcInvoke<{ inserted: number; updated: number; errors: string[] }>(
      "bulk_upsert_items",
      { items },
    ),
  massUpdateItems: (items: Array<{ id: string } & Partial<ItemInput>>) =>
    ipcInvoke<number>("mass_update_items", { items }),

  // ---- Inventory ----
  recordStockMovement: (input: StockMovementInput) =>
    ipcInvoke<StockTransaction>("record_stock_movement", { input }),
  listStockTransactions: (itemId: string | null, limit: number) =>
    ipcInvoke<StockTransaction[]>("list_stock_transactions", {
      itemId,
      limit,
    }),
  applyStockOpname: (
    rows: Array<{ item_id: string; counted_qty: number }>,
    refDoc: string,
  ) => ipcInvoke<number>("apply_stock_opname", { rows, refDoc }),

  // ---- Sales / POS ----
  createSale: (input: SaleInput) => ipcInvoke<Sale>("create_sale", { input }),
  getSale: (id: string) => ipcInvoke<SaleWithItems>("get_sale", { id }),
  listHeldSales: () => ipcInvoke<Sale[]>("list_held_sales"),
  voidSale: (id: string, reason: string) =>
    ipcInvoke<void>("void_sale", { id, reason }),

  // ---- Shifts ----
  openShift: (openingCash: number) =>
    ipcInvoke<Shift>("open_shift", { openingCash }),
  closeShift: (closingCash: number) =>
    ipcInvoke<Shift>("close_shift", { closingCash }),
  currentShift: () => ipcInvoke<Shift | null>("current_shift"),

  // ---- Audit ----
  listAuditLogs: (limit: number) =>
    ipcInvoke<AuditLog[]>("list_audit_logs", { limit }),

  // ---- Reports ----
  dailySalesReport: (fromMs: number, toMs: number) =>
    ipcInvoke<DailySalesReport[]>("daily_sales_report", { fromMs, toMs }),
  topSellingProducts: (fromMs: number, toMs: number, limit: number) =>
    ipcInvoke<TopSellingProduct[]>("top_selling_products", {
      fromMs,
      toMs,
      limit,
    }),
  stockReport: () => ipcInvoke<StockReportRow[]>("stock_report"),

  // ---- Sync ----
  syncStatus: () => ipcInvoke<SyncStatusInfo>("sync_status"),
  triggerSync: () => ipcInvoke<SyncStatusInfo>("trigger_sync"),
  listConflicts: () => ipcInvoke<SyncConflict[]>("list_conflicts"),
  resolveConflict: (id: string, resolution: "kept_local" | "kept_server") =>
    ipcInvoke<void>("resolve_conflict", { id, resolution }),
};
