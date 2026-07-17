<script lang="ts">
  import { get } from "svelte/store";
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import { activeTabId, closeTab } from "$lib/stores/tabs";
  import { markStockBatchesDirty } from "$lib/stores/stockBatchSignal";
  import ProductSearchPopup from "$lib/components/ProductSearchPopup.svelte";
  import type { ProductWithStock, StockKind, StockMovementBatchDetail } from "$lib/types";

  let { batchId, kind }: { batchId: string; kind: StockKind } = $props();

  const verb = kind === "in" ? "Masuk" : "Keluar";

  interface EditItem {
    product_id: string;
    name: string;
    qty: number;
    note: string;
  }

  let batch = $state<StockMovementBatchDetail | null>(null);
  let loading = $state(true);
  let items = $state<EditItem[]>([]);
  let note = $state("");
  let busy = $state(false);
  let showPicker = $state(false);

  async function load() {
    loading = true;
    try {
      const d = await api.getStockMovementBatch(batchId);
      if (!d) {
        showToast("Batch tidak ditemukan.", "error");
        batch = null;
        return;
      }
      batch = d;
      note = d.note ?? "";
      items = d.items.map((it) => ({ product_id: it.product_id, name: it.product_name, qty: it.qty, note: it.note ?? "" }));
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    batchId;
    load();
  });

  function addItem(p: ProductWithStock) {
    const ex = items.find((l) => l.product_id === p.id);
    if (ex) {
      ex.qty += 1;
      items = [...items];
    } else {
      items = [...items, { product_id: p.id, name: p.name, qty: 1, note: "" }];
    }
    showPicker = false;
  }
  function setQty(item: EditItem, qty: number) {
    item.qty = Math.max(0.01, qty);
    items = [...items];
  }
  function removeItem(id: string) {
    items = items.filter((l) => l.product_id !== id);
  }

  function batal() {
    closeTab(get(activeTabId) ?? "");
  }

  async function save() {
    if (items.length === 0) return showToast("Batch tidak boleh kosong.", "error");
    busy = true;
    try {
      await api.updateStockMovementBatch(
        batchId,
        items.map((l) => ({ product_id: l.product_id, qty: l.qty, note: l.note || null })),
        note || null,
      );
      showToast(`Item ${verb.toLowerCase()} diperbarui.`, "success");
      markStockBatchesDirty();
      closeTab(get(activeTabId) ?? "");
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="page-head">
  <h1>Edit Item {verb}{batch ? ` — ${batch.no}` : ""}</h1>
</div>

{#if loading}
  <div class="card text-dim" style="text-align:center; padding:2rem;">Memuat…</div>
{:else if !batch}
  <div class="card text-dim" style="text-align:center; padding:2rem;">Batch tidak ditemukan.</div>
{:else}
  <div class="row" style="margin-bottom:0.8rem;">
    <button onclick={() => (showPicker = true)}>🔍 Tambah Barang</button>
  </div>

  <div class="card" style="padding:0; overflow:hidden; margin-bottom:0.8rem;">
    <table>
      <thead>
        <tr>
          <th>Barang</th>
          <th style="width:120px">Jumlah {verb}</th>
          <th>Keterangan Baris</th>
          <th style="width:2rem"></th>
        </tr>
      </thead>
      <tbody>
        {#each items as item (item.product_id)}
          <tr>
            <td>{item.name}</td>
            <td>
              <input
                class="mono cell-num"
                type="number"
                min="0.01"
                step="0.01"
                value={item.qty}
                oninput={(e) => setQty(item, +e.currentTarget.value)}
              />
            </td>
            <td><input bind:value={item.note} placeholder="opsional" /></td>
            <td><button class="btn-ghost" style="color:var(--danger);" onclick={() => removeItem(item.product_id)}>✕</button></td>
          </tr>
        {:else}
          <tr><td colspan="4" class="text-dim" style="text-align:center; padding:1rem 0;">Belum ada item.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>

  <div class="card" style="margin-bottom:0.8rem;">
    <label>Keterangan Umum</label>
    <input bind:value={note} placeholder="opsional" />
  </div>

  <div class="bottom-bar">
    <button onclick={batal}>Batal</button>
    <button class="btn-primary" disabled={busy} onclick={save}>💾 Simpan Perubahan</button>
  </div>
{/if}

{#if showPicker}
  <ProductSearchPopup onClose={() => (showPicker = false)} onPick={addItem} />
{/if}

<style>
  .cell-num { width: 100%; text-align: right; padding: 0.25rem 0.4rem; }
</style>
