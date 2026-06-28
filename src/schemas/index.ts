import { z } from "zod";

/** Master item form / import validation. */
export const itemSchema = z.object({
  kode_item: z.string().trim().max(50).optional().nullable(),
  barcode: z.string().trim().max(50).optional().nullable(),
  nama_item: z.string().trim().min(1, "Nama item wajib diisi").max(200),
  jenis: z.string().trim().max(100).optional().nullable(),
  merek: z.string().trim().max(100).optional().nullable(),
  satuan_dasar: z.string().trim().max(50).optional().nullable(),
  harga_beli: z.coerce.number().min(0, "Harga beli tidak boleh negatif"),
  harga_jual: z.coerce.number().min(0, "Harga jual tidak boleh negatif"),
  diskon_persen: z.coerce.number().min(0).max(100).default(0),
});

export type ItemFormValues = z.infer<typeof itemSchema>;

/** Validation for a single row coming from an Excel import. */
export const itemImportRowSchema = itemSchema.extend({
  nama_item: z.string().trim().min(1, "Nama item wajib diisi"),
  harga_beli: z.coerce.number().min(0).default(0),
  harga_jual: z.coerce.number().min(0).default(0),
});

export const loginSchema = z.object({
  username: z.string().trim().min(1, "Username wajib diisi"),
  password: z.string().min(1, "Password wajib diisi"),
});

export type LoginFormValues = z.infer<typeof loginSchema>;

export const stockMovementSchema = z.object({
  item_id: z.string().min(1, "Item wajib dipilih"),
  type: z.enum(["in", "out", "adjustment"]),
  qty: z.coerce.number().refine((n) => n !== 0, "Qty tidak boleh 0"),
  ref_doc: z.string().trim().optional().nullable(),
  note: z.string().trim().optional().nullable(),
});

export type StockMovementValues = z.infer<typeof stockMovementSchema>;

export const storeSettingsSchema = z.object({
  name: z.string().trim().min(1, "Nama toko wajib diisi"),
  address: z.string().trim().optional().nullable(),
  phone: z.string().trim().optional().nullable(),
  tax_percent: z.coerce.number().min(0).max(100),
});

export const printerSettingsSchema = z.object({
  printer_name: z.string().trim().min(1, "Nama printer wajib diisi"),
  paper_width: z.coerce.number().refine((n) => n === 58 || n === 80, {
    message: "Lebar kertas harus 58 atau 80 mm",
  }),
  header_text: z.string().trim().max(120),
  footer_text: z.string().trim().max(120),
});
