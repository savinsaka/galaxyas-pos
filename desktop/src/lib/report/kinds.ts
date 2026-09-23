/**
 * Registri "jenis laporan" (kind): skema field & dataset yang boleh diikat
 * sebuah template (murni, tanpa dependensi Tauri). Pemuatan data nyata ada di
 * `data.ts`. Editor memakai skema ini untuk daftar field; renderer memakai
 * konteks (`ReportContext`) untuk mengisi band.
 */
import type { PaperPreset } from "./template";

export type FieldFormat = "text" | "money" | "qty" | "int" | "datetime";

export interface FieldDef {
  id: string;
  label: string;
  format: FieldFormat;
}

export interface DatasetDef {
  id: string;
  label: string;
  fields: FieldDef[];
}

export interface ReportKind {
  id: string;
  label: string;
  /** Folder kategori penyimpanan .Greport (harus salah satu REPORT_CATEGORIES di Rust). */
  category: string;
  /** Bentuk umum: struk thermal vs laporan tabular A4. */
  shape: "receipt" | "report";
  scalars: FieldDef[];
  datasets: DatasetDef[];
  defaultPaper: PaperPreset;
}

/** Konteks data siap-render: skalar (untuk text/pair) + dataset (untuk items). */
export interface ReportContext {
  kind: string;
  title: string;
  subtitle?: string;
  meta?: string;
  scalars: Record<string, string | number | null | undefined>;
  datasets: Record<string, Array<Record<string, unknown>>>;
}

// ---- Skema field bersama untuk struk transaksi & dokumen stok ----

const STORE_SCALARS: FieldDef[] = [
  { id: "store_name", label: "Nama Toko", format: "text" },
  { id: "address", label: "Alamat", format: "text" },
  { id: "phone", label: "Telepon", format: "text" },
  { id: "tax_id", label: "NPWP", format: "text" },
  { id: "social", label: "Sosial Media", format: "text" },
  { id: "header", label: "Header Tambahan", format: "text" },
  { id: "footer", label: "Footer", format: "text" },
];

const TRX_ITEM_FIELDS: FieldDef[] = [
  { id: "name", label: "Nama Barang", format: "text" },
  { id: "qty", label: "Qty", format: "qty" },
  { id: "price", label: "Harga", format: "money" },
  { id: "discount", label: "Diskon", format: "money" },
  { id: "line_total", label: "Subtotal Baris", format: "money" },
];

export const REPORT_KINDS: ReportKind[] = [
  {
    id: "struk-transaksi",
    label: "Struk Transaksi",
    category: "kasir",
    shape: "receipt",
    defaultPaper: "struk80",
    scalars: [
      ...STORE_SCALARS,
      { id: "invoice_no", label: "No. Invoice", format: "text" },
      { id: "date", label: "Tanggal", format: "text" },
      { id: "total_item", label: "Total Item (pcs)", format: "qty" },
      { id: "subtotal", label: "Subtotal", format: "money" },
      { id: "discount", label: "Diskon", format: "money" },
      { id: "total", label: "Total", format: "money" },
      { id: "payment_method", label: "Metode Bayar", format: "text" },
      { id: "paid", label: "Dibayar", format: "money" },
      { id: "change", label: "Kembali", format: "money" },
    ],
    datasets: [{ id: "items", label: "Daftar Item", fields: TRX_ITEM_FIELDS }],
  },
  {
    id: "dok-item-masuk",
    label: "Dokumen Item Masuk",
    category: "kasir",
    shape: "receipt",
    defaultPaper: "struk80",
    scalars: [
      ...STORE_SCALARS,
      { id: "no", label: "No. Dokumen", format: "text" },
      { id: "date", label: "Tanggal", format: "text" },
      { id: "total_item", label: "Total Jenis", format: "int" },
      { id: "total_qty", label: "Total Qty", format: "qty" },
      { id: "note", label: "Catatan", format: "text" },
      { id: "user_id", label: "Oleh", format: "text" },
    ],
    datasets: [
      {
        id: "items",
        label: "Daftar Item",
        fields: [
          { id: "product_name", label: "Nama Barang", format: "text" },
          { id: "qty", label: "Qty", format: "qty" },
          { id: "note", label: "Ket.", format: "text" },
        ],
      },
    ],
  },
  {
    id: "dok-item-keluar",
    label: "Dokumen Item Keluar",
    category: "kasir",
    shape: "receipt",
    defaultPaper: "struk80",
    scalars: [
      ...STORE_SCALARS,
      { id: "no", label: "No. Dokumen", format: "text" },
      { id: "date", label: "Tanggal", format: "text" },
      { id: "total_item", label: "Total Jenis", format: "int" },
      { id: "total_qty", label: "Total Qty", format: "qty" },
      { id: "note", label: "Catatan", format: "text" },
      { id: "user_id", label: "Oleh", format: "text" },
    ],
    datasets: [
      {
        id: "items",
        label: "Daftar Item",
        fields: [
          { id: "product_name", label: "Nama Barang", format: "text" },
          { id: "qty", label: "Qty", format: "qty" },
          { id: "note", label: "Ket.", format: "text" },
        ],
      },
    ],
  },
];

