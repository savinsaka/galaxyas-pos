<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatQty, formatDateTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import { currentMonthBounds } from "$lib/monthPager";
  import { openTab } from "$lib/stores/tabs";
  import { stockBatchesDirty } from "$lib/stores/stockBatchSignal";
  import MonthPager from "$lib/components/MonthPager.svelte";
  import type { StockKind, StockMovementBatch } from "$lib/types";

  let { kind, title }: { kind: StockKind; title: string } = $props();

  const verb = kind === "in" ? "Masuk" : "Keluar";
  const PAGE_SIZE = 50;

  let batches = $state<StockMovementBatch[]>([]);
  let total = $state(0);
  let page = $state(0);
  let selectedId = $state<string | null>(null);
  let from = $state("");
  let to = $state("");

  const selected = $derived(batches.find((b) => b.id === selectedId) ?? null);
  const totalPages = $derived(Math.max(1, Math.ceil(total / PAGE_SIZE)));

  async function load() {
    try {
      const res = await api.listStockMovementBatches(kind as "in" | "out", from, to, PAGE_SIZE, page * PAGE_SIZE);
      batches = res.items;
      total = res.total;
    } catch (e) {
      toastError(e);
    }
  }
  function reload() {
    page = 0;
    load();
  }
  function goPage(delta: number) {
    const next = page + delta;
    if (next < 0 || next >= totalPages) return;
    page = next;
    load();
  }
  onMount(() => {
    [from, to] = currentMonthBounds();
    load();
  });

  let firstDirtyCheck = true;
  $effect(() => {
    $stockBatchesDirty;
    if (firstDirtyCheck) {
      firstDirtyCheck = false;
      return;
    }
    reload();
  });

  function onListKey(e: KeyboardEvent) {
    const idx = batches.findIndex((b) => b.id === selectedId);
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selectedId = idx < 0 ? batches[0]?.id ?? null : (batches[idx + 1]?.id ?? selectedId);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (idx > 0) selectedId = batches[idx - 1].id;
    }
  }

  function tambah() {
    openTab({
      viewKey: kind === "in" ? "item-masuk" : "item-keluar",
      title: `Tambah Item ${verb}`,
      icon: kind === "in" ? "⬇️" : "⬆️",
      singleton: true,
    });
  }
  function edit() {
    if (!selected) return showToast("Pilih baris dulu.", "info");
    openTab({
      viewKey: "edit-stock-batch",
      title: `Edit Item ${verb} — ${selected.no}`,
      icon: "✏️",
      singleton: true,
      props: { batchId: selected.id, kind },
    });
  }
  async function hapus() {
    if (!selected) return showToast("Pilih baris dulu.", "info");
    if (!confirm(`Hapus batch ${selected.no}? Stok akan dikoreksi untuk semua barang di dalamnya.`)) return;
    try {
      await api.deleteStockMovementBatch(selected.id);
      showToast("Batch dihapus, stok dikoreksi.", "success");
      selectedId = null;
      await load();
    } catch (e) {
      toastError(e);
    }
  }
</script>

<div class="view-flex">
<div class="page-head"><h1>{title}</h1></div>

<div class="card" style="margin-bottom:0.8rem; flex-shrink:0;">
  <MonthPager bind:from bind:to onchange={reload} />
</div>

<div class="card list-card">
  <div class="list-scroll" tabindex="-1" role="grid" aria-label="{title}" onkeydown={onListKey}>
    <table>
      <thead>
        <tr>
          <th>No Transaksi</th><th>Waktu</th><th class="text-right">Item</th>
          <th class="text-right">Qty</th><th>Catatan</th><th>Oleh</th>
        </tr>
      </thead>
      <tbody>
        {#each batches as b (b.id)}
          <tr onclick={(e) => { selectedId = b.id; (e.currentTarget.closest(".list-scroll") as HTMLElement)?.focus(); }} style={selectedId === b.id ? "background:var(--baby-blue-soft);" : ""}>
            <td class="mono">{b.no}</td>
            <td>{formatDateTime(b.created_at)}</td>
            <td class="text-right mono">{b.item_count}</td>
            <td class="text-right mono">{formatQty(b.total_qty)}</td>
            <td>{b.note ?? "-"}</td>
            <td>{b.user_id ?? "-"}</td>
          </tr>
        {:else}
          <tr><td colspan="6" class="text-dim">Belum ada data.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
  <div class="pager">
    <span class="text-dim" style="font-size:0.82rem;">
      {total.toLocaleString("id-ID")} transaksi · Hal {page + 1} / {totalPages}
    </span>
    <div class="row" style="gap:0.3rem;">
      <button disabled={page === 0} onclick={() => goPage(-1)}>‹ Sebelumnya</button>
      <button disabled={page + 1 >= totalPages} onclick={() => goPage(1)}>Berikutnya ›</button>
    </div>
  </div>
</div>

<div class="bottom-bar">
  <button class="btn-primary" onclick={tambah}>➕ Tambah</button>
  {#if $currentUser?.role === "admin"}
    <button onclick={edit} disabled={!selected}>✏️ Edit</button>
  {/if}
  <button class="btn-danger" onclick={hapus} disabled={!selected}>🗑️ Hapus</button>
</div>
</div>

<style>
  .view-flex { height:100%; min-height:0; display:flex; flex-direction:column; }
  .list-card { padding:0; overflow:hidden; flex:1; min-height:0; display:flex; flex-direction:column; }
  .list-scroll { flex:1; min-height:0; overflow:auto; }

  .pager {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.5rem 0.9rem; border-top: 1px solid var(--border); flex-shrink:0;
  }
</style>
