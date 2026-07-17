import type { Component } from "svelte";

import DataBarang from "./DataBarang.svelte";
import TambahBarang from "./TambahBarang.svelte";
import DataSheet from "./DataSheet.svelte";
import ImportExport from "./ImportExport.svelte";
import DiskonPeriodik from "./DiskonPeriodik.svelte";
import DaftarMerek from "./DaftarMerek.svelte";
import SyncCenter from "./SyncCenter.svelte";
import DaftarKasir from "./DaftarKasir.svelte";
import EditKasir from "./EditKasir.svelte";
import KasirPOS from "./KasirPOS.svelte";
import Opname from "./Opname.svelte";
import ItemMasuk from "./ItemMasuk.svelte";
import ItemKeluar from "./ItemKeluar.svelte";
import DaftarItemMasuk from "./DaftarItemMasuk.svelte";
import DaftarItemKeluar from "./DaftarItemKeluar.svelte";
import EditStockBatch from "./EditStockBatch.svelte";
import LaporanPenjualan from "./LaporanPenjualan.svelte";
import LaporanPersediaan from "./LaporanPersediaan.svelte";
import LaporanUmum from "./LaporanUmum.svelte";
import LaporanItem from "./LaporanItem.svelte";
import DesainLaporan from "./DesainLaporan.svelte";
import Pengaturan from "./Pengaturan.svelte";
import HakAkses from "./HakAkses.svelte";
import DaftarPelanggan from "./DaftarPelanggan.svelte";
import Pengeluaran from "./Pengeluaran.svelte";
import ShiftKasir from "./ShiftKasir.svelte";

export const VIEW_REGISTRY: Record<string, Component<any>> = {
  "data-barang": DataBarang,
  "tambah-barang": TambahBarang,
  "data-sheet": DataSheet,
  "import-export": ImportExport,
  diskon: DiskonPeriodik,
  "daftar-merek": DaftarMerek,
  sync: SyncCenter,
  "daftar-kasir": DaftarKasir,
  "edit-kasir": EditKasir,
  "kasir-pos": KasirPOS,
  opname: Opname,
  "item-masuk": ItemMasuk,
  "item-keluar": ItemKeluar,
  "daftar-item-masuk": DaftarItemMasuk,
  "daftar-item-keluar": DaftarItemKeluar,
  "edit-stock-batch": EditStockBatch,
  "laporan-penjualan": LaporanPenjualan,
  "laporan-persediaan": LaporanPersediaan,
  "laporan-umum": LaporanUmum,
  "laporan-item": LaporanItem,
  "desain-laporan": DesainLaporan,
  pengaturan: Pengaturan,
  "hak-akses": HakAkses,
  "daftar-pelanggan": DaftarPelanggan,
  pengeluaran: Pengeluaran,
  "shift-kasir": ShiftKasir,
};