// ---- Laporan tabular (A4) ----

/** Judul/subjudul/periode — tersedia di semua laporan tabular. */
const REPORT_HEADER_SCALARS: FieldDef[] = [
  { id: "title", label: "Judul Laporan", format: "text" },
  { id: "subtitle", label: "Subjudul / Periode", format: "text" },
  { id: "meta", label: "Keterangan", format: "text" },
];

const LABEL_VALUE: FieldDef[] = [
  { id: "label", label: "Keterangan", format: "text" },
  { id: "value", label: "Nilai", format: "text" },
];
const SALES_PER_BARANG: FieldDef[] = [
  { id: "name", label: "Barang", format: "text" },
  { id: "brand", label: "Merek", format: "text" },
  { id: "qty", label: "Qty", format: "qty" },
  { id: "discount", label: "Diskon", format: "money" },
  { id: "net", label: "Total", format: "money" },
];
const SALES_PER_MEREK: FieldDef[] = [
  { id: "brand", label: "Merek", format: "text" },
  { id: "qty", label: "Qty", format: "qty" },
  { id: "discount", label: "Diskon", format: "money" },
  { id: "net", label: "Total", format: "money" },
];

REPORT_KINDS.push(
  {
    id: "recap-penjualan",
    label: "Recap Penjualan",
    category: "penjualan",
    shape: "report",
    defaultPaper: "a4",
    scalars: REPORT_HEADER_SCALARS,
    datasets: [
      { id: "ringkasan", label: "Ringkasan Penjualan", fields: LABEL_VALUE },
      { id: "laba_rugi", label: "Laba / Rugi", fields: LABEL_VALUE },
      { id: "per_barang", label: "Per Barang", fields: SALES_PER_BARANG },
      { id: "per_merek", label: "Per Merek", fields: SALES_PER_MEREK },
      {
        id: "per_periode",
        label: "Per Periode",
        fields: [
          { id: "periode", label: "Periode", format: "text" },
          { id: "count", label: "Trx", format: "int" },
          { id: "discount", label: "Diskon", format: "money" },
          { id: "total", label: "Total", format: "money" },
        ],
      },
      {
        id: "item_terlaris",
        label: "Item Terlaris",
        fields: [
          { id: "name", label: "Barang", format: "text" },
          { id: "qty", label: "Qty", format: "qty" },
        ],
      },
      {
        id: "per_metode",
        label: "Per Metode Pembayaran",
        fields: [
          { id: "metode", label: "Metode", format: "text" },
          { id: "count", label: "Trx", format: "int" },
          { id: "total", label: "Total", format: "money" },
        ],
      },
      {
        id: "per_kasir",
        label: "Per Kasir",
        fields: [
          { id: "kasir", label: "Kasir", format: "text" },
          { id: "count", label: "Trx", format: "int" },
          { id: "total", label: "Total", format: "money" },
        ],
      },
    ],
  },
  {
    id: "recap-item",
    label: "Recap Item",
    category: "item",
    shape: "report",
    defaultPaper: "a4",
    scalars: REPORT_HEADER_SCALARS,
    datasets: [
      { id: "per_barang", label: "Per Barang", fields: SALES_PER_BARANG },
      { id: "per_merek", label: "Per Merek", fields: SALES_PER_MEREK },
    ],
  },
  {
    id: "laporan-persediaan",
    label: "Laporan Persediaan",
    category: "persediaan",
    shape: "report",
    defaultPaper: "a4",
    scalars: REPORT_HEADER_SCALARS,
    datasets: [
      { id: "nilai_stok", label: "Nilai Stok", fields: [
        { id: "label", label: "Keterangan", format: "text" },
        { id: "value", label: "Nilai", format: "money" },
      ] },
      {
        id: "pergerakan",
        label: "Pergerakan per Periode",
        fields: [
          { id: "periode", label: "Periode", format: "text" },
          { id: "masuk", label: "Masuk", format: "qty" },
          { id: "keluar", label: "Keluar", format: "qty" },
          { id: "jual", label: "Terjual", format: "qty" },
          { id: "opname", label: "Opname", format: "int" },
        ],
      },
      {
        id: "rekap_barang",
        label: "Rekap per Barang",
        fields: [
          { id: "name", label: "Barang", format: "text" },
          { id: "masuk", label: "Masuk", format: "qty" },
          { id: "keluar", label: "Keluar", format: "qty" },
          { id: "jual", label: "Terjual", format: "qty" },
        ],
      },
    ],
  },
);

