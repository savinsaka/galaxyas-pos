import type { StockMovementBatchDetail, TransactionDetail } from "./types";
import { paperCols, type ReceiptConfig } from "./receipt";

const formatQty = (n: number) => (Number.isInteger(n) ? n.toString() : n.toFixed(2));

const ESC = 0x1b;
const GS = 0x1d;

function money(n: number): string {
  return "Rp" + Math.round(n || 0).toLocaleString("id-ID");
}

/** Printer thermal umumnya CP437/ASCII, bukan UTF-8 — buang diakritik, ganti sisanya dengan '?'. */
function asciiBytes(s: string): number[] {
  const norm = s.normalize("NFKD").replace(/[̀-ͯ]/g, "");
  const out: number[] = [];
  for (const ch of norm) {
    const code = ch.codePointAt(0) ?? 63;
    out.push(code < 128 ? code : 63);
  }
  return out;
}

class EscPosBuilder {
  private bytes: number[] = [];
  raw(...b: number[]): this {
    this.bytes.push(...b);
    return this;
  }
  text(s: string): this {
    this.bytes.push(...asciiBytes(s));
    return this;
  }
  line(s = ""): this {
    return this.text(s).raw(0x0a);
  }
  init(): this {
    return this.raw(ESC, 0x40);
  }
  align(a: "left" | "center" | "right"): this {
    const n = a === "center" ? 1 : a === "right" ? 2 : 0;
    return this.raw(ESC, 0x61, n);
  }
  bold(on: boolean): this {
    return this.raw(ESC, 0x45, on ? 1 : 0);
  }
  /** Double-height text (GS ! n). Makes text taller while keeping normal width. */
  doubleHeight(on: boolean): this {
    return this.raw(GS, 0x21, on ? 0x01 : 0x00);
  }
  feed(lines: number): this {
    return this.raw(ESC, 0x64, Math.max(0, Math.min(255, lines)));
  }
  /** Potong kertas. `partial=true` = potong sebagian (umum untuk struk beruntun). */
  cut(partial = true): this {
    return this.raw(GS, 0x56, partial ? 1 : 0);
  }
  build(): Uint8Array {
    return new Uint8Array(this.bytes);
  }
}

/** Bangun urutan byte ESC/POS untuk struk transaksi, diakhiri feed + autocut. */
export function buildReceiptEscPos(detail: TransactionDetail, cfg: ReceiptConfig): Uint8Array {
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

  const { show } = cfg;
  const b = new EscPosBuilder();
  b.init();
  b.align("center");

  // Header — printer is already in center-align mode (ESC a 1), so do NOT
  // also pad with center(); that double-centres the text (shifts it right).
  if (show.storeName) b.bold(true).doubleHeight(true).line(cfg.storeName).doubleHeight(false).bold(false);
  if (show.address && cfg.address.trim()) cfg.address.split("\n").map((x) => x.trim()).filter(Boolean).forEach((a) => b.line(a));
  if (show.phone && cfg.phone.trim()) b.line(cfg.phone.trim());
  if (show.taxId && cfg.taxId.trim()) b.line(`NPWP: ${cfg.taxId.trim()}`);
  if (show.social) {
    const social = [
      cfg.instagram.trim() && `IG: ${cfg.instagram.trim()}`,
      cfg.tiktok.trim() && `TikTok: ${cfg.tiktok.trim()}`,
      cfg.whatsapp.trim() && `WA: ${cfg.whatsapp.trim()}`,
    ]
      .filter(Boolean)
      .join(" · ");
    if (social) b.line(social);
  }
  if (show.header) {
    cfg.header
      .split("\n")
      .map((x) => x.trim())
      .filter(Boolean)
      .forEach((h) => b.line(h));
  }
  if (show.date) b.line(new Date(detail.created_at).toLocaleString("id-ID"));
  if (show.invoiceNo) b.line(detail.invoice_no);

  if (show.items) {
    b.align("left").line(line());
    for (const it of detail.items) {
      b.line(it.name.slice(0, w));
      b.line(two(`  ${it.qty} x ${money(it.price)}`, money(it.price * it.qty)));
      if (it.discount > 0) b.line(two("  Diskon", "-" + money(it.discount)));
    }
  }

  const hasSummary = show.subtotal || show.discount || show.total || show.paymentMethod || show.change;
  if (hasSummary) {
    b.align("left").line(line());
    if (show.subtotal) b.line(two("Subtotal", money(detail.subtotal)));
    if (show.discount) b.line(two("Diskon", "-" + money(detail.discount)));
    if (show.total) b.bold(true).line(two("TOTAL", money(detail.total))).bold(false);
    if (show.paymentMethod) {
      if (detail.payment_method === "Kombinasi") {
        b.line(two("Tunai", money(detail.paid_cash ?? 0)));
        b.line(two("QRIS", money(detail.paid_qris ?? 0)));
      } else {
        b.line(two(detail.payment_method, money(detail.paid)));
      }
    }
    if (show.change) b.line(two("Kembali", money(detail.change)));
    b.line(line());
  }

  if (show.footer && cfg.footer.trim()) {
    b.align("center");
    // Footer — same as header: printer is in center-align, so no manual padding.
    cfg.footer
      .split("\n")
      .map((x) => x.trim())
      .filter(Boolean)
      .forEach((f) => b.line(f));
  }

  b.feed(3);
  b.cut(true);

  return b.build();
}

/** Bangun urutan byte ESC/POS untuk dokumen Item Masuk/Keluar, diakhiri feed + autocut. */
export function buildStockDocEscPos(detail: StockMovementBatchDetail, cfg: ReceiptConfig): Uint8Array {
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

  const verb = detail.kind === "in" ? "ITEM MASUK" : "ITEM KELUAR";
  const { show } = cfg;
  const b = new EscPosBuilder();
  b.init();
  b.align("center");

  if (show.storeName && cfg.storeName.trim()) b.bold(true).doubleHeight(true).line(cfg.storeName).doubleHeight(false).bold(false);
  b.bold(true).line(verb).bold(false);
  b.line(detail.no);
  b.line(new Date(detail.created_at).toLocaleString("id-ID"));

  b.align("left").line(line());
  for (const it of detail.items) {
    b.line(it.product_name.slice(0, w));
    b.line(two(`  Qty: ${formatQty(it.qty)}`, ""));
    if (it.note) b.line(`  Ket: ${it.note}`.slice(0, w));
  }
  b.line(line());
  b.line(two("Total Item", String(detail.items.length)));
  b.line(two("Total Qty", formatQty(detail.items.reduce((s, it) => s + it.qty, 0))));
  if (detail.note) b.line(`Catatan: ${detail.note}`.slice(0, w));
  if (detail.user_id) b.line(`Oleh: ${detail.user_id}`.slice(0, w));
  b.line(line());

  if (show.footer && cfg.footer.trim()) {
    b.align("center");
    cfg.footer
      .split("\n")
      .map((x) => x.trim())
      .filter(Boolean)
      .forEach((f) => b.line(f));
  }

  b.feed(2);
  b.cut(true);

  return b.build();
}
