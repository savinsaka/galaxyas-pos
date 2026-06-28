import type { PrinterSettings, SaleItem, Sale, Store } from "@/types";
import { isTauri } from "@/lib/ipc/client";
import { formatCurrency, formatDateTime } from "@/lib/utils";

interface ReceiptData {
  store: Store;
  printer: PrinterSettings;
  sale: Sale;
  items: SaleItem[];
}

/**
 * Print a thermal receipt. Inside Tauri this delegates to the Rust
 * `print_receipt` command (raw ESC/POS to the configured printer). In the
 * browser dev fallback it opens a print-friendly window.
 */
export async function printReceipt(data: ReceiptData): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("print_receipt", { data });
    return;
  }
  printInBrowser(data);
}

function printInBrowser({ store, printer, sale, items }: ReceiptData) {
  const width = printer.paper_width === 80 ? 320 : 240;
  const rows = items
    .map(
      (i) =>
        `<tr><td colspan="3">${i.nama_item}</td></tr>
         <tr><td>${i.qty} x ${formatCurrency(i.harga)}</td><td></td>
         <td style="text-align:right">${formatCurrency(i.subtotal)}</td></tr>`,
    )
    .join("");

  const html = `<!doctype html><html><head><meta charset="utf-8"/>
    <title>Struk ${sale.invoice_no ?? ""}</title>
    <style>
      body{font-family:monospace;width:${width}px;margin:0 auto;padding:8px;font-size:12px;color:#000}
      h3{text-align:center;margin:4px 0}
      table{width:100%;border-collapse:collapse}
      .line{border-top:1px dashed #000;margin:6px 0}
      .totals td{padding:1px 0}
      .right{text-align:right}
      .center{text-align:center}
    </style></head><body>
    <h3>${printer.header_text || store.name}</h3>
    <div class="center">${store.address ?? ""}</div>
    <div class="center">${store.phone ?? ""}</div>
    <div class="line"></div>
    <div>No: ${sale.invoice_no ?? "-"}</div>
    <div>${formatDateTime(sale.created_at)}</div>
    <div class="line"></div>
    <table>${rows}</table>
    <div class="line"></div>
    <table class="totals">
      <tr><td>Subtotal</td><td class="right">${formatCurrency(sale.subtotal)}</td></tr>
      <tr><td>Diskon</td><td class="right">${formatCurrency(sale.diskon)}</td></tr>
      <tr><td>Pajak</td><td class="right">${formatCurrency(sale.pajak)}</td></tr>
      <tr><td><b>TOTAL</b></td><td class="right"><b>${formatCurrency(sale.total)}</b></td></tr>
      <tr><td>Bayar</td><td class="right">${formatCurrency(sale.bayar)}</td></tr>
      <tr><td>Kembali</td><td class="right">${formatCurrency(sale.kembali)}</td></tr>
    </table>
    <div class="line"></div>
    <div class="center">${printer.footer_text}</div>
    </body></html>`;

  const w = window.open("", "_blank", "width=400,height=600");
  if (!w) return;
  w.document.write(html);
  w.document.close();
  w.focus();
  setTimeout(() => w.print(), 250);
}
