<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { toastError } from "$lib/toast";
  import { formatPeriodLabel } from "$lib/dateTime";
  import type { Brand } from "$lib/types";
  import PrintableReportModal from "$lib/components/PrintableReportModal.svelte";
  import ItemRecapReport from "$lib/components/ItemRecapReport.svelte";
  import ItemDetailReport from "$lib/components/ItemDetailReport.svelte";
  import ItemDailyReport from "$lib/components/ItemDailyReport.svelte";
  import BrandMultiSelect from "$lib/components/BrandMultiSelect.svelte";

  let from = $state("");
  let to = $state("");
  let allBrands = $state<Brand[]>([]);
  let selectedBrands = $state<Set<string>>(new Set());
  let openModal = $state<"recap" | "detail" | "daily" | null>(null);

  const brandFilter = $derived([...selectedBrands]);
  const periodSubtitle = $derived(
    selectedBrands.size ? `${formatPeriodLabel(from, to)} · Merek: ${[...selectedBrands].join(", ")}` : formatPeriodLabel(from, to),
  );

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

  onMount(async () => {
    try {
      allBrands = await api.listBrands();
    } catch (e) {
      toastError(e);
    }
  });
</script>

<div class="page-head">
  <h1>Laporan Item</h1>
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

<div class="card" style="margin-bottom:1rem;">
  <div class="text-dim" style="font-size:0.8rem; margin-bottom:0.4rem;">
    Filter Merek (kosongkan untuk semua merek)
  </div>
  <BrandMultiSelect {allBrands} bind:selected={selectedBrands} />
</div>

<div class="row" style="gap:1rem; flex-wrap:wrap;">
  <button class="report-tile" onclick={() => (openModal = "recap")}>
    <span class="rt-icon">📦</span>
    <span class="rt-label">Recap Laporan Item</span>
    <span class="rt-desc text-dim">Rekap penjualan per barang dan per merek.</span>
  </button>
  <button class="report-tile" onclick={() => (openModal = "detail")}>
    <span class="rt-icon">🧾</span>
    <span class="rt-label">Laporan Item Detail</span>
    <span class="rt-desc text-dim">Satu baris per item terjual selama periode.</span>
  </button>
  <button class="report-tile" onclick={() => (openModal = "daily")}>
    <span class="rt-icon">📅</span>
    <span class="rt-label">Laporan Item Per Hari</span>
    <span class="rt-desc text-dim">Agregasi qty & nilai penjualan item per tanggal.</span>
  </button>
</div>

{#if openModal === "recap"}
  <PrintableReportModal title="Recap Laporan Item" subtitle={periodSubtitle} onClose={() => (openModal = null)}>
    {#snippet children()}
      <ItemRecapReport {from} {to} brands={brandFilter} />
    {/snippet}
  </PrintableReportModal>
{:else if openModal === "detail"}
  <PrintableReportModal title="Laporan Item Detail" subtitle={periodSubtitle} onClose={() => (openModal = null)}>
    {#snippet children()}
      <ItemDetailReport {from} {to} brands={brandFilter} />
    {/snippet}
  </PrintableReportModal>
{:else if openModal === "daily"}
  <PrintableReportModal title="Laporan Item Per Hari" subtitle={periodSubtitle} onClose={() => (openModal = null)}>
    {#snippet children()}
      <ItemDailyReport {from} {to} brands={brandFilter} />
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
