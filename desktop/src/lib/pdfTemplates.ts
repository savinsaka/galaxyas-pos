import { api } from "$lib/api";
import type { Template } from "@pdfme/common";

/**
 * Template PDF (pdfme) untuk Laporan Kasir Detail — layout tetap (tanpa
 * designer visual; drag-and-drop-nya dicoba lalu dicabut karena kurang
 * matang, tapi hasil PDF dari layout default ini sudah bagus).
 *
 * Modul pdfme di-import secara dinamis supaya bundel generator tidak
 * membebani startup aplikasi.
 */

export interface KasirDetailPdfInput {
  title: string;
  store_name: string;
  periode: string;
  dicetak: string;
  /** JSON array-of-arrays untuk schema table pdfme: [["Total Transaksi","4"], …] */
  ringkasan: string;
}

function kasirDetailTemplate(): Template {
  return {
    basePdf: { width: 210, height: 297, padding: [15, 15, 15, 15] },
    schemas: [
      [
        {
          name: "title",
          type: "text",
          content: "LAPORAN KASIR DETAIL",
          position: { x: 15, y: 15 },
          width: 180,
          height: 10,
          fontSize: 18,
          alignment: "left",
        },
        {
          name: "store_name",
          type: "text",
          content: "Nama Toko",
          position: { x: 15, y: 26 },
          width: 180,
          height: 7,
          fontSize: 12,
        },
        {
          name: "periode",
          type: "text",
          content: "Periode: -",
          position: { x: 15, y: 33 },
          width: 180,
          height: 6,
          fontSize: 10,
        },
        {
          name: "dicetak",
          type: "text",
          content: "Dicetak: -",
          position: { x: 15, y: 39 },
          width: 180,
          height: 6,
          fontSize: 9,
          fontColor: "#555555",
        },
        {
          name: "ringkasan",
          type: "table",
          position: { x: 15, y: 50 },
          width: 180,
          height: 60,
          content: JSON.stringify([
            ["Total Transaksi", "0"],
            ["Total", "Rp 0"],
            ["Pembayaran Tunai", "Rp 0"],
            ["Pembayaran QRIS", "Rp 0"],
            ["Pembayaran Kombinasi", "Rp 0"],
            ["Pembayaran Kartu", "Rp 0"],
          ]),
          showHead: true,
          head: ["Keterangan", "Nilai"],
          headWidthPercentages: [60, 40],
          tableStyles: { borderWidth: 0.3, borderColor: "#000000" },
          headStyles: {
            fontSize: 10,
            fontColor: "#000000",
            backgroundColor: "#eeeeee",
            borderWidth: { top: 0.3, right: 0.3, bottom: 0.3, left: 0.3 },
            borderColor: "#000000",
            padding: { top: 2, right: 2, bottom: 2, left: 2 },
          },
          bodyStyles: {
            fontSize: 10,
            fontColor: "#000000",
            borderWidth: { top: 0.3, right: 0.3, bottom: 0.3, left: 0.3 },
            borderColor: "#000000",
            padding: { top: 2, right: 2, bottom: 2, left: 2 },
          },
          columnStyles: { alignment: { "1": "right" } },
        },
      ],
    ],
  } as Template;
}

/** Generate PDF Laporan Kasir Detail, tulis ke file temp, kembalikan path-nya. */
export async function generateKasirDetailPdf(input: KasirDetailPdfInput): Promise<string> {
  const [{ generate }, { text, table }] = await Promise.all([import("@pdfme/generator"), import("@pdfme/schemas")]);
  const pdf = await generate({
    template: kasirDetailTemplate(),
    inputs: [{ ...input }],
    plugins: { Teks: text, Tabel: table },
  });
  return api.writeTempFile("laporan-kasir-detail.pdf", Array.from(pdf));
}
