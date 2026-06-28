import type { ModuleKey } from "$lib/types";

export interface RibbonAction {
  key: string;
  label: string;
  icon: string;
  viewKey: string;
  title: string;
  /** Fokus tab yang sudah ada (default true). Set false untuk tab yang bisa banyak (POS kasir). */
  singleton?: boolean;
}

export interface RibbonGroup {
  label: string;
  actions: RibbonAction[];
}

export interface RibbonCategory {
  key: string;
  label: string;
  perm: ModuleKey;
  groups: RibbonGroup[];
}

export const RIBBON: RibbonCategory[] = [
  {
    key: "master",
    label: "Master Data",
    perm: "master",
    groups: [
      {
        label: "Barang",
        actions: [
          { key: "data-barang", label: "Data Barang", icon: "📦", viewKey: "data-barang", title: "Data Barang", singleton: true },
          { key: "tambah-barang", label: "Tambah Barang", icon: "➕", viewKey: "tambah-barang", title: "Tambah Barang", singleton: true },
          { key: "data-sheet", label: "Data Sheet", icon: "📝", viewKey: "data-sheet", title: "Data Sheet", singleton: true },
        ],
      },
      {
        label: "Ekspor / Impor",
        actions: [
          { key: "import-export", label: "Import / Export", icon: "📤", viewKey: "import-export", title: "Import / Export Barang", singleton: true },
        ],
      },
      {
        label: "Referensi",
        actions: [
          { key: "daftar-merek", label: "Daftar Merek", icon: "🏭", viewKey: "daftar-merek", title: "Daftar Merek", singleton: true },
          { key: "diskon", label: "Diskon Periodik", icon: "🏷️", viewKey: "diskon", title: "Diskon Periodik", singleton: true },
        ],
      },
      {
        label: "Server",
        actions: [
          { key: "sync", label: "Sync In / Out", icon: "🔄", viewKey: "sync", title: "Sinkronisasi", singleton: true },
        ],
      },
    ],
  },
  {
    key: "penjualan",
    label: "Penjualan",
    perm: "penjualan",
    groups: [
      {
        label: "Kasir",
        actions: [
          { key: "daftar-kasir", label: "Daftar Kasir", icon: "🧾", viewKey: "daftar-kasir", title: "Daftar Kasir", singleton: true },
          { key: "tambah-kasir", label: "Tambah Kasir", icon: "🛒", viewKey: "kasir-pos", title: "Kasir", singleton: false },
        ],
      },
    ],
  },
  {
    key: "persediaan",
    label: "Persediaan",
    perm: "persediaan",
    groups: [
      {
        label: "Stok Opname",
        actions: [
          { key: "opname", label: "Opname", icon: "📊", viewKey: "opname", title: "Stok Opname", singleton: true },
        ],
      },
      {
        label: "Pergerakan",
        actions: [
          { key: "item-masuk", label: "Item Masuk", icon: "⬇️", viewKey: "item-masuk", title: "Daftar Item Masuk", singleton: true },
          { key: "item-keluar", label: "Item Keluar", icon: "⬆️", viewKey: "item-keluar", title: "Daftar Item Keluar", singleton: true },
        ],
      },
    ],
  },
  {
    key: "laporan",
    label: "Laporan",
    perm: "laporan",
    groups: [
      {
        label: "Laporan",
        actions: [
          { key: "lap-penjualan", label: "Lap. Penjualan", icon: "💰", viewKey: "laporan-penjualan", title: "Laporan Penjualan", singleton: true },
          { key: "lap-persediaan", label: "Lap. Persediaan", icon: "📈", viewKey: "laporan-persediaan", title: "Laporan Persediaan", singleton: true },
        ],
      },
    ],
  },
  {
    key: "pengaturan",
    label: "Pengaturan",
    perm: "pengaturan",
    groups: [
      {
        label: "Aplikasi",
        actions: [
          { key: "pengaturan", label: "Pengaturan", icon: "⚙️", viewKey: "pengaturan", title: "Pengaturan", singleton: true },
          { key: "hak-akses", label: "Hak Akses", icon: "🔐", viewKey: "hak-akses", title: "Hak Akses & Pengguna", singleton: true },
        ],
      },
    ],
  },
];
