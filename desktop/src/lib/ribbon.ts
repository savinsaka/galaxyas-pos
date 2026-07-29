import type { ModuleKey } from "$lib/types";

export interface RibbonAction {
  key: string;
  label: string;
  icon: string;
  viewKey: string;
  title: string;
  /** Fokus tab yang sudah ada (default true). Set false untuk tab yang bisa banyak (POS kasir). */
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
          { key: "daftar-pelanggan", label: "Pelanggan", icon: "🧑‍🤝‍🧑", viewKey: "daftar-pelanggan", title: "Manajemen Pelanggan", singleton: true },
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
        ],
      },
      {
        label: "Shift",
        actions: [
          { key: "shift-kasir", label: "Buka/Tutup Kasir", icon: "🕒", viewKey: "shift-kasir", title: "Manajemen Shift", singleton: true },
        ],
      },
      {
        label: "Kas",
        actions: [
          { key: "pengeluaran", label: "Pengeluaran", icon: "💸", viewKey: "pengeluaran", title: "Pengeluaran (Kas Keluar)", singleton: true },
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
          { key: "lap-penjualan", label: "Lap. Penjualan", icon: "💰", viewKey: "laporan-penjualan", title: "Laporan Penjualan", singleton: true },
          { key: "lap-persediaan", label: "Lap. Persediaan", icon: "📈", viewKey: "laporan-persediaan", title: "Laporan Persediaan", singleton: true },
          { key: "lap-item", label: "Lap. Item", icon: "🏷️", viewKey: "laporan-item", title: "Laporan Item", singleton: true },
          { key: "lap-umum", label: "Lap. Umum", icon: "📋", viewKey: "laporan-umum", title: "Laporan Umum", singleton: true },
        ],
      },
      {
        label: "Desain",
        actions: [
          { key: "desain-laporan", label: "Desain Laporan", icon: "🧩", viewKey: "desain-laporan", title: "Desain Laporan", singleton: true },
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
          { key: "pengaturan-toko", label: "Info Toko", icon: "🏪", viewKey: "pengaturan-toko", title: "Informasi Toko", singleton: true, props: { section: "toko" } },
          { key: "pengaturan-server", label: "Server Sinkronisasi", icon: "🔄", viewKey: "pengaturan-server", title: "Server Sinkronisasi", singleton: true, props: { section: "server" } },
          { key: "pengaturan-lan", label: "Server Pusat", icon: "🖧", viewKey: "pengaturan-lan", title: "Server Pusat", singleton: true, props: { section: "lan" } },
          { key: "pengaturan-struk", label: "Struk & Printer", icon: "🖨️", viewKey: "pengaturan-struk", title: "Struk & Printer", singleton: true, props: { section: "struk" } },
          { key: "pengaturan-tema", label: "Tema", icon: "🎨", viewKey: "pengaturan-tema", title: "Tema", singleton: true, props: { section: "tema" } },
          { key: "pengaturan-kasir", label: "Preferensi Kasir", icon: "🧮", viewKey: "pengaturan-kasir", title: "Preferensi Kasir", singleton: true, props: { section: "kasir" } },
          { key: "pengaturan-lanjutan", label: "Lanjutan", icon: "⚠️", viewKey: "pengaturan-lanjutan", title: "Lanjutan", singleton: true, props: { section: "lanjutan" } },
          { key: "hak-akses", label: "Hak Akses", icon: "🔐", viewKey: "hak-akses", title: "Hak Akses & Pengguna", singleton: true },
        ],
      },
    ],
  },
];
