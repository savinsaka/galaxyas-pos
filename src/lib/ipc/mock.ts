/**
 * In-memory mock of the Rust backend, used only when running in a plain browser
 * (`npm run dev`) without the Tauri runtime. It persists to localStorage so the
 * demo survives reloads. This is NOT used inside the packaged desktop app.
 */
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
  SaleItem,
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
  User,
} from "@/types";

const STORE_ID = "store-001";
const now = () => Date.now();
const uuid = () =>
  globalThis.crypto?.randomUUID?.() ??
  `id-${Math.random().toString(36).slice(2)}-${now()}`;

interface DB {
  store: Store;
  printer: PrinterSettings;
  users: User[];
  items: Item[];
  stockTx: StockTransaction[];
  sales: Sale[];
  saleItems: SaleItem[];
  shifts: Shift[];
  audit: AuditLog[];
  conflicts: SyncConflict[];
  session: Session | null;
  pendingCount: number;
  lastSyncAt: number | null;
}

const STORAGE_KEY = "galaxyas-mock-db-v1";

function seed(): DB {
  const tNow = now();
  const users: User[] = [
    {
      id: "user-admin",
      store_id: STORE_ID,
      username: "admin",
      full_name: "Administrator",
      role: "admin",
      is_active: true,
      created_at: tNow,
      updated_at: tNow,
    },
    {
      id: "user-kasir",
      store_id: STORE_ID,
      username: "kasir",
      full_name: "Kasir Satu",
      role: "kasir",
      is_active: true,
      created_at: tNow,
      updated_at: tNow,
    },
  ];
  const sampleNames = [
    "Indomie Goreng",
    "Aqua 600ml",
    "Teh Botol Sosro",
    "Kopi Kapal Api",
    "Beras Pandan 5kg",
    "Minyak Goreng 2L",
    "Gula Pasir 1kg",
    "Telur Ayam 1kg",
    "Susu Ultra 1L",
    "Sabun Lifebuoy",
  ];
  const items: Item[] = sampleNames.map((nama, i) => ({
    id: `item-${i + 1}`,
    store_id: STORE_ID,
    kode_item: `BRG${String(i + 1).padStart(4, "0")}`,
    barcode: `89900000000${i + 1}`,
    nama_item: nama,
    jenis: i % 2 === 0 ? "Makanan" : "Minuman",
    merek: "Umum",
    satuan_dasar: "PCS",
    harga_beli: 2000 + i * 500,
    harga_jual: 3000 + i * 700,
    diskon_persen: 0,
    stok: 50 + i * 3,
    created_at: tNow,
    updated_at: tNow,
    deleted_at: null,
    sync_status: "synced",
  }));
  return {
    store: {
      id: STORE_ID,
      name: "GalaxyAS Toko Pusat",
      address: "Jl. Merdeka No. 1",
      phone: "021-0000000",
      tax_percent: 11,
      created_at: tNow,
      updated_at: tNow,
    },
    printer: {
      printer_name: "POS-58",
      paper_width: 58,
      header_text: "GalaxyAS Toko Pusat",
      footer_text: "Terima kasih atas kunjungan Anda",
    },
    users,
    items,
    stockTx: [],
    sales: [],
    saleItems: [],
    shifts: [],
    audit: [],
    conflicts: [],
    session: null,
    pendingCount: 0,
    lastSyncAt: null,
  };
}

let db: DB = load();

function load(): DB {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw) as DB;
  } catch {
    /* ignore */
  }
  return seed();
}

function persist() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(db));
  } catch {
    /* ignore */
  }
}

function audit(action: string, entityType?: string, entityId?: string) {
  db.audit.unshift({
    id: uuid(),
    store_id: STORE_ID,
    user_id: db.session?.user.id ?? "system",
    action,
    entity_type: entityType ?? null,
    entity_id: entityId ?? null,
    detail: null,
    created_at: now(),
  });
}

