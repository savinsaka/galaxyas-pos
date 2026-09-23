/**
 * Pemuatan konteks data nyata untuk renderer (memakai api.ts + settings toko).
 * Dipisah dari `kinds.ts` supaya skema field tetap murni & bisa diuji/di-render
 * tanpa dependensi Tauri.
 */
import { api } from "$lib/api";
import { parseReceiptConfig } from "$lib/receipt";
import type { TransactionDetail, StockMovementBatchDetail } from "$lib/types";
import { findKind, sampleContext, type ReportContext } from "./kinds";

/** Field toko/struk bersama (nama, alamat, sosial, header/footer) dari settings. */
export function storeScalars(settings: Record<string, string>): Record<string, string> {
  const cfg = parseReceiptConfig(settings);
  const social = [
    cfg.instagram.trim() && `IG: ${cfg.instagram.trim()}`,
    cfg.tiktok.trim() && `TikTok: ${cfg.tiktok.trim()}`,
    cfg.whatsapp.trim() && `WA: ${cfg.whatsapp.trim()}`,
  ]
    .filter(Boolean)
    .join(" · ");
  return {
    store_name: cfg.storeName,
    address: cfg.address,
    phone: cfg.phone,
    tax_id: cfg.taxId,
    social,
    header: cfg.header,
    footer: cfg.footer,
  };
}

export function trxContext(detail: TransactionDetail, settings: Record<string, string>): ReportContext {
  const scalars = storeScalars(settings);
  const totalItem = detail.items.reduce((s, it) => s + it.qty, 0);
  return {
    kind: "struk-transaksi",
    title: scalars.store_name || "GALAXYAS POS",
    scalars: {
      ...scalars,
      invoice_no: detail.invoice_no,
      date: new Date(detail.created_at).toLocaleString("id-ID"),
      total_item: totalItem,
      subtotal: detail.subtotal,
      discount: detail.discount,
      total: detail.total,
      payment_method: detail.payment_method,
      paid: detail.paid,
      change: detail.change,
    },
    datasets: {
      items: detail.items.map((it) => ({
        name: it.name,
        qty: it.qty,
        price: it.price,
        discount: it.discount,
        line_total: it.price * it.qty,
      })),
    },
  };
}

export function stockDocContext(
  detail: StockMovementBatchDetail,
  settings: Record<string, string>,
): ReportContext {
  const scalars = storeScalars(settings);
  const kind = detail.kind === "in" ? "dok-item-masuk" : "dok-item-keluar";
  const totalQty = detail.items.reduce((s, it) => s + it.qty, 0);
  return {
    kind,
    title: detail.kind === "in" ? "ITEM MASUK" : "ITEM KELUAR",
    scalars: {
      ...scalars,
      no: detail.no,
      date: new Date(detail.created_at).toLocaleString("id-ID"),
      total_item: detail.items.length,
      total_qty: totalQty,
      note: detail.note ?? "",
      user_id: detail.user_id ?? "",
    },
    datasets: {
      items: detail.items.map((it) => ({
        product_name: it.product_name,
        qty: it.qty,
        note: it.note ?? "",
      })),
    },
  };
}

/**
 * Konteks PRATINJAU untuk editor: pakai setelan toko NYATA (nama, alamat, IG,
 * footer, dst dari settings) supaya struk default tampil seperti struk asli —
 * bukan "GALAXYAS POS" dummy. Bagian yang per-transaksi (item, total, no.
 * invoice) tetap data contoh karena tidak ada transaksi live saat mendesain.
 */
export async function previewContext(kind: string): Promise<ReportContext> {
  const base = sampleContext(kind);
  try {
    if (findKind(kind)?.shape === "receipt") {
      const store = storeScalars(await api.getSettings());
      return { ...base, title: store.store_name || base.title, scalars: { ...base.scalars, ...store } };
    }
  } catch {
    // Gagal baca settings (mis. di luar Tauri) — pakai contoh apa adanya.
  }
  return base;
}

/**
 * Muat konteks data nyata untuk sebuah kind. `params` menyediakan id sumber
 * (mis. id transaksi / batch). Melempar bila data tidak ditemukan.
 */
export async function loadContext(
  kind: string,
  params: { transactionId?: string; batchId?: string },
): Promise<ReportContext> {
  const settings = await api.getSettings();
  if (kind === "struk-transaksi") {
    if (!params.transactionId) throw new Error("transactionId wajib untuk struk transaksi");
    const detail = await api.getTransaction(params.transactionId);
    if (!detail) throw new Error("Transaksi tidak ditemukan");
    return trxContext(detail, settings);
  }
  if (kind === "dok-item-masuk" || kind === "dok-item-keluar") {
    if (!params.batchId) throw new Error("batchId wajib untuk dokumen stok");
    const detail = await api.getStockMovementBatch(params.batchId);
    if (!detail) throw new Error("Dokumen stok tidak ditemukan");
    return stockDocContext(detail, settings);
  }
  throw new Error(`Jenis laporan belum didukung: ${kind}`);
}
