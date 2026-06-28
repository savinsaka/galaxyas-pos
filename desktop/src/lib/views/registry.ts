import type { Component } from "svelte";

import DataBarang from "./DataBarang.svelte";
import TambahBarang from "./TambahBarang.svelte";
import DataSheet from "./DataSheet.svelte";
import ImportExport from "./ImportExport.svelte";
import DiskonPeriodik from "./DiskonPeriodik.svelte";
import DaftarMerek from "./DaftarMerek.svelte";
import SyncCenter from "./SyncCenter.svelte";
import DaftarKasir from "./DaftarKasir.svelte";
import KasirPOS from "./KasirPOS.svelte";
import Opname from "./Opname.svelte";
import ItemMasuk from "./ItemMasuk.svelte";
import ItemKeluar from "./ItemKeluar.svelte";
import LaporanPenjualan from "./LaporanPenjualan.svelte";
import LaporanPersediaan from "./LaporanPersediaan.svelte";
import Pengaturan from "./Pengaturan.svelte";
import HakAkses from "./HakAkses.svelte";

export const VIEW_REGISTRY: Record<string, Component<any>> = {
  "data-barang": DataBarang,
  "tambah-barang": TambahBarang,
  "data-sheet": DataSheet,
  "import-export": ImportExport,
  diskon: DiskonPeriodik,
  "daftar-merek": DaftarMerek,
  sync: SyncCenter,
  "daftar-kasir": DaftarKasir,
  "kasir-pos": KasirPOS,
  opname: Opname,
  "item-masuk": ItemMasuk,
  "item-keluar": ItemKeluar,
  "laporan-penjualan": LaporanPenjualan,
  "laporan-persediaan": LaporanPersediaan,
  pengaturan: Pengaturan,
  "hak-akses": HakAkses,
};
