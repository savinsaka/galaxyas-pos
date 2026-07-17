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

  let batches = $state<StockMovementBatch[]>([]);
  let selectedId = $state<string | null>(null);
  let from = $state("");
  let to = $state("");

  const selected = $derived(batches.find((b) => b.id === selectedId) ?? null);

  async function load() {
    try {
      batches = await api.listStockMovementBatches(kind as "in" | "out", from, to, 500);
    } catch (e) {
      toastError(e);
    }
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
    load();
  });

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

<div class="page-head"><h1>{title}</h1></div>

<div class="card" style="margin-bottom:0.8rem;">
  <MonthPager bind:from bind:to onchange={load} />
</div>

<div class="card" style="padding:0; overflow:hidden;">
  <div style="max-height:calc(100vh - 360px); overflow:auto;">
    <table>
      <thead>
        <tr>
          <th>No Transaksi</th><th>Waktu</th><th class="text-right">Item</th>
          <th class="text-right">Qty</th><th>Catatan</th><th>Oleh</th>
        </tr>
      </thead>
      <tbody>
        {#each batches as b (b.id)}
          <tr onclick={() => (selectedId = b.id)} style={selectedId === b.id ? "background:var(--baby-blue-soft);" : ""}>
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
</div>

<div class="bottom-bar">
  {#if $currentUser?.role === "admin"}
    <button onclick={edit} disabled={!selected}>✏️ Edit</button>
  {/if}
  <button class="btn-danger" onclick={hapus} disabled={!selected}>🗑️ Hapus</button>
</div>
