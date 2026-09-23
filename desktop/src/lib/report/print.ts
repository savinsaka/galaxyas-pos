/**
 * Jembatan cetak: mengubah struk/dokumen stok menjadi byte ESC/POS lewat
 * TEMPLATE aktif bila user sudah memilihnya di Editor Laporan, jika belum tetap
 * memakai jalur bawaan (buildReceiptEscPos/buildStockDocEscPos) — jadi perilaku
 * cetak tidak berubah sampai user sengaja mengaktifkan sebuah template.
 */
import { buildReceiptEscPos, buildStockDocEscPos } from "$lib/escpos";
import { parseReceiptConfig } from "$lib/receipt";
import type { StockMovementBatchDetail, TransactionDetail } from "$lib/types";
import { renderToEscPos } from "./render";
import { trxContext, stockDocContext } from "./data";
import { hasActiveTemplate, loadActiveTemplate } from "./storage";

/** Byte ESC/POS untuk struk transaksi (template aktif → fallback bawaan). */
export async function receiptEscPos(
  detail: TransactionDetail,
  settings: Record<string, string>,
): Promise<Uint8Array> {
  if (await hasActiveTemplate("struk-transaksi")) {
    const tpl = await loadActiveTemplate("struk-transaksi");
    return renderToEscPos(tpl, trxContext(detail, settings));
  }
  return buildReceiptEscPos(detail, parseReceiptConfig(settings));
}

/** Byte ESC/POS untuk dokumen Item Masuk/Keluar (template aktif → fallback bawaan). */
export async function stockDocEscPos(
  detail: StockMovementBatchDetail,
  settings: Record<string, string>,
): Promise<Uint8Array> {
  const kind = detail.kind === "in" ? "dok-item-masuk" : "dok-item-keluar";
  if (await hasActiveTemplate(kind)) {
    const tpl = await loadActiveTemplate(kind);
    return renderToEscPos(tpl, stockDocContext(detail, settings));
  }
  return buildStockDocEscPos(detail, parseReceiptConfig(settings));
}
