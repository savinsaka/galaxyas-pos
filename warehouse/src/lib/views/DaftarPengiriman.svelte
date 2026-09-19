<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatQty, formatDateTime } from "$lib/format";
  import { toastError } from "$lib/toast";
  import { currentMonthBounds } from "$lib/monthPager";
  import { stockBatchesDirty } from "$lib/stores/stockBatchSignal";
  import type { StockMovementBatch, StockMovementBatchDetail } from "$lib/types";
  import MonthPager from "$lib/components/MonthPager.svelte";
  import StockDocPrint from "$lib/components/StockDocPrint.svelte";

  let { tabId }: { tabId?: string } = $props();

  let all = $state<StockMovementBatch[]>([]);
  let from = $state("");
  let to = $state("");
  let detail = $state<StockMovementBatchDetail | null>(null);

  // Pengiriman = batch keluar ber-note "[kirim:<toko>]" (dibuat menu Kirim ke Toko).
  const KIRIM_RE = /^\[kirim:(.+?)\]\s*(.*)$/s;
  function parseKirim(note: string | null): { store: string; extra: string } | null {
    const m = KIRIM_RE.exec(note ?? "");
    return m ? { store: m[1], extra: m[2] ?? "" } : null;
  }

  const shipments = $derived(
    all
      .map((b) => ({ batch: b, k: parseKirim(b.note) }))
      .filter((x) => x.k !== null) as { batch: StockMovementBatch; k: { store: string; extra: string } }[],
  );

  async function load() {
    try {
      const page = await api.listStockMovementBatches("out", from, to, 500, 0);
      all = page.items;
    } catch (e) { toastError(e); }
  }

  onMount(() => {
    [from, to] = currentMonthBounds();
    load();
  });
  // Refresh otomatis kalau ada pengiriman baru dari menu Kirim ke Toko.
  $effect(() => { $stockBatchesDirty; if (from) load(); });

  async function openDetail(id: string) {
    try {
      detail = await api.getStockMovementBatch(id);
    } catch (e) { toastError(e); }
  }
</script>

<div class="view-flex">
  <div class="page-head">
    <h1>Daftar Pengiriman</h1>
    <MonthPager bind:from bind:to onchange={load} />
  </div>

  <div class="card" style="padding:0; overflow:hidden; flex:1; min-height:0; display:flex; flex-direction:column;">
    <div style="flex:1; min-height:0; overflow:auto;">
      <table class="tbl">
        <thead>
          <tr>
            <th>Waktu</th>
            <th>No.</th>
            <th>Toko Tujuan</th>
            <th class="text-right">Item</th>
            <th class="text-right">Total Qty</th>
            <th>Catatan</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each shipments as s (s.batch.id)}
            <tr>
              <td class="mono" style="white-space:nowrap;">{formatDateTime(s.batch.created_at)}</td>
              <td class="mono">{s.batch.no}</td>
              <td><span class="store-badge">{s.k.store}</span></td>
              <td class="text-right mono">{s.batch.item_count}</td>
              <td class="text-right mono">{formatQty(s.batch.total_qty)}</td>
              <td class="text-dim">{s.k.extra || "—"}</td>
              <td><button onclick={() => openDetail(s.batch.id)}>Detail</button></td>
            </tr>
          {:else}
            <tr><td colspan="7" class="text-dim" style="padding:1rem;">Belum ada pengiriman pada periode ini.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
    <div class="foot text-dim">{shipments.length.toLocaleString("id-ID")} pengiriman</div>
  </div>
</div>

{#if detail}
  <StockDocPrint {detail} onClose={() => (detail = null)} />
{/if}

<style>
  .view-flex { height: 100%; min-height: 0; display: flex; flex-direction: column; }
  .page-head { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 0.6rem; }
  .tbl { width: 100%; border-collapse: collapse; font-size: 0.86rem; }
  .tbl thead th {
    background: var(--baby-blue-bg); padding: 0.5rem 0.6rem; text-align: left;
    font-size: 0.72rem; font-weight: 650; color: var(--text-dim);
    text-transform: uppercase; letter-spacing: 0.04em;
    border-bottom: 1px solid var(--border); position: sticky; top: 0;
  }
  .tbl tbody td { padding: 0.4rem 0.6rem; border-bottom: 1px solid var(--border); vertical-align: middle; }
  .text-right { text-align: right; }
  .store-badge {
    display: inline-block; padding: 0.1rem 0.5rem; border-radius: 999px;
    background: var(--baby-blue-bg); border: 1px solid var(--border); font-weight: 600; font-size: 0.8rem;
  }
  .foot { padding: 0.5rem 0.9rem; border-top: 1px solid var(--border); font-size: 0.82rem; flex-shrink: 0; }
</style>
