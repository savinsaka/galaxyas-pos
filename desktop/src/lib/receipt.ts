export interface ReceiptShowFlags {
  storeName: boolean;
  address: boolean;
  phone: boolean;
  taxId: boolean;
  social: boolean;
  header: boolean;
  invoiceNo: boolean;
  date: boolean;
  items: boolean;
  subtotal: boolean;
  discount: boolean;
  total: boolean;
  paymentMethod: boolean;
  change: boolean;
  footer: boolean;
}

export interface ReceiptConfig {
  storeName: string;
  address: string;
  phone: string;
  taxId: string;
  instagram: string;
  tiktok: string;
  whatsapp: string;
  header: string;
  footer: string;
  paper: "58" | "80";
  fontSize: number;
  lineHeight: number;
  margin: number;
  printer: string | null;
  show: ReceiptShowFlags;
}

/** Kunci setting untuk toggle blok struk. Default aktif (true) bila belum pernah diatur. */
export const RECEIPT_SHOW_KEYS: { key: keyof ReceiptShowFlags; setting: string; label: string }[] = [
  { key: "storeName", setting: "receipt_show_store_name", label: "Nama Toko" },
  { key: "address", setting: "receipt_show_address", label: "Alamat" },
  { key: "phone", setting: "receipt_show_phone", label: "Telepon" },
  { key: "taxId", setting: "receipt_show_tax_id", label: "NPWP" },
  { key: "social", setting: "receipt_show_social", label: "Sosial Media" },
  { key: "header", setting: "receipt_show_header", label: "Header Tambahan" },
  { key: "invoiceNo", setting: "receipt_show_invoice_no", label: "No. Invoice" },
  { key: "date", setting: "receipt_show_date", label: "Tanggal" },
  { key: "items", setting: "receipt_show_items", label: "Daftar Item" },
  { key: "subtotal", setting: "receipt_show_subtotal", label: "Subtotal" },
  { key: "discount", setting: "receipt_show_discount", label: "Diskon" },
  { key: "total", setting: "receipt_show_total", label: "Total" },
  { key: "paymentMethod", setting: "receipt_show_payment_method", label: "Metode Bayar" },
  { key: "change", setting: "receipt_show_change", label: "Kembalian" },
  { key: "footer", setting: "receipt_show_footer", label: "Footer" },
];

function boolSetting(s: Record<string, string>, key: string, def = true): boolean {
  const v = s[key];
  if (v === undefined || v === "") return def;
  return v !== "0";
}

export function parseReceiptConfig(s: Record<string, string>): ReceiptConfig {
  const show = {} as ReceiptShowFlags;
  for (const { key, setting } of RECEIPT_SHOW_KEYS) show[key] = boolSetting(s, setting, true);
  return {
    storeName: s.store_name || "GALAXYAS POS",
    address: s.store_address || "",
    phone: s.store_phone || "",
    taxId: s.store_tax_id || "",
    instagram: s.store_instagram || "",
    tiktok: s.store_tiktok || "",
    whatsapp: s.store_whatsapp || "",
    header: s.receipt_header || "",
    footer: s.receipt_footer || "",
    paper: s.receipt_paper === "58" ? "58" : "80",
    fontSize: Number(s.receipt_font_size) || 12,
    lineHeight: Number(s.receipt_line_height) || 1.35,
    margin: Number(s.receipt_margin) || 3,
    printer: s.receipt_printer || null,
    show,
  };
}

export const paperWidthMm = (paper: "58" | "80") => (paper === "58" ? 58 : 80);
/** Perkiraan jumlah karakter monospace per baris untuk kertas thermal. */
export const paperCols = (paper: "58" | "80") => (paper === "58" ? 32 : 48);