const delay = (ms = 60) => new Promise((r) => setTimeout(r, ms));

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const handlers: Record<string, (a: any) => unknown> = {
  login: ({ username }: { username: string; password: string }) => {
    const user = db.users.find((u) => u.username === username);
    if (!user) throw new Error("Username atau password salah");
    const session: Session = {
      user,
      token: `mock-token-${uuid()}`,
      expires_at: now() + 15 * 60 * 1000,
    };
    db.session = session;
    audit("login");
    return session;
  },
  logout: () => {
    audit("logout");
    db.session = null;
    return null;
  },
  current_session: () => db.session,
  list_users: () => db.users,

  get_store: () => db.store,
  update_store: ({ input }: { input: Partial<Store> }) => {
    db.store = { ...db.store, ...input, updated_at: now() };
    audit("update_store", "store", db.store.id);
    return db.store;
  },
  get_printer_settings: () => db.printer,
  update_printer_settings: ({ settings }: { settings: PrinterSettings }) => {
    db.printer = settings;
    return db.printer;
  },

  list_items: ({ query }: { query: ItemQuery }): ItemPage => {
    let rows = db.items.filter((i) => !i.deleted_at);
    if (query.search) {
      const s = query.search.toLowerCase();
      rows = rows.filter(
        (i) =>
          i.nama_item.toLowerCase().includes(s) ||
          i.barcode?.toLowerCase().includes(s) ||
          i.kode_item?.toLowerCase().includes(s),
      );
    }
    if (query.jenis) rows = rows.filter((i) => i.jenis === query.jenis);
    if (query.merek) rows = rows.filter((i) => i.merek === query.merek);
    rows.sort((a, b) =>
      a.nama_item === b.nama_item
        ? a.id.localeCompare(b.id)
        : a.nama_item.localeCompare(b.nama_item),
    );
    const total = rows.length;
    let start = 0;
    if (query.cursor_name != null && query.cursor_id != null) {
      const idx = rows.findIndex(
        (i) =>
          i.nama_item === query.cursor_name && i.id === query.cursor_id,
      );
      if (idx >= 0) start = idx + 1;
    }
    const page = rows.slice(start, start + query.limit);
    const last = page[page.length - 1];
    return {
      items: page,
      total,
      next_cursor_name: page.length === query.limit && last ? last.nama_item : null,
      next_cursor_id: page.length === query.limit && last ? last.id : null,
    };
  },
  get_item: ({ id }: { id: string }) => {
    const it = db.items.find((i) => i.id === id);
    if (!it) throw new Error("Item tidak ditemukan");
    return it;
  },
  find_item_by_barcode: ({ barcode }: { barcode: string }) =>
    db.items.find((i) => i.barcode === barcode && !i.deleted_at) ?? null,
  create_item: ({ input }: { input: ItemInput }): Item => {
    const item: Item = {
      id: uuid(),
      store_id: STORE_ID,
      kode_item: input.kode_item ?? null,
      barcode: input.barcode ?? null,
      nama_item: input.nama_item,
      jenis: input.jenis ?? null,
      merek: input.merek ?? null,
      satuan_dasar: input.satuan_dasar ?? null,
      harga_beli: input.harga_beli,
      harga_jual: input.harga_jual,
      diskon_persen: input.diskon_persen,
      stok: 0,
      created_at: now(),
      updated_at: now(),
      deleted_at: null,
      sync_status: "pending",
    };
    db.items.push(item);
    db.pendingCount++;
    audit("create_item", "item", item.id);
    return item;
  },
  update_item: ({ id, input }: { id: string; input: ItemInput }) => {
    const it = db.items.find((i) => i.id === id);
    if (!it) throw new Error("Item tidak ditemukan");
    Object.assign(it, input, { updated_at: now(), sync_status: "pending" });
    db.pendingCount++;
    audit("update_item", "item", id);
    return it;
  },
  delete_item: ({ id }: { id: string }) => {
    const it = db.items.find((i) => i.id === id);
    if (it) {
      it.deleted_at = now();
      it.updated_at = now();
      it.sync_status = "pending";
      db.pendingCount++;
      audit("delete_item", "item", id);
    }
    return null;
  },
  duplicate_item: ({ id }: { id: string }) => {
    const src = db.items.find((i) => i.id === id);
    if (!src) throw new Error("Item tidak ditemukan");
    const copy: Item = {
      ...src,
      id: uuid(),
      kode_item: src.kode_item ? `${src.kode_item}-COPY` : null,
      barcode: null,
      nama_item: `${src.nama_item} (Copy)`,
      stok: 0,
      created_at: now(),
      updated_at: now(),
      sync_status: "pending",
    };
    db.items.push(copy);
    db.pendingCount++;
    return copy;
  },
  bulk_upsert_items: ({ items }: { items: ItemInput[] }) => {
    let inserted = 0;
    let updated = 0;
    const errors: string[] = [];
    items.forEach((input, idx) => {
      if (!input.nama_item) {
        errors.push(`Baris ${idx + 2}: nama_item kosong`);
        return;
      }
      const existing = input.barcode
        ? db.items.find((i) => i.barcode === input.barcode)
        : undefined;
      if (existing) {
        Object.assign(existing, input, {
          updated_at: now(),
          sync_status: "pending",
        });
        updated++;
      } else {
        handlers.create_item({ input });
        inserted++;
      }
    });
    db.pendingCount += inserted + updated;
    audit("bulk_upsert_items");
    return { inserted, updated, errors };
  },
  mass_update_items: ({
    items,
  }: {
    items: Array<{ id: string } & Partial<ItemInput>>;
  }) => {
    let count = 0;
    for (const patch of items) {
      const it = db.items.find((i) => i.id === patch.id);
      if (it) {
        Object.assign(it, patch, { updated_at: now(), sync_status: "pending" });
        count++;
      }
    }
    db.pendingCount += count;
    audit("mass_update_items");
    return count;
  },

  record_stock_movement: ({ input }: { input: StockMovementInput }) => {
    const it = db.items.find((i) => i.id === input.item_id);
    if (!it) throw new Error("Item tidak ditemukan");
    const before = it.stok;
    let after = before;
    if (input.type === "in") after = before + input.qty;
    else if (input.type === "out") after = before - input.qty;
    else after = input.qty; // adjustment / opname set absolute
    it.stok = after;
    it.updated_at = now();
    it.sync_status = "pending";
    const tx: StockTransaction = {
      id: uuid(),
      store_id: STORE_ID,
      item_id: input.item_id,
      type: input.type,
      qty: input.qty,
      qty_before: before,
      qty_after: after,
      ref_doc: input.ref_doc ?? null,
      note: input.note ?? null,
      user_id: db.session?.user.id ?? "system",
      created_at: now(),
      updated_at: now(),
      sync_status: "pending",
    };
    db.stockTx.unshift(tx);
    db.pendingCount += 2;
    audit("record_stock_movement", "stock_transaction", tx.id);
    return tx;
  },
  list_stock_transactions: ({
    itemId,
    limit,
  }: {
    itemId: string | null;
    limit: number;
  }) =>
    db.stockTx
      .filter((t) => (itemId ? t.item_id === itemId : true))
      .slice(0, limit),
  apply_stock_opname: ({
    rows,
  }: {
    rows: Array<{ item_id: string; counted_qty: number }>;
    refDoc: string;
  }) => {
    let count = 0;
    for (const r of rows) {
      handlers.record_stock_movement({
        input: {
          item_id: r.item_id,
          type: "opname",
          qty: r.counted_qty,
          note: "Stock opname",
        },
      });
      count++;
    }
    return count;
  },

  create_sale: ({ input }: { input: SaleInput }): Sale => {
    const subtotal = input.items.reduce(
      (s, i) => s + (i.harga * i.qty - i.diskon),
      0,
    );
    const afterDiskon = subtotal - input.diskon;
    const pajak = Math.round((afterDiskon * input.pajak_persen) / 100);
    const total = afterDiskon + pajak;
    const saleId = uuid();
    const sale: Sale = {
      id: saleId,
      store_id: STORE_ID,
      invoice_no:
        input.status === "completed"
          ? `INV-${new Date().toISOString().slice(0, 10)}-${(db.sales.length + 1)
              .toString()
              .padStart(4, "0")}`
          : null,
      user_id: db.session?.user.id ?? "system",
      shift_id: input.shift_id ?? null,
      subtotal,
      diskon: input.diskon,
      pajak,
      total,
      bayar: input.bayar,
      kembali: Math.max(0, input.bayar - total),
      status: input.status,
      created_at: now(),
      updated_at: now(),
      deleted_at: null,
      sync_status: "pending",
    };
    db.sales.unshift(sale);
    for (const li of input.items) {
      db.saleItems.push({
        id: uuid(),
        sale_id: saleId,
        item_id: li.item_id,
        nama_item: li.nama_item,
        qty: li.qty,
        harga: li.harga,
        diskon: li.diskon,
        subtotal: li.harga * li.qty - li.diskon,
      });
      if (input.status === "completed") {
        const it = db.items.find((i) => i.id === li.item_id);
        if (it) it.stok -= li.qty;
      }
    }
    db.pendingCount++;
    audit("create_sale", "sale", saleId);
    return sale;
  },
  get_sale: ({ id }: { id: string }): SaleWithItems => {
    const sale = db.sales.find((s) => s.id === id);
    if (!sale) throw new Error("Transaksi tidak ditemukan");
    return { sale, items: db.saleItems.filter((i) => i.sale_id === id) };
  },
  list_held_sales: () => db.sales.filter((s) => s.status === "held"),
  void_sale: ({ id }: { id: string; reason: string }) => {
    const sale = db.sales.find((s) => s.id === id);
    if (sale) {
      sale.status = "void";
      sale.updated_at = now();
      sale.sync_status = "pending";
      audit("void_sale", "sale", id);
    }
    return null;
  },

  open_shift: ({ openingCash }: { openingCash: number }) => {
    const shift: Shift = {
      id: uuid(),
      store_id: STORE_ID,
      user_id: db.session?.user.id ?? "system",
      opening_cash: openingCash,
      closing_cash: null,
      expected_cash: null,
      total_sales: 0,
      opened_at: now(),
      closed_at: null,
      status: "open",
      created_at: now(),
      updated_at: now(),
      sync_status: "pending",
    };
    db.shifts.unshift(shift);
    audit("open_shift", "shift", shift.id);
    return shift;
  },
  close_shift: ({ closingCash }: { closingCash: number }) => {
    const shift = db.shifts.find((s) => s.status === "open");
    if (!shift) throw new Error("Tidak ada shift yang terbuka");
    const totalSales = db.sales
      .filter((s) => s.status === "completed" && s.shift_id === shift.id)
      .reduce((sum, s) => sum + s.total, 0);
    shift.total_sales = totalSales;
    shift.expected_cash = shift.opening_cash + totalSales;
    shift.closing_cash = closingCash;
    shift.closed_at = now();
    shift.status = "closed";
    shift.updated_at = now();
    shift.sync_status = "pending";
    audit("close_shift", "shift", shift.id);
    return shift;
  },
  current_shift: () => db.shifts.find((s) => s.status === "open") ?? null,

  list_audit_logs: ({ limit }: { limit: number }) => db.audit.slice(0, limit),

  daily_sales_report: ({
    fromMs,
    toMs,
  }: {
    fromMs: number;
    toMs: number;
  }): DailySalesReport[] => {
    const map = new Map<string, DailySalesReport>();
    for (const s of db.sales) {
      if (s.status !== "completed") continue;
      if (s.created_at < fromMs || s.created_at > toMs) continue;
      const date = new Date(s.created_at).toISOString().slice(0, 10);
      const row = map.get(date) ?? {
        date,
        total_transactions: 0,
        total_revenue: 0,
        total_discount: 0,
        total_tax: 0,
      };
      row.total_transactions++;
      row.total_revenue += s.total;
      row.total_discount += s.diskon;
      row.total_tax += s.pajak;
      map.set(date, row);
    }
    return [...map.values()].sort((a, b) => b.date.localeCompare(a.date));
  },
  top_selling_products: ({
    limit,
  }: {
    fromMs: number;
    toMs: number;
    limit: number;
  }): TopSellingProduct[] => {
    const map = new Map<string, TopSellingProduct>();
    for (const li of db.saleItems) {
      const row = map.get(li.item_id) ?? {
        item_id: li.item_id,
        nama_item: li.nama_item,
        total_qty: 0,
        total_revenue: 0,
      };
      row.total_qty += li.qty;
      row.total_revenue += li.subtotal;
      map.set(li.item_id, row);
    }
    return [...map.values()]
      .sort((a, b) => b.total_qty - a.total_qty)
      .slice(0, limit);
  },
  stock_report: (): StockReportRow[] =>
    db.items
      .filter((i) => !i.deleted_at)
      .map((i) => ({
        item_id: i.id,
        kode_item: i.kode_item,
        nama_item: i.nama_item,
        satuan_dasar: i.satuan_dasar,
        stok: i.stok,
        harga_jual: i.harga_jual,
        nilai_stok: i.stok * i.harga_jual,
      })),

  sync_status: (): SyncStatusInfo => ({
    state: navigator.onLine ? "idle" : "offline",
    pending_count: db.pendingCount,
    conflict_count: db.conflicts.filter((c) => c.resolution === "pending")
      .length,
    last_sync_at: db.lastSyncAt,
    last_error: null,
  }),
  trigger_sync: (): SyncStatusInfo => {
    db.pendingCount = 0;
    db.lastSyncAt = now();
    db.items.forEach((i) => (i.sync_status = "synced"));
    return handlers.sync_status({}) as SyncStatusInfo;
  },
  list_conflicts: () => db.conflicts,
  resolve_conflict: ({
    id,
    resolution,
  }: {
    id: string;
    resolution: "kept_local" | "kept_server";
  }) => {
    const c = db.conflicts.find((x) => x.id === id);
    if (c) {
      c.resolution = resolution;
      c.resolved_at = now();
      c.resolved_by = db.session?.user.id ?? "system";
    }
    return null;
  },
};

export async function mockInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  await delay();
  const handler = handlers[command];
  if (!handler) {
    throw new Error(`[mock] command tidak dikenal: ${command}`);
  }
  const result = handler(args ?? {}) as T;
  persist();
  return result;
}