REPORT_KINDS.push(
  {
    id: "kasir-detail",
    label: "Laporan Kasir Detail",
    category: "penjualan",
    shape: "report",
    defaultPaper: "a4",
    scalars: REPORT_HEADER_SCALARS,
    datasets: [{ id: "ringkasan", label: "Ringkasan", fields: LABEL_VALUE }],
  },
  {
    id: "item-detail",
    label: "Laporan Item Detail",
    category: "item",
    shape: "report",
    defaultPaper: "a4",
    scalars: REPORT_HEADER_SCALARS,
    datasets: [
      {
        id: "rows",
        label: "Item Terjual",
        fields: [
          { id: "invoice_no", label: "Invoice", format: "text" },
          { id: "created_at", label: "Tanggal", format: "text" },
          { id: "cashier_id", label: "Kasir", format: "text" },
          { id: "name", label: "Barang", format: "text" },
          { id: "barcode", label: "Barcode", format: "text" },
          { id: "brand", label: "Merek", format: "text" },
          { id: "qty", label: "Qty", format: "qty" },
          { id: "price", label: "Harga", format: "money" },
          { id: "discount", label: "Diskon", format: "money" },
          { id: "net", label: "Total", format: "money" },
        ],
      },
    ],
  },
  {
    id: "item-daily",
    label: "Laporan Item Per Hari",
    category: "item",
    shape: "report",
    defaultPaper: "a4",
    scalars: REPORT_HEADER_SCALARS,
    datasets: [
      {
        id: "rows",
        label: "Per Hari",
        fields: [
          { id: "day", label: "Tanggal", format: "text" },
          { id: "qty", label: "Qty", format: "qty" },
          { id: "gross", label: "Gross", format: "money" },
          { id: "discount", label: "Diskon", format: "money" },
          { id: "net", label: "Total", format: "money" },
        ],
      },
    ],
  },
);

export function findKind(id: string): ReportKind | undefined {
  return REPORT_KINDS.find((k) => k.id === id);
}

/** Peta field-id → format, untuk formatting nilai di renderer. */
export function fieldFormats(kind: ReportKind): Record<string, FieldFormat> {
  const map: Record<string, FieldFormat> = {};
  for (const f of kind.scalars) map[f.id] = f.format;
  for (const ds of kind.datasets) for (const f of ds.fields) map[`${ds.id}.${f.id}`] = f.format;
  return map;
}

