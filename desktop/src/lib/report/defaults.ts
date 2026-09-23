/**
 * Template default per jenis laporan — replika tampilan yang ADA sekarang, jadi
 * "yang sekarang" otomatis jadi template pertama. Elemen ditata bertumpuk
 * vertikal (full-width area isi); untuk struk urutan Y = urutan baris cetak.
 */
import {
  GREPORT_VERSION,
  contentWidthMm,
  newElId,
  paperFromPreset,
  type Align,
  type Element,
  type ItemColumn,
  type PaperPreset,
  type ReportTemplate,
  type TextSize,
} from "./template";

type Spec =
  | { type: "text"; bind?: string; value?: string; prefix?: string; align?: Align; bold?: boolean; size?: TextSize; h?: number }
  | { type: "pair"; label: string; labelBind?: string; bind: string; prefix?: string; bold?: boolean }
  | { type: "line" }
  | { type: "spacer"; lines?: number }
  | { type: "items"; source: string; heading?: string; layout?: "table" | "receipt"; showHeader?: boolean; total?: boolean; columns: ItemColumn[]; h?: number };

function specHeight(s: Spec): number {
  if (s.type === "items") return s.h ?? 40;
  if (s.type === "line") return 3;
  if (s.type === "spacer") return Math.max(1, s.lines ?? 1) * 4;
  if (s.type === "text") return s.h ?? (s.size === "lg" ? 8 : 6);
  return 6;
}

/** Tumpuk spec jadi elemen berposisi (x=0, w=lebar isi, y berurutan). */
function build(kind: string, name: string, preset: PaperPreset, specs: Spec[]): ReportTemplate {
  const paper = paperFromPreset(preset);
  const cw = contentWidthMm(paper);
  let y = 0;
  const elements: Element[] = specs.map((s) => {
    const h = specHeight(s);
    const el = { id: newElId(), x: 0, y, w: cw, h, ...s } as Element;
    y += h + 1;
    return el;
  });
  return { greport: GREPORT_VERSION, kind, name, paper, elements };
}

// ---- kolom ----
const L = (bind: string, label: string, widthPct?: number): ItemColumn => ({ bind, label, align: "left", widthPct, show: true });
const R = (bind: string, label: string, widthPct?: number): ItemColumn => ({ bind, label, align: "right", widthPct, show: true });

const trxItemColumns: ItemColumn[] = [
  L("name", "Barang", 46),
  R("qty", "Qty", 12),
  R("price", "Harga", 21),
  R("line_total", "Subtotal", 21),
  { bind: "discount", label: "Diskon", align: "right", show: false },
];
const stockItemColumns: ItemColumn[] = [L("product_name", "Barang", 60), R("qty", "Qty", 20), L("note", "Ket.", 20)];

// ---- struk & dokumen stok ----

function strukTransaksi(): ReportTemplate {
  return build("struk-transaksi", "Struk Default", "struk80", [
    { type: "text", bind: "store_name", align: "center", bold: true, size: "lg" },
    { type: "text", bind: "address", align: "center" },
    { type: "text", bind: "phone", align: "center" },
    { type: "text", bind: "tax_id", prefix: "NPWP: ", align: "center" },
    { type: "text", bind: "social", align: "center" },
    { type: "text", bind: "header", align: "center" },
    { type: "text", bind: "date", align: "center" },
    { type: "text", bind: "invoice_no", align: "center" },
    { type: "line" },
    { type: "items", source: "items", layout: "receipt", showHeader: true, columns: trxItemColumns },
    { type: "line" },
    { type: "pair", label: "Total Item", bind: "total_item" },
    { type: "pair", label: "Subtotal", bind: "subtotal" },
    { type: "pair", label: "Diskon", bind: "discount", prefix: "-" },
    { type: "pair", label: "TOTAL", bind: "total", bold: true },
    { type: "pair", label: "Bayar", labelBind: "payment_method", bind: "paid" },
    { type: "pair", label: "Kembali", bind: "change" },
    { type: "line" },
    { type: "text", bind: "footer", align: "center" },
  ]);
}

function stockDoc(kind: "dok-item-masuk" | "dok-item-keluar"): ReportTemplate {
  const verb = kind === "dok-item-masuk" ? "ITEM MASUK" : "ITEM KELUAR";
  return build(kind, `${verb} Default`, "struk80", [
    { type: "text", bind: "store_name", align: "center", bold: true, size: "lg" },
    { type: "text", value: verb, align: "center", bold: true },
    { type: "text", bind: "no", align: "center" },
    { type: "text", bind: "date", align: "center" },
    { type: "line" },
    { type: "items", source: "items", layout: "receipt", showHeader: true, columns: stockItemColumns },
    { type: "line" },
    { type: "pair", label: "Total Item", bind: "total_item" },
    { type: "pair", label: "Total Qty", bind: "total_qty" },
    { type: "text", bind: "note", prefix: "Catatan: ", align: "left" },
    { type: "text", bind: "user_id", prefix: "Oleh: ", align: "left" },
    { type: "line" },
    { type: "text", bind: "footer", align: "center" },
  ]);
}

// ---- laporan tabular (A4) ----

