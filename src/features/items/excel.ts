import * as XLSX from "xlsx";
import { itemImportRowSchema } from "@/schemas";
import type { ItemInput, Item } from "@/types";

export interface ImportResult {
  valid: ItemInput[];
  errors: string[];
}

const COLUMN_MAP: Record<string, keyof ItemInput> = {
  kode_item: "kode_item",
  "kode item": "kode_item",
  kode: "kode_item",
  barcode: "barcode",
  nama_item: "nama_item",
  "nama item": "nama_item",
  nama: "nama_item",
  jenis: "jenis",
  merek: "merek",
  satuan_dasar: "satuan_dasar",
  satuan: "satuan_dasar",
  harga_beli: "harga_beli",
  "harga beli": "harga_beli",
  harga_jual: "harga_jual",
  "harga jual": "harga_jual",
  diskon_persen: "diskon_persen",
  diskon: "diskon_persen",
};

/** Parse and validate an .xlsx/.csv file into item inputs. */
export async function parseItemsExcel(file: File): Promise<ImportResult> {
  const buffer = await file.arrayBuffer();
  const wb = XLSX.read(buffer, { type: "array" });
  const sheet = wb.Sheets[wb.SheetNames[0]];
  const rows = XLSX.utils.sheet_to_json<Record<string, unknown>>(sheet, {
    defval: null,
  });

  const valid: ItemInput[] = [];
  const errors: string[] = [];

  rows.forEach((raw, idx) => {
    const normalized: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(raw)) {
      const mapped = COLUMN_MAP[key.trim().toLowerCase()];
      if (mapped) normalized[mapped] = value;
    }
    const parsed = itemImportRowSchema.safeParse(normalized);
    if (parsed.success) {
      valid.push(parsed.data as ItemInput);
    } else {
      const msg = parsed.error.issues
        .map((i) => `${i.path.join(".")}: ${i.message}`)
        .join("; ");
      errors.push(`Baris ${idx + 2}: ${msg}`);
    }
  });

  return { valid, errors };
}

/** Export items to an .xlsx file and trigger a browser download. */
export function exportItemsExcel(items: Item[], filename = "barang.xlsx") {
  const data = items.map((i) => ({
    kode_item: i.kode_item,
    barcode: i.barcode,
    nama_item: i.nama_item,
    jenis: i.jenis,
    merek: i.merek,
    satuan_dasar: i.satuan_dasar,
    harga_beli: i.harga_beli,
    harga_jual: i.harga_jual,
    diskon_persen: i.diskon_persen,
    stok: i.stok,
  }));
  const ws = XLSX.utils.json_to_sheet(data);
  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, ws, "Barang");
  XLSX.writeFile(wb, filename);
}

/** Download a blank import template with the expected headers. */
export function downloadImportTemplate() {
  const ws = XLSX.utils.json_to_sheet([
    {
      kode_item: "BRG0001",
      barcode: "8990000000001",
      nama_item: "Contoh Barang",
      jenis: "Makanan",
      merek: "Umum",
      satuan_dasar: "PCS",
      harga_beli: 1000,
      harga_jual: 1500,
      diskon_persen: 0,
    },
  ]);
  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, ws, "Template");
  XLSX.writeFile(wb, "template-import-barang.xlsx");
}
