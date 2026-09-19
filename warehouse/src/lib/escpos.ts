import type { StockMovementBatchDetail, TransactionDetail } from "./types";
import { paperCols, type CashDrawerPin, type ReceiptConfig } from "./receipt";

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

/**
 * Byte pembuka laci kasir (ESC p m t1 t2 — "drawer kick"). Laci tidak
 * tersambung ke PC, tapi ke port RJ11 di belakang printer, jadi perintahnya
 * dikirim lewat jalur cetak yang sama.
 *
 * `m` menentukan pin konektor: 0 = pin 2 (paling umum), 1 = pin 5. t1/t2 =
 * lama pulsa on/off dalam kelipatan 2 ms (25 → 50 ms, 250 → 500 ms) — nilai
 * aman yang dipakai kebanyakan laci; terlalu pendek bisa bikin solenoid tidak
 * kuat menarik. Sengaja BUKAN bagian dari buildReceiptEscPos supaya bisa
 * dipakai sendiri untuk tombol "Buka Laci" manual.
 */
export function buildDrawerKick(drawer: CashDrawerPin): Uint8Array {
  const pulse = (m: number) => [ESC, 0x70, m, 0x19, 0xfa];
  switch (drawer) {
    case "pin2": return new Uint8Array(pulse(0));
    case "pin5": return new Uint8Array(pulse(1));
    case "both": return new Uint8Array([...pulse(0), ...pulse(1)]);
    default: return new Uint8Array();
  }
}

/**
 * Sisipkan perintah buka laci di AKHIR job cetak (sesudah feed + potong), jadi
 * satu kali kirim ke printer. Sengaja di akhir, bukan di awal: struk diawali
 * `ESC @` (reset printer), dan menaruh perintah laci sebelum reset itu
 * mengandalkan perilaku printer yang tidak seragam antar merek.
 */
export function withDrawerKick(bytes: Uint8Array, drawer: CashDrawerPin): Uint8Array {
  const kick = buildDrawerKick(drawer);
  if (kick.length === 0) return bytes;
  const out = new Uint8Array(bytes.length + kick.length);
  out.set(bytes, 0);
  out.set(kick, bytes.length);
  return out;
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

  const hasSummary =
    show.totalItem || show.subtotal || show.discount || show.total || show.paymentMethod || show.change;
  if (hasSummary) {
    b.align("left").line(line());
    // Total item = jumlah semua pcs yang dibeli (bukan jumlah jenis barang).
    if (show.totalItem) {
      b.line(two("Total Item", formatQty(detail.items.reduce((s, it) => s + it.qty, 0))));
    }
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

/** Satu baris data pada tabel/ringkasan laporan yang mau dicetak sebagai struk. */
export interface ReportEscPosRow {
  cells: string[];
  bold?: boolean;
}

/** Satu kartu/tabel dalam laporan (mis. "Per Barang", ringkasan pembayaran). */
export interface ReportEscPosSection {
  heading?: string;
  /** Nama kolom (dari <thead>). Kosong/undefined untuk tabel label/nilai (2 kolom, tanpa header). */
  columns?: string[];
  rows: ReportEscPosRow[];
}

export interface ReportEscPosDoc {
  title: string;
  subtitle?: string;
  meta?: string;
  sections: ReportEscPosSection[];
}

/**
 * Bangun urutan byte ESC/POS untuk laporan generik (dipetakan dari tabel HTML
 * di layar) yang dicetak langsung ke printer thermal, diakhiri feed + autocut.
 */
export function buildReportEscPos(doc: ReportEscPosDoc, cfg: ReceiptConfig): Uint8Array {
  const w = paperCols(cfg.paper);
  const line = (ch = "-") => ch.repeat(w);
  const two = (l: string, r: string) => {
    const left = l.slice(0, Math.max(0, w - r.length - 1));
    const space = Math.max(1, w - left.length - r.length);
    return left + " ".repeat(space) + r;
  };

  const b = new EscPosBuilder();
  b.init();
  b.align("center");
  b.bold(true).doubleHeight(true).line(doc.title).doubleHeight(false).bold(false);
  if (doc.subtitle) b.line(doc.subtitle);
  if (doc.meta) b.line(doc.meta);

  for (const sec of doc.sections) {
    b.align("left").line(line());
    if (sec.heading) b.bold(true).line(sec.heading.slice(0, w)).bold(false);

    const wide = (sec.columns?.length ?? 0) > 2;
    if (wide) {
      for (const row of sec.rows) {
        const [first, ...rest] = row.cells;
        b.bold(!!row.bold).line((first ?? "").slice(0, w)).bold(false);
        rest.forEach((val, i) => {
          const header = sec.columns![i + 1] ?? "";
          if (val) b.line(two(`  ${header}`, val));
        });
      }
    } else {
      for (const row of sec.rows) {
        const [label, value] = row.cells;
        b.bold(!!row.bold);
        b.line(two(label ?? "", value ?? ""));
        b.bold(false);
      }
    }
  }
  b.align("left").line(line());

  b.align("center");
  b.feed(3);
  b.cut(true);

  return b.build();
}
