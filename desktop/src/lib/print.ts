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

// Semua gaya cetak laporan KECUALI aturan @page — ukuran halaman dipilih per
// mode: A4 untuk Print biasa, atau halaman panjang custom untuk Export PDF.
const REPORT_BODY_CSS = `
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
  const css = `@page { size: A4; margin: 15mm; }\n` + REPORT_BODY_CSS;
  openPrintWindow(clone.outerHTML, currentAppCss() + css, title, 980, 760);
}

// Export PDF = SATU halaman panjang yang ukurannya (lebar & tinggi) mengikuti
// isi laporan — tidak terpaku A4/kertas yang tersedia di dialog. Lantai lebar
// dipakai supaya laporan sempit tetap seukuran A4 (tidak melebar sia-sia),
// sedangkan laporan berkolom banyak melebar otomatis biar kolom kanan tidak
// terpotong.
const PDF_MARGIN_MM = 15;
const PDF_MIN_CONTENT_MM = 210 - PDF_MARGIN_MM * 2; // area isi A4 = 180mm
const PX_PER_MM = 96 / 25.4;

/** Bikin klon laporan (id print-root) di dalam holder tak terlihat, lalu
 * jalankan `measure` untuk membaca dimensinya memakai CSS cetak yang sama. */
function withOffscreenClone(elementId: string, holderWidth: string, measure: (clone: HTMLElement) => number): number {
  const source = document.getElementById(elementId);
  if (!source) return 0;
  const clone = source.cloneNode(true) as HTMLElement;
  clone.id = "print-root";
  const holder = document.createElement("div");
  holder.setAttribute(
    "style",
    `position:fixed; left:-100000px; top:0; width:${holderWidth}; visibility:hidden; pointer-events:none; z-index:-1;`,
  );
  const style = document.createElement("style");
  style.textContent = REPORT_BODY_CSS;
  holder.appendChild(style);
  holder.appendChild(clone);
  document.body.appendChild(holder);
  const result = measure(clone);
  document.body.removeChild(holder);
  return result;
}

/** Lebar minimum (mm) supaya tidak ada kolom yang terpotong: holder dipaksa
 * 1px sehingga tabel menyusut ke min-content, lalu overflow-nya diukur. */
function measureNaturalWidthMm(elementId: string): number {
  return withOffscreenClone(elementId, "1px", (clone) => clone.scrollWidth) / PX_PER_MM;
}

/** Tinggi laporan (mm) kalau dirender pada lebar cetak `contentWidthMm`. */
function measureReportHeightMm(elementId: string, contentWidthMm: number): number {
  const widthPx = Math.round(contentWidthMm * PX_PER_MM);
  return withOffscreenClone(elementId, `${widthPx}px`, (clone) => clone.scrollHeight) / PX_PER_MM;
}

/**
 * Export laporan `elementId` sebagai PDF: buka preview cetak yang halamannya
 * sudah dipatok jadi satu lembar panjang custom (lebar & tinggi mengikuti isi),
 * lalu user pilih "Save as PDF". Beda dari printElement (A4) yang ukurannya
 * terpaku dan bisa memotong laporan lebar/panjang.
 */
export function printElementPdf(elementId: string, title = "Export PDF Laporan") {
  const source = document.getElementById(elementId);
  if (!source) return;
  // Lebar = max(A4, lebar yang benar-benar dibutuhkan tabel) + sedikit slack.
  const contentWidthMm = Math.max(PDF_MIN_CONTENT_MM, Math.ceil(measureNaturalWidthMm(elementId)) + 6);
  // Tinggi diukur pada lebar itu; +slack supaya baris terakhir tidak mepet.
  const heightMm = Math.ceil(measureReportHeightMm(elementId, contentWidthMm)) + PDF_MARGIN_MM * 2 + 8;
  const pageWidthMm = Math.ceil(contentWidthMm) + PDF_MARGIN_MM * 2;
  const clone = source.cloneNode(true) as HTMLElement;
  clone.id = "print-root";
  const css = `@page { size: ${pageWidthMm}mm ${heightMm}mm; margin: ${PDF_MARGIN_MM}mm; }\n` + REPORT_BODY_CSS;
  openPrintWindow(clone.outerHTML, currentAppCss() + css, title, 980, 760);
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
