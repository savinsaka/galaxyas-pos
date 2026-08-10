<script lang="ts">
  /**
   * Migrasi toko: seluruh isi toko yang sedang terbuka keluar-masuk lewat satu
   * berkas `.gpos`.
   *
   * Dua hal yang bentuk layar ini seluruhnya ditentukan olehnya:
   *
   * 1. **Impor mengganti, bukan menggabung.** Karena itu berkas selalu
   *    diperiksa dan ditampilkan asal-usulnya lebih dulu, dan konfirmasinya
   *    kalimat yang harus diketik ulang — bukan tombol "Ya".
   * 2. **Apa yang hilang harus dibaca sebelum, bukan sesudah.** `catatan` di
   *    kepala berkas datang dari aplikasi yang membuatnya, dan ia ditampilkan
   *    di langkah pratinjau — saat membatalkan masih ada gunanya.
   */
  import { openPath } from "@tauri-apps/plugin-opener";
  import { api } from "$lib/api";
  import { formatDateTime, formatIDR } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import type { MigrationResult, MigrationSource } from "$lib/types";

  const KONFIRMASI = "GANTI SEMUA DATA";

  let sibuk = $state(false);
  let hasilEkspor = $state<{ path: string; hasil: MigrationResult } | null>(null);

  let berkasNama = $state("");
  let berkasIsi = $state<Uint8Array | null>(null);
  let asal = $state<MigrationSource | null>(null);
  let ketikan = $state("");
  let hasilImpor = $state<MigrationResult | null>(null);

  const NAMA_APP: Record<string, string> = {
    gpos1: "GALAXYAS POS (versi 1)",
    gpos2: "GALAXYAS POS 2",
  };
  const namaApp = (kode: string) => NAMA_APP[kode] ?? kode;

  const totalBaris = (baris: Record<string, number>) =>
    Object.values(baris).reduce((a, b) => a + b, 0);

  async function ekspor() {
    sibuk = true;
    hasilEkspor = null;
    try {
      const keluar = await api.migrationExport();
      hasilEkspor = { path: keluar.path, hasil: keluar.hasil };
      showToast("Berkas migrasi dibuat.", "success", 5000);
    } catch (e) {
      toastError(e);
    } finally {
      sibuk = false;
    }
  }

  async function bukaFolder() {
    if (!hasilEkspor) return;
    try {
      await openPath(hasilEkspor.path);
    } catch (e) {
      toastError(e);
    }
  }

  async function pilihBerkas(ev: Event) {
    const input = ev.target as HTMLInputElement;
    const file = input.files?.[0];
    asal = null;
    hasilImpor = null;
    ketikan = "";
    berkasIsi = null;
    berkasNama = "";
    if (!file) return;

    sibuk = true;
    try {
      const isi = new Uint8Array(await file.arrayBuffer());
      // Diperiksa dulu, bukan langsung diimpor: berkas yang salah pilih harus
      // ditolak selagi data lama masih utuh.
      asal = await api.migrationInspect(isi);
      berkasIsi = isi;
      berkasNama = file.name;
    } catch (e) {
      toastError(e);
      input.value = "";
    } finally {
      sibuk = false;
    }
  }

  async function impor() {
    if (!berkasIsi || ketikan.trim() !== KONFIRMASI) return;
    sibuk = true;
    try {
      hasilImpor = await api.migrationImport(berkasIsi, ketikan.trim());
      asal = null;
      berkasIsi = null;
      ketikan = "";
      showToast("Data toko sudah diganti. Tutup dan buka lagi aplikasinya.", "success", 8000);
    } catch (e) {
      toastError(e);
    } finally {
      sibuk = false;
    }
  }
</script>

<div class="page-head"><h1>Migrasi Data Toko</h1></div>

<p class="text-dim" style="max-width:60rem; margin-bottom:1rem;">
  Memindahkan <strong>seluruh isi satu toko</strong> — barang, stok, riwayat
  penjualan, shift, pengeluaran, pengguna, dan pengaturannya — antara GALAXYAS
  POS dan GALAXYAS POS 2, lewat satu berkas <code>.gpos</code>. Yang ikut hanya
  <strong>toko yang sedang terbuka</strong>; kalau PC ini memegang beberapa
  toko, pindah dulu ke toko yang dimaksud lalu ulangi.
</p>

