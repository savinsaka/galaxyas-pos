<script lang="ts">
  import { printElement } from "$lib/print";
  import SalesRecapReport from "$lib/components/SalesRecapReport.svelte";

  let from = $state("");
  let to = $state("");

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

<div id="printable-page">
<div class="page-head">
  <h1>Laporan Umum</h1>
  <div class="row no-print">
    <button onclick={() => printElement("printable-page", "Laporan Umum")}>🖨️ Print</button>
  </div>
</div>

<div class="card no-print" style="margin-bottom:1rem;">
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

<SalesRecapReport {from} {to} />
</div>