/** Konteks contoh (data dummy) untuk pratinjau di editor tanpa transaksi nyata. */
export function sampleContext(kind: string): ReportContext {
  const header = {
    title: findKind(kind)?.label ?? "Laporan",
    subtitle: "1–30 September 2026",
    meta: "Data contoh",
  };
  if (kind === "recap-penjualan") {
    return {
      kind,
      ...header,
      scalars: header,
      datasets: {
        ringkasan: [
          { label: "Transaksi", value: "128" },
          { label: "Total Penjualan", value: "Rp12.450.000" },
          { label: "Total Diskon", value: "Rp350.000" },
          { label: "Rata-rata", value: "Rp97.266" },
        ],
        laba_rugi: [
          { label: "HPP (Modal Barang)", value: "Rp8.100.000" },
          { label: "Laba Kotor", value: "Rp4.000.000" },
          { label: "Pengeluaran Operasional", value: "-Rp600.000" },
          { label: "Laba Bersih", value: "Rp3.400.000" },
        ],
        per_barang: [
          { name: "Kopi Sachet", brand: "Kapal Api", qty: 240, discount: 0, net: 1200000 },
          { name: "Roti Tawar", brand: "Sari Roti", qty: 60, discount: 50000, net: 2050000 },
        ],
        per_merek: [
          { brand: "Kapal Api", qty: 240, discount: 0, net: 1200000 },
          { brand: "Sari Roti", qty: 60, discount: 50000, net: 2050000 },
        ],
        per_periode: [
          { periode: "2026-09-01", count: 40, discount: 100000, total: 3800000 },
          { periode: "2026-09-02", count: 88, discount: 250000, total: 8650000 },
        ],
        item_terlaris: [
          { name: "Kopi Sachet", qty: 240 },
          { name: "Roti Tawar", qty: 60 },
        ],
        per_metode: [
          { metode: "Tunai", count: 90, total: 8000000 },
          { metode: "QRIS", count: 38, total: 4450000 },
        ],
        per_kasir: [
          { kasir: "admin", count: 100, total: 9800000 },
          { kasir: "kasir1", count: 28, total: 2650000 },
        ],
      },
    };
  }
  if (kind === "recap-item") {
    return {
      kind,
      ...header,
      scalars: header,
      datasets: {
        per_barang: [
          { name: "Kopi Sachet", brand: "Kapal Api", qty: 240, discount: 0, net: 1200000 },
          { name: "Roti Tawar", brand: "Sari Roti", qty: 60, discount: 50000, net: 2050000 },
        ],
        per_merek: [
          { brand: "Kapal Api", qty: 240, discount: 0, net: 1200000 },
          { brand: "Sari Roti", qty: 60, discount: 50000, net: 2050000 },
        ],
      },
    };
  }
  if (kind === "kasir-detail") {
    return {
      kind,
      ...header,
      scalars: header,
      datasets: {
        ringkasan: [
          { label: "Total Transaksi", value: "128" },
          { label: "Total", value: "Rp12.450.000" },
          { label: "Pembayaran Tunai", value: "Rp8.000.000" },
          { label: "Pembayaran QRIS", value: "Rp4.450.000" },
        ],
      },
    };
  }
  if (kind === "item-detail") {
    return {
      kind,
      ...header,
      scalars: header,
      datasets: {
        rows: [
          { invoice_no: "INV-000123", created_at: "22/09/2026 10:00", cashier_id: "admin", name: "Kopi Sachet", barcode: "899000111", brand: "Kapal Api", qty: 2, price: 5000, discount: 0, net: 10000 },
          { invoice_no: "INV-000124", created_at: "22/09/2026 10:05", cashier_id: "kasir1", name: "Roti Tawar", barcode: "899000222", brand: "Sari Roti", qty: 1, price: 35000, discount: 5000, net: 30000 },
        ],
      },
    };
  }
  if (kind === "item-daily") {
    return {
      kind,
      ...header,
      scalars: header,
      datasets: {
        rows: [
          { day: "2026-09-01", qty: 40, gross: 4000000, discount: 100000, net: 3900000 },
          { day: "2026-09-02", qty: 88, gross: 8900000, discount: 250000, net: 8650000 },
        ],
      },
    };
  }
  if (kind === "laporan-persediaan") {
    return {
      kind,
      ...header,
      scalars: header,
      datasets: {
        nilai_stok: [
          { label: "Nilai Stok (Pokok)", value: 8100000 },
          { label: "Nilai Stok (Jual)", value: 12450000 },
        ],
        pergerakan: [
          { periode: "2026-09", masuk: 500, keluar: 20, jual: 300, opname: 2 },
          { periode: "2026-08", masuk: 320, keluar: 10, jual: 210, opname: 1 },
        ],
        rekap_barang: [
          { name: "Kopi Sachet", masuk: 500, keluar: 20, jual: 300 },
          { name: "Roti Tawar", masuk: 120, keluar: 5, jual: 60 },
        ],
      },
    };
  }
  const base = {
    store_name: "GALAXYAS POS",
    address: "Jl. Contoh No. 1\nKota Contoh",
    phone: "0812-0000-0000",
    tax_id: "00.000.000.0-000.000",
    social: "IG: @galaxyas · WA: 0812",
    header: "Terima kasih telah berbelanja",
    footer: "Barang yang sudah dibeli tidak dapat ditukar",
  };
  if (kind === "struk-transaksi") {
    return {
      kind,
      title: base.store_name,
      scalars: {
        ...base,
        invoice_no: "INV-000123",
        date: new Date().toLocaleString("id-ID"),
        total_item: 3,
        subtotal: 45000,
        discount: 5000,
        total: 40000,
        payment_method: "Tunai",
        paid: 50000,
        change: 10000,
      },
      datasets: {
        items: [
          { name: "Kopi Sachet", qty: 2, price: 5000, discount: 0, line_total: 10000 },
          { name: "Roti Tawar", qty: 1, price: 35000, discount: 5000, line_total: 35000 },
        ],
      },
    };
  }
  // Dokumen stok (masuk/keluar)
  return {
    kind,
    title: kind === "dok-item-masuk" ? "ITEM MASUK" : "ITEM KELUAR",
    scalars: {
      ...base,
      no: "DOC-000045",
      date: new Date().toLocaleString("id-ID"),
      total_item: 2,
      total_qty: 12,
      note: "Contoh catatan",
      user_id: "admin",
    },
    datasets: {
      items: [
        { product_name: "Kopi Sachet", qty: 10, note: "" },
        { product_name: "Roti Tawar", qty: 2, note: "kadaluarsa dekat" },
      ],
    },
  };
}
