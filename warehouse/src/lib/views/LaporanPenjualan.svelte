<script lang="ts">
  import PrintableReportModal from "$lib/components/PrintableReportModal.svelte";
  import SalesRecapReport from "$lib/components/SalesRecapReport.svelte";
  import KasirDetailReport from "$lib/components/KasirDetailReport.svelte";
  import { formatPeriodLabel } from "$lib/dateTime";

  let from = $state("");
  let to = $state("");
  let openModal = $state<"recap" | "kasir-detail" | null>(null);

  const dateStr = (iso: string) => {
    const d = new Date(iso);
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  };

  function presetHariIni() {
    const t = dateStr(new Date().toISOString());
    from = t;
    to = t;
  }
  function presetBulanIni() {
    const d = new Date();
    from = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-01`;
    to = dateStr(d.toISOString());
  }
  function presetTahunIni() {
    const d = new Date();
    from = `${d.getFullYear()}-01-01`;
    to = dateStr(d.toISOString());
  }
  function resetRange() {
    from = "";
    to = "";
  }

  presetBulanIni();
</script>

<div class="page-head">
  <h1>Laporan Penjualan</h1>
</div>

<div class="card" style="margin-bottom:1rem;">
  <div class="row" style="flex-wrap:wrap; gap:0.8rem;">
    <div><label>Dari</label><input type="date" bind:value={from} /></div>
    <div><label>Sampai</label><input type="date" bind:value={to} /></div>
    <div style="align-self:flex-end;" class="row">
      <button onclick={presetHariIni}>Hari Ini</button>
      <button onclick={presetBulanIni}>Bulan Ini</button>
      <button onclick={presetTahunIni}>Tahun Ini</button>
      <button onclick={resetRange}>Semua</button>
    </div>
  </div>
</div>

<div class="row" style="gap:1rem; flex-wrap:wrap;">
  <button class="report-tile" onclick={() => (openModal = "recap")}>
    <span class="rt-icon">📊</span>
    <span class="rt-label">Recap Laporan</span>
    <span class="rt-desc text-dim">Ringkasan penjualan, laba/rugi, per barang/merek/kasir/metode.</span>
  </button>
  <button class="report-tile" onclick={() => (openModal = "kasir-detail")}>
    <span class="rt-icon">🧾</span>
    <span class="rt-label">Laporan Kasir Detail</span>
    <span class="rt-desc text-dim">Satu baris per transaksi selama periode yang dipilih.</span>
  </button>
</div>

{#if openModal === "recap"}
  <PrintableReportModal title="Recap Laporan Penjualan" subtitle={formatPeriodLabel(from, to)} onClose={() => (openModal = null)}>
    {#snippet children()}
      <SalesRecapReport {from} {to} />
    {/snippet}
  </PrintableReportModal>
{:else if openModal === "kasir-detail"}
  <PrintableReportModal title="Laporan Kasir Detail" onClose={() => (openModal = null)}>
    {#snippet children()}
      <KasirDetailReport />
    {/snippet}
  </PrintableReportModal>
{/if}

<style>
  .report-tile {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.3rem;
    width: 260px;
    padding: 1.1rem 1.2rem;
    text-align: left;
    border-radius: var(--radius);
  }
  .rt-icon { font-size: 1.6rem; }
  .rt-label { font-size: 1.05rem; font-weight: 700; }
  .rt-desc { font-size: 0.8rem; font-weight: 400; }
</style>