function reportHeader(): Spec[] {
  return [
    { type: "text", bind: "title", align: "center", bold: true, size: "lg" },
    { type: "text", bind: "subtitle", align: "center", size: "sm" },
    { type: "text", bind: "meta", align: "center", size: "sm" },
    { type: "line" },
  ];
}

function recapPenjualan(): ReportTemplate {
  return build("recap-penjualan", "Recap Penjualan Default", "a4", [
    ...reportHeader(),
    { type: "items", source: "ringkasan", heading: "Ringkasan Penjualan", showHeader: false, columns: [L("label", "Keterangan"), R("value", "Nilai")] },
    { type: "items", source: "laba_rugi", heading: "Laba / Rugi", showHeader: false, columns: [L("label", "Keterangan"), R("value", "Nilai")] },
    { type: "items", source: "per_barang", heading: "Per Barang", total: true, columns: [L("name", "Barang", 40), L("brand", "Merek", 20), R("qty", "Qty"), R("discount", "Diskon"), R("net", "Total")] },
    { type: "items", source: "per_merek", heading: "Per Merek", columns: [L("brand", "Merek", 40), R("qty", "Qty"), R("discount", "Diskon"), R("net", "Total")] },
    { type: "items", source: "per_periode", heading: "Per Periode", columns: [L("periode", "Periode", 40), R("count", "Trx"), R("discount", "Diskon"), R("total", "Total")] },
    { type: "items", source: "item_terlaris", heading: "Item Terlaris", columns: [L("name", "Barang", 70), R("qty", "Qty")] },
    { type: "items", source: "per_metode", heading: "Per Metode Pembayaran", columns: [L("metode", "Metode", 50), R("count", "Trx"), R("total", "Total")] },
    { type: "items", source: "per_kasir", heading: "Per Kasir", columns: [L("kasir", "Kasir", 50), R("count", "Trx"), R("total", "Total")] },
  ]);
}

function recapItem(): ReportTemplate {
  return build("recap-item", "Recap Item Default", "a4", [
    ...reportHeader(),
    { type: "items", source: "per_barang", heading: "Per Barang", total: true, columns: [L("name", "Barang", 40), L("brand", "Merek", 20), R("qty", "Qty"), R("discount", "Diskon"), R("net", "Total")] },
    { type: "items", source: "per_merek", heading: "Per Merek", columns: [L("brand", "Merek", 40), R("qty", "Qty"), R("discount", "Diskon"), R("net", "Total")] },
  ]);
}

function laporanPersediaan(): ReportTemplate {
  return build("laporan-persediaan", "Laporan Persediaan Default", "a4", [
    ...reportHeader(),
    { type: "items", source: "nilai_stok", heading: "Nilai Stok", showHeader: false, columns: [L("label", "Keterangan"), R("value", "Nilai")] },
    { type: "items", source: "pergerakan", heading: "Pergerakan per Periode", columns: [L("periode", "Periode", 32), R("masuk", "Masuk"), R("keluar", "Keluar"), R("jual", "Terjual"), R("opname", "Opname")] },
    { type: "items", source: "rekap_barang", heading: "Rekap per Barang", columns: [L("name", "Barang", 40), R("masuk", "Masuk"), R("keluar", "Keluar"), R("jual", "Terjual")] },
  ]);
}

function kasirDetail(): ReportTemplate {
  return build("kasir-detail", "Kasir Detail Default", "a4", [
    ...reportHeader(),
    { type: "items", source: "ringkasan", heading: "Ringkasan", showHeader: false, columns: [L("label", "Keterangan"), R("value", "Nilai")] },
  ]);
}

function itemDetail(): ReportTemplate {
  return build("item-detail", "Item Detail Default", "a4", [
    ...reportHeader(),
    { type: "items", source: "rows", heading: "Item Terjual", total: true, columns: [
      L("invoice_no", "Invoice", 12), L("created_at", "Tanggal", 14), L("cashier_id", "Kasir", 8),
      L("name", "Barang", 18), L("barcode", "Barcode", 10), L("brand", "Merek", 10),
      R("qty", "Qty"), { ...R("price", "Harga"), noTotal: true }, R("discount", "Diskon"), R("net", "Total"),
    ] },
  ]);
}

function itemDaily(): ReportTemplate {
  return build("item-daily", "Item Per Hari Default", "a4", [
    ...reportHeader(),
    { type: "items", source: "rows", heading: "Penjualan per Hari", total: true, columns: [
      L("day", "Tanggal", 40), R("qty", "Qty"), { ...R("gross", "Gross"), noTotal: true }, R("discount", "Diskon"), R("net", "Total"),
    ] },
  ]);
}

/** Template default untuk sebuah kind, atau null bila belum ada defaultnya. */
export function defaultTemplate(kind: string): ReportTemplate | null {
  switch (kind) {
    case "struk-transaksi": return strukTransaksi();
    case "dok-item-masuk": return stockDoc("dok-item-masuk");
    case "dok-item-keluar": return stockDoc("dok-item-keluar");
    case "recap-penjualan": return recapPenjualan();
    case "recap-item": return recapItem();
    case "laporan-persediaan": return laporanPersediaan();
    case "kasir-detail": return kasirDetail();
    case "item-detail": return itemDetail();
    case "item-daily": return itemDaily();
    default: return null;
  }
}
