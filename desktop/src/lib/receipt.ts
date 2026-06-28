import type { TransactionDetail } from "$lib/types";

export interface ReceiptConfig {
  storeName: string;
  header: string;
  footer: string;
  paper: "58" | "80";
  fontSize: number;
  lineHeight: number;
  margin: number;
  printer: string | null;
}

export function parseReceiptConfig(s: Record<string, string>): ReceiptConfig {
  return {
    storeName: s.store_name || "GALAXYAS POS",
    header: s.receipt_header || "",
    footer: s.receipt_footer || "",
    paper: s.receipt_paper === "58" ? "58" : "80",
    fontSize: Number(s.receipt_font_size) || 12,
    lineHeight: Number(s.receipt_line_height) || 1.35,
    margin: Number(s.receipt_margin) || 3,
    printer: s.receipt_printer || null,
  };
}

export const paperWidthMm = (paper: "58" | "80") => (paper === "58" ? 58 : 80);
/** Perkiraan jumlah karakter monospace per baris untuk kertas thermal. */
export const paperCols = (paper: "58" | "80") => (paper === "58" ? 32 : 48);

function money(n: number): string {
  return "Rp" + Math.round(n || 0).toLocaleString("id-ID");
}

/** Bangun teks struk polos (monospace) untuk dikirim ke printer via Out-Printer. */
export function buildReceiptText(detail: TransactionDetail, cfg: ReceiptConfig): string {
  const w = paperCols(cfg.paper);
  const line = (ch = "-") => ch.repeat(w);
  const center = (t: string) => {
    const s = t.slice(0, w);
    const pad = Math.max(0, Math.floor((w - s.length) / 2));
    return " ".repeat(pad) + s;
  };
  const two = (l: string, r: string) => {
    const left = l.slice(0, Math.max(0, w - r.length - 1));
    const space = Math.max(1, w - left.length - r.length);
    return left + " ".repeat(space) + r;
  };

  const out: string[] = [];
  out.push(center(cfg.storeName));
  cfg.header
    .split("\n")
    .map((x) => x.trim())
    .filter(Boolean)
    .forEach((h) => out.push(center(h)));
  out.push(center(new Date(detail.created_at).toLocaleString("id-ID")));
  out.push(center(detail.invoice_no));
  out.push(line());
  for (const it of detail.items) {
    out.push(it.name.slice(0, w));
    out.push(two(`  ${it.qty} x ${money(it.price)}`, money(it.price * it.qty)));
    if (it.discount > 0) out.push(two("  Diskon", "-" + money(it.discount)));
  }
  out.push(line());
  out.push(two("Subtotal", money(detail.subtotal)));
  out.push(two("Diskon", "-" + money(detail.discount)));
  out.push(two("TOTAL", money(detail.total)));
  out.push(two(detail.payment_method, money(detail.paid)));
  out.push(two("Kembali", money(detail.change)));
  out.push(line());
  cfg.footer
    .split("\n")
    .map((x) => x.trim())
    .filter(Boolean)
    .forEach((f) => out.push(center(f)));
  out.push("\n\n");
  return out.join("\n");
}
