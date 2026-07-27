import { api } from "$lib/api";
import { toastError } from "$lib/toast";
import type { ReportEscPosDoc, ReportEscPosRow, ReportEscPosSection } from "$lib/escpos";

/**
 * Cetak lewat window Tauri terpisah: konten di-clone + seluruh CSS aplikasi
 * diserialisasi, dititip ke backend, lalu window `print.html` mengambilnya
 * dan memanggil window.print() dari sana. Window ini punya title bar dan
 * tombol ✕ sendiri, jadi dialog print tidak pernah menutupi tombol ✕
 * window utama. (Freeze pada percobaan pertama fitur ini disebabkan command
 * `open_print_window` yang sinkron — sudah dibuat async di backend.)
 */

/** Ambil seluruh CSS aplikasi yang aktif (termasuk gaya ter-scope Svelte). */
function currentAppCss(): string {
  let out = "";
  for (const sheet of Array.from(document.styleSheets)) {
    try {
      for (const rule of Array.from(sheet.cssRules)) out += rule.cssText + "\n";
    } catch {
      // Stylesheet lintas origin — tidak ada di build lokal, lewati saja.
    }
  }
  return out;
}

async function openPrintWindow(html: string, css: string, title: string, width: number, height: number) {
  try {
    await api.openPrintWindow(html, css, title, width, height);
  } catch (e) {
    toastError(e);
  }
}

const REPORT_OVERRIDES = `
  @page { size: A4; margin: 15mm; }
  body { margin: 0; }
  #print-root {
    background: #fff !important; color: #000 !important;
    font-family: Georgia, "Times New Roman", serif; font-size: 10.5pt;
  }
  #print-root .no-print, #print-root button { display: none !important; }
  #print-root .print-header { display: block !important; margin-bottom: 8mm; }
  #print-root .print-header h1 { font-size: 16pt !important; margin: 0 0 1.5mm !important; }
  #print-root .print-header .print-subtitle { font-size: 10pt !important; color: #333 !important; margin-bottom: 1mm; }
  #print-root .print-header .print-meta { font-size: 8.5pt !important; color: #555 !important; }
  #print-root .print-header hr { border: none; border-top: 1px solid #000; margin-top: 3mm; }
  #print-root .card {
    border: 1px solid #999 !important; box-shadow: none !important;
    border-radius: 0 !important; background: #fff !important; break-inside: avoid;
  }
  #print-root h1, #print-root h2, #print-root h3 { color: #000 !important; }
  #print-root table { width: 100%; border-collapse: collapse !important; }
  #print-root th, #print-root td {
    border: 1px solid #000 !important; background: transparent !important; color: #000 !important;
    position: static !important;
  }
  #print-root thead th { background: #eee !important; }
  #print-root .text-dim { color: #444 !important; }
  #print-root .badge, #print-root .disc-badge, #print-root .stock-badge {
    border: 1px solid #000 !important; background: #fff !important; color: #000 !important; border-radius: 0 !important;
  }
`;

/** Cetak isi elemen `elementId` sebagai dokumen laporan A4 di window terpisah. */
export function printElement(elementId: string, title = "Cetak Laporan") {
  const source = document.getElementById(elementId);
  if (!source) return;
  const clone = source.cloneNode(true) as HTMLElement;
  clone.id = "print-root";
  openPrintWindow(clone.outerHTML, currentAppCss() + REPORT_OVERRIDES, title, 980, 760);
}

/** Cetak isi elemen `elementId` sebagai struk thermal (lebar kertas dalam mm) di window terpisah. */
export function printReceiptElement(elementId: string, widthMm: number, title = "Cetak Struk") {
  const source = document.getElementById(elementId);
  if (!source) return;
  const clone = source.cloneNode(true) as HTMLElement;
  clone.id = "print-root";
  const overrides = `
    @page { size: ${widthMm}mm auto; margin: 0; }
    body { margin: 0; background: #fff; }
    #content-wrap { display: flex; justify-content: center; }
    #print-root { background: #fff !important; color: #000 !important; }
  `;
  openPrintWindow(clone.outerHTML, currentAppCss() + overrides, title, widthMm >= 80 ? 460 : 380, 700);
}

/**
 * Baca tabel-tabel di dalam elemen `elementId` (kartu berisi <table>, format
 * yang dipakai semua laporan) dan petakan jadi struktur ReportEscPosDoc,
 * supaya bisa dicetak langsung ke printer thermal (bukan lewat dialog print
 * — dialog print browser/webview mencetak hasil raster yang buram di kertas
 * thermal, sama seperti struk transaksi & dokumen stok yang sudah lebih
 * dulu pakai jalur cetak-langsung ESC/POS).
 */
export function extractReportForEscPos(
  elementId: string,
  title: string,
  subtitle: string | undefined,
  meta: string,
): ReportEscPosDoc | null {
  const root = document.getElementById(elementId);
  if (!root) return null;

  const sections: ReportEscPosSection[] = [];
  root.querySelectorAll(".card").forEach((card) => {
    const table = card.querySelector("table");
    if (!table) return;

    const heading = card.querySelector(":scope > div > b")?.textContent?.trim() || undefined;
    const columns = Array.from(table.querySelectorAll("thead th")).map((th) => th.textContent?.trim() || "");

    const rows: ReportEscPosRow[] = [];
    // `tfoot` ikut dibaca supaya baris total (mis. Total Pendapatan di Laporan
    // Item Detail) juga ikut tercetak di struk thermal, bukan cuma di layar/A4.
    table.querySelectorAll("tbody tr, tfoot tr").forEach((tr) => {
      const tds = Array.from(tr.querySelectorAll("td"));
      if (tds.length <= 1) return; // baris placeholder "Tidak ada data" (colspan)
      rows.push({
        cells: tds.map((td) => td.textContent?.trim() || ""),
        bold: !!tr.closest("tfoot") || tds.some((td) => td.classList.contains("fw-bold")),
      });
    });
    if (rows.length === 0) return;

    sections.push({ heading, columns: columns.length ? columns : undefined, rows });
  });

  if (sections.length === 0) return null;
  return { title, subtitle, meta, sections };
}
