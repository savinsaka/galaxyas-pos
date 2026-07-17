import { api } from "$lib/api";

export interface ReportBlockDef {
  id: string;
  label: string;
}

export interface ReportDesignConfig {
  order: string[];
  hidden: string[];
}

export interface ReportTypeDef {
  key: string;
  label: string;
  blocks: ReportBlockDef[];
}

/** Registri statis blok yang bisa disusun ulang / disembunyikan per jenis laporan (hanya laporan dengan >1 bagian yang layak diatur ulang). */
export const REPORT_TYPES: ReportTypeDef[] = [
  {
    key: "persediaan",
    label: "Laporan Persediaan",
    blocks: [
      { id: "nilai_stok", label: "Nilai Stok (Pokok / Jual)" },
      { id: "pergerakan", label: "Pergerakan per Periode" },
      { id: "rekap_barang", label: "Rekap per Barang" },
    ],
  },
  {
    key: "sales-recap",
    label: "Recap Penjualan (dipakai di Laporan Umum & Recap Laporan Penjualan)",
    blocks: [
      { id: "ringkasan", label: "Ringkasan Penjualan" },
      { id: "laba_rugi", label: "Laba / Rugi" },
      { id: "per_barang", label: "Per Barang" },
      { id: "per_merek", label: "Per Merek" },
      { id: "per_periode", label: "Per Periode" },
      { id: "item_terlaris", label: "Item Terlaris" },
      { id: "per_metode", label: "Per Metode Pembayaran" },
      { id: "per_kasir", label: "Per Kasir" },
    ],
  },
  {
    key: "item-recap",
    label: "Recap Laporan Item",
    blocks: [
      { id: "per_barang", label: "Per Barang" },
      { id: "per_merek", label: "Per Merek" },
    ],
  },
];

const settingKey = (type: string) => `report_design_${type}`;

export function defaultConfig(blocks: ReportBlockDef[]): ReportDesignConfig {
  return { order: blocks.map((b) => b.id), hidden: [] };
}

/** Buang id yang sudah tidak ada di kode, tambahkan id baru yang belum tersimpan — supaya konfigurasi lama tetap aman dipakai setelah blok berubah. */
export function normalizeConfig(cfg: ReportDesignConfig, blocks: ReportBlockDef[]): ReportDesignConfig {
  const validIds = new Set(blocks.map((b) => b.id));
  const order = cfg.order.filter((id) => validIds.has(id));
  for (const b of blocks) if (!order.includes(b.id)) order.push(b.id);
  const hidden = cfg.hidden.filter((id) => validIds.has(id));
  return { order, hidden };
}

export function parseReportDesign(settings: Record<string, string>, type: string, blocks: ReportBlockDef[]): ReportDesignConfig {
  const raw = settings[settingKey(type)];
  if (!raw) return defaultConfig(blocks);
  try {
    return normalizeConfig(JSON.parse(raw) as ReportDesignConfig, blocks);
  } catch {
    return defaultConfig(blocks);
  }
}

/** Ambil urutan+visibilitas blok laporan `type` langsung dari settings backend. */
export async function loadReportDesign(type: string, blocks: ReportBlockDef[]): Promise<ReportDesignConfig> {
  const settings = await api.getSettings();
  return parseReportDesign(settings, type, blocks);
}

export async function saveReportDesign(type: string, config: ReportDesignConfig): Promise<void> {
  await api.updateSetting(settingKey(type), JSON.stringify(config));
}

export function blockOrder(cfg: ReportDesignConfig, id: string): number {
  const i = cfg.order.indexOf(id);
  return i === -1 ? 999 : i;
}

export function blockHidden(cfg: ReportDesignConfig, id: string): boolean {
  return cfg.hidden.includes(id);
}
