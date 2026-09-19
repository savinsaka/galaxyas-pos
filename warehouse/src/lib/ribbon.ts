import type { ModuleKey } from "$lib/types";

export interface RibbonAction {
  key: string;
  label: string;
  icon: string;
  viewKey: string;
  title: string;
  /** Fokus tab yang sudah ada (default true). Set false untuk tab yang bisa banyak. */
  singleton?: boolean;
  /** Props diteruskan ke komponen tab (mis. `section` untuk Pengaturan yang dipecah per menu). */
  props?: Record<string, unknown>;
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

// GPOS WAREHOUSE — turunan GALAXYAS POS tanpa kasir. Menu diringkas ke
// pengelolaan barang (Master + Persediaan), Pengiriman ke toko, lalu laporan
// Persediaan & Pengiriman. Fitur penjualan/kasir/pelanggan/diskon dibuang.
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
    key: "pengiriman",
    label: "Pengiriman",
    perm: "pengiriman",
    groups: [
      {
        label: "Kirim",
        actions: [
          { key: "kirim-toko", label: "Kirim ke Toko", icon: "🚚", viewKey: "kirim-toko", title: "Kirim ke Toko", singleton: true },
        ],
      },
      {
        label: "Riwayat",
        actions: [
          { key: "daftar-pengiriman", label: "Daftar Pengiriman", icon: "📋", viewKey: "daftar-pengiriman", title: "Daftar Pengiriman", singleton: true },
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
          { key: "opname-brand", label: "Opname per Merek", icon: "🗂️", viewKey: "opname-brand", title: "Opname per Merek", singleton: true },
          { key: "opname-spesial", label: "Opname Spesial", icon: "⚡", viewKey: "opname-spesial", title: "Opname Spesial", singleton: true },
          { key: "time-opname", label: "Time Opname", icon: "⏱️", viewKey: "time-opname", title: "Time Opname", singleton: true },
        ],
      },
      {
        label: "Pergerakan",
        actions: [
          { key: "daftar-item-masuk", label: "Daftar Item Masuk", icon: "📥", viewKey: "daftar-item-masuk", title: "Daftar Item Masuk", singleton: true },
          { key: "daftar-item-keluar", label: "Daftar Item Keluar", icon: "📤", viewKey: "daftar-item-keluar", title: "Daftar Item Keluar", singleton: true },
          { key: "alur-barang", label: "Alur Barang", icon: "🔀", viewKey: "alur-barang", title: "Alur Barang", singleton: true },
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
          { key: "lap-persediaan", label: "Lap. Persediaan", icon: "📈", viewKey: "laporan-persediaan", title: "Laporan Persediaan", singleton: true },
          { key: "lap-pengiriman", label: "Lap. Pengiriman", icon: "🚚", viewKey: "laporan-pengiriman", title: "Laporan Pengiriman", singleton: true },
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
          { key: "pengaturan-toko", label: "Info Gudang", icon: "🏪", viewKey: "pengaturan-toko", title: "Informasi Gudang", singleton: true, props: { section: "toko" } },
          { key: "pengaturan-server", label: "Server Sinkronisasi", icon: "🔄", viewKey: "pengaturan-server", title: "Server Sinkronisasi", singleton: true, props: { section: "server" } },
          { key: "pengaturan-struk", label: "Struk & Printer", icon: "🖨️", viewKey: "pengaturan-struk", title: "Struk & Printer", singleton: true, props: { section: "struk" } },
          { key: "pengaturan-tema", label: "Tema", icon: "🎨", viewKey: "pengaturan-tema", title: "Tema", singleton: true, props: { section: "tema" } },
          { key: "pengaturan-lanjutan", label: "Lanjutan", icon: "⚠️", viewKey: "pengaturan-lanjutan", title: "Lanjutan", singleton: true, props: { section: "lanjutan" } },
          { key: "hak-akses", label: "Hak Akses", icon: "🔐", viewKey: "hak-akses", title: "Hak Akses & Pengguna", singleton: true },
        ],
      },
    ],
  },
];