<div class="grid-2" style="align-items:start;">
  <!-- ---------------------------------------------------------------- -->
  <div class="card">
    <h2>Migrasi Keluar</h2>
    <p class="text-dim" style="font-size:0.82rem;">
      Membuat berkas <code>.gpos</code> berisi seluruh isi toko ini. Berkasnya
      hanya bisa dibuka GALAXYAS POS — bukan Excel, bukan aplikasi lain.
    </p>

    <button class="primary" onclick={ekspor} disabled={sibuk}>
      {sibuk ? "Menyusun…" : "Buat Berkas Migrasi"}
    </button>

    {#if hasilEkspor}
      <div class="hasil">
        <div class="row-antara">
          <strong>{totalBaris(hasilEkspor.hasil.baris).toLocaleString("id-ID")} baris</strong>
          <span class="text-dim">
            {(hasilEkspor.hasil.ukuran / 1024).toFixed(0)} KB
          </span>
        </div>
        <div class="jalur">{hasilEkspor.path}</div>
        <button onclick={bukaFolder}>Buka Berkasnya</button>

        <table class="rincian">
          <tbody>
            {#each Object.entries(hasilEkspor.hasil.baris) as [tabel, n] (tabel)}
              <tr><td>{tabel}</td><td class="angka">{n.toLocaleString("id-ID")}</td></tr>
            {/each}
          </tbody>
        </table>

        {#each hasilEkspor.hasil.peringatan as pesan (pesan)}
          <div class="catatan">{pesan}</div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- ---------------------------------------------------------------- -->
  <div class="card">
    <h2>Migrasi Masuk</h2>
    <div class="bahaya">
      Impor <strong>mengganti seluruh isi toko ini</strong> dengan isi berkas —
      bukan menggabungkan. Data toko yang sekarang akan dicadangkan otomatis
      sebelum diganti, tapi yang berjalan sesudahnya adalah isi berkas.
    </div>

    <input type="file" accept=".gpos" onchange={pilihBerkas} disabled={sibuk} />

    {#if asal}
      <div class="hasil">
        <h3 style="margin:0 0 0.4rem;">{berkasNama}</h3>
        <table class="rincian">
          <tbody>
            <tr><td>Dari</td><td class="angka">{namaApp(asal.app)} {asal.versiApp}</td></tr>
            <tr><td>Toko</td><td class="angka">{asal.tokoNama || "(tanpa nama)"}</td></tr>
            <tr><td>Dibuat</td><td class="angka">{formatDateTime(asal.dibuat)}</td></tr>
            <tr><td>Omzet di dalamnya</td><td class="angka">{formatIDR(asal.omzetTotal)}</td></tr>
            {#if asal.penjualanPertama}
              <tr>
                <td>Penjualan</td>
                <td class="angka">
                  {formatDateTime(asal.penjualanPertama)} — {formatDateTime(asal.penjualanTerakhir ?? asal.penjualanPertama)}
                </td>
              </tr>
            {/if}
            {#each Object.entries(asal.jumlah) as [tabel, n] (tabel)}
              <tr><td>{tabel}</td><td class="angka">{n.toLocaleString("id-ID")}</td></tr>
            {/each}
          </tbody>
        </table>

        {#each asal.catatan as pesan (pesan)}
          <div class="catatan">{pesan}</div>
        {/each}

        <label for="konfirmasi-migrasi" style="margin-top:0.8rem; display:block;">
          Ketik <code>{KONFIRMASI}</code> untuk melanjutkan
        </label>
        <div class="row">
          <input id="konfirmasi-migrasi" bind:value={ketikan} placeholder={KONFIRMASI} />
          <button
            class="danger"
            onclick={impor}
            disabled={sibuk || ketikan.trim() !== KONFIRMASI}
          >
            {sibuk ? "Mengganti…" : "Ganti Semua Data"}
          </button>
        </div>
      </div>
    {/if}

    {#if hasilImpor}
      <div class="hasil">
        <strong>Selesai — {totalBaris(hasilImpor.baris).toLocaleString("id-ID")} baris masuk.</strong>
        <table class="rincian">
          <tbody>
            {#each Object.entries(hasilImpor.baris) as [tabel, n] (tabel)}
              <tr><td>{tabel}</td><td class="angka">{n.toLocaleString("id-ID")}</td></tr>
            {/each}
          </tbody>
        </table>

        {#each hasilImpor.peringatan as pesan (pesan)}
          <div class="catatan">{pesan}</div>
        {/each}

        {#if hasilImpor.cadangan}
          <div class="catatan">
            Data lama dicadangkan di:<br /><span class="jalur">{hasilImpor.cadangan}</span>
          </div>
        {/if}

        <div class="bahaya" style="margin-top:0.6rem;">
          Tutup dan buka lagi aplikasinya supaya seluruh layar membaca data yang baru.
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .hasil {
    margin-top: 0.9rem;
    padding-top: 0.9rem;
    border-top: 1px solid var(--border);
  }
  .row-antara {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 0.4rem;
  }
  .jalur {
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 0.74rem;
    word-break: break-all;
    color: var(--text-dim);
    margin-bottom: 0.5rem;
  }
  .rincian {
    width: 100%;
    margin-top: 0.6rem;
    font-size: 0.8rem;
  }
  .rincian td {
    padding: 0.12rem 0;
  }
  .rincian .angka {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .catatan {
    margin-top: 0.6rem;
    padding: 0.5rem 0.6rem;
    border-left: 3px solid var(--warn, #d09a2c);
    background: rgba(208, 154, 44, 0.08);
    font-size: 0.8rem;
    line-height: 1.45;
  }
  .bahaya {
    margin: 0.6rem 0 0.9rem;
    padding: 0.55rem 0.7rem;
    border-left: 3px solid var(--danger, #c0392b);
    background: rgba(192, 57, 43, 0.09);
    font-size: 0.82rem;
    line-height: 1.45;
  }
</style>
