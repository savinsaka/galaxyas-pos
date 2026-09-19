<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { printElement, printElementPdf } from "$lib/print";
  import { formatQty, formatDateTime } from "$lib/format";
  import { toastError } from "$lib/toast";
  import { currentMonthBounds } from "$lib/monthPager";
  import type { StockMovementBatch } from "$lib/types";
  import MonthPager from "$lib/components/MonthPager.svelte";

  let { tabId }: { tabId?: string } = $props();

  let all = $state<StockMovementBatch[]>([]);
  let from = $state("");
  let to = $state("");

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

  const perToko = $derived.by(() => {
    const map = new Map<string, { pengiriman: number; item: number; qty: number }>();
    for (const s of shipments) {
      const cur = map.get(s.k.store) ?? { pengiriman: 0, item: 0, qty: 0 };
      cur.pengiriman += 1;
      cur.item += s.batch.item_count;
      cur.qty += s.batch.total_qty;
      map.set(s.k.store, cur);
    }
    return [...map.entries()].map(([store, v]) => ({ store, ...v })).sort((a, b) => b.qty - a.qty);
  });

  const totalKirim = $derived(shipments.length);
  const totalQty = $derived(shipments.reduce((s, x) => s + x.batch.total_qty, 0));

  async function load() {
    try {
      const page = await api.listStockMovementBatches("out", from, to, 1000, 0);
      all = page.items;
    } catch (e) { toastError(e); }
  }

  onMount(() => {
    [from, to] = currentMonthBounds();
    load();
  });

  const periodLabel = $derived(
    from && to
      ? `${new Date(from).toLocaleDateString("id-ID", { day: "2-digit", month: "short", year: "numeric" })} – ${new Date(to).toLocaleDateString("id-ID", { day: "2-digit", month: "short", year: "numeric" })}`
      : "",
  );
</script>

<div id="printable-page">
  <div class="page-head">
    <div>
      <h1>Laporan Pengiriman</h1>
      <div class="text-dim print-period" style="font-size:0.8rem;">{periodLabel}</div>
    </div>
    <div class="row no-print" style="gap:0.4rem; align-items:center;">
      <MonthPager bind:from bind:to onchange={load} />
      <button onclick={() => printElement("printable-page", "Laporan Pengiriman")}>🖨️ Print</button>
      <button onclick={() => printElementPdf("printable-page", "Laporan Pengiriman")} title="Simpan sebagai PDF — ukuran kertas mengikuti isi">📄 Export PDF</button>
    </div>
  </div>

  <div class="card" style="margin-bottom:0.9rem;">
    <div style="padding:0.6rem 0.8rem;"><b>Ringkasan per Toko</b></div>
    <table>
      <thead>
        <tr><th>Toko Tujuan</th><th class="text-right">Pengiriman</th><th class="text-right">Total Item</th><th class="text-right">Total Qty</th></tr>
      </thead>
      <tbody>
        {#each perToko as r (r.store)}
          <tr>
            <td>{r.store}</td>
            <td class="text-right mono">{r.pengiriman}</td>
            <td class="text-right mono">{r.item}</td>
            <td class="text-right mono">{formatQty(r.qty)}</td>
          </tr>
        {:else}
          <tr><td colspan="4" class="text-dim">Tidak ada pengiriman pada periode ini.</td></tr>
        {/each}
      </tbody>
      {#if perToko.length}
        <tfoot>
          <tr>
            <td class="fw-bold">Total ({totalKirim} pengiriman)</td>
            <td class="text-right mono fw-bold">{totalKirim}</td>
            <td class="text-right mono fw-bold">{perToko.reduce((s, r) => s + r.item, 0)}</td>
            <td class="text-right mono fw-bold">{formatQty(totalQty)}</td>
          </tr>
        </tfoot>
      {/if}
    </table>
  </div>

  <div class="card">
    <div style="padding:0.6rem 0.8rem;"><b>Rincian Pengiriman</b></div>
    <table>
      <thead>
        <tr><th>Waktu</th><th>No.</th><th>Toko</th><th class="text-right">Item</th><th class="text-right">Qty</th><th>Catatan</th></tr>
      </thead>
      <tbody>
        {#each shipments as s (s.batch.id)}
          <tr>
            <td class="mono" style="white-space:nowrap;">{formatDateTime(s.batch.created_at)}</td>
            <td class="mono">{s.batch.no}</td>
            <td>{s.k.store}</td>
            <td class="text-right mono">{s.batch.item_count}</td>
            <td class="text-right mono">{formatQty(s.batch.total_qty)}</td>
            <td class="text-dim">{s.k.extra || "—"}</td>
          </tr>
        {:else}
          <tr><td colspan="6" class="text-dim">Tidak ada pengiriman.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .page-head { display: flex; align-items: flex-start; justify-content: space-between; flex-wrap: wrap; gap: 0.6rem; }
  table { width: 100%; border-collapse: collapse; font-size: 0.86rem; }
  thead th {
    background: var(--baby-blue-bg); padding: 0.45rem 0.6rem; text-align: left;
    font-size: 0.72rem; font-weight: 650; color: var(--text-dim);
    text-transform: uppercase; letter-spacing: 0.04em; border-bottom: 1px solid var(--border);
  }
  tbody td, tfoot td { padding: 0.4rem 0.6rem; border-bottom: 1px solid var(--border); }
  .text-right { text-align: right; }
  .fw-bold { font-weight: 700; }
  tfoot td { border-top: 2px solid var(--border); }
</style>
