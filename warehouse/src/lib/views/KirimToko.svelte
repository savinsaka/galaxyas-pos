<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { api } from "$lib/api";
  import { formatQty, formatTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import { createLiveClock } from "$lib/liveClock.svelte";
  import { markStockBatchesDirty } from "$lib/stores/stockBatchSignal";
  import { setTabDirty, clearTabDirty } from "$lib/stores/tabGuard";
  import { activeTabId } from "$lib/stores/tabs";
  import type { ProductWithStock, DestStore, StockMovementBatchDetail } from "$lib/types";
  import ProductSearchPopup from "$lib/components/ProductSearchPopup.svelte";
  import StockDocPrint from "$lib/components/StockDocPrint.svelte";
  import ShortcutBar from "$lib/components/ShortcutBar.svelte";

  let { tabId }: { tabId?: string } = $props();

  const clock = createLiveClock();
  onDestroy(() => clock.stop());

  interface CartRow {
    id: number;
    search: string;
    product: ProductWithStock | null;
    qty: number;
    dropOpen: boolean;
  }

  let products = $state<ProductWithStock[]>([]);
  let stores = $state<DestStore[]>([]);
  let storeId = $state("");
  let storesLoading = $state(false);
  let storesError = $state("");
  let catatan = $state("");
  let nextId = 1;
  let rows = $state<CartRow[]>([newRow()]);
  let busy = $state(false);
  let popupRow = $state<CartRow | null>(null);
  let lastSaved = $state<StockMovementBatchDetail | null>(null);
  let showPrint = $state(false);

  const selectedStore = $derived(stores.find((s) => s.id === storeId) ?? null);

  $effect(() => {
    if (tabId) setTabDirty(tabId, rows.some((r) => r.product !== null));
  });
  onDestroy(() => { if (tabId) clearTabDirty(tabId); });

  function newRow(): CartRow {
    return { id: nextId++, search: "", product: null, qty: 1, dropOpen: false };
  }

  const filtered = (row: CartRow) => {
    const q = row.search.trim().toLowerCase();
    if (!q) return [];
    return products
      .filter((p) => p.name.toLowerCase().includes(q) || (p.barcode ?? "").toLowerCase().includes(q))
      .slice(0, 8);
  };

  async function loadProducts() {
    try {
      products = await api.listProducts("", true);
    } catch (e) { toastError(e); }
  }

  /** Ambil daftar toko tujuan dari server mobile (butuh Pengaturan → Pengiriman terisi). */
  async function loadStores() {
    storesLoading = true;
    storesError = "";
    try {
      stores = await api.shipListStores();
      if (stores.length && !storeId) storeId = stores[0].id;
    } catch (e) {
      storesError = e instanceof Error ? e.message : String(e);
    } finally {
      storesLoading = false;
    }
  }

  onMount(() => {
    loadProducts();
    loadStores();
  });

  function onGlobalKey(e: KeyboardEvent) {
    if (tabId && $activeTabId !== tabId) return;
    if (e.key === "F9") {
      e.preventDefault();
      if (!busy) kirim();
    } else if (e.key === "F6") {
      e.preventDefault();
      addRow();
    }
  }
  onMount(() => window.addEventListener("keydown", onGlobalKey));
  onDestroy(() => window.removeEventListener("keydown", onGlobalKey));

  async function onRowKey(e: KeyboardEvent, row: CartRow) {
    const idx = rows.findIndex((r) => r.id === row.id);
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (idx < rows.length - 1) focusSearch(rows[idx + 1].id);
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (idx > 0) focusSearch(rows[idx - 1].id);
      return;
    }
    if (e.key === "ArrowRight") {
      const input = e.currentTarget as HTMLInputElement;
      if (input.selectionStart === input.value.length) {
        e.preventDefault();
        focusQty(row.id);
      }
      return;
    }
    if (e.key === "Delete" && !row.search.trim() && !row.product) {
      e.preventDefault();
      removeRow(row.id);
      return;
    }
    if (e.key !== "Enter") return;
    const term = row.search.trim();
    if (!term) return;
    try {
      const p = await api.findByBarcode(term);
      if (p) { selectProduct(row, p); return; }
    } catch (_) { /* skip */ }
    const f = filtered(row);
    if (f.length === 1) { selectProduct(row, f[0]); return; }
    popupRow = row;
  }

  async function focusQty(rowId: number) {
    await tick();
    const el = document.querySelector<HTMLInputElement>(`[data-qty-row="${rowId}"]`);
    el?.focus();
    el?.select();
  }
  async function focusSearch(rowId: number) {
    await tick();
    const el = document.querySelector<HTMLInputElement>(`[data-search-row="${rowId}"]`);
    el?.focus();
  }

  function selectProduct(row: CartRow, p: ProductWithStock) {
    lastSaved = null;
    row.product = p;
    row.search = p.name;
    row.dropOpen = false;
    rows = [...rows];
    focusQty(row.id);
  }

  function advanceRow(row: CartRow) {
    if (!row.product || row.qty <= 0) return;
    const idx = rows.findIndex((r) => r.id === row.id);
    if (idx === rows.length - 1) {
      const nr = newRow();
      rows = [...rows, nr];
      focusSearch(nr.id);
    } else {
      focusSearch(rows[idx + 1].id);
    }
  }

  function onQtyKey(e: KeyboardEvent, row: CartRow) {
    if (e.key === "ArrowLeft") {
      const input = e.currentTarget as HTMLInputElement;
      let atStart = true;
      try { atStart = input.selectionStart === 0; } catch (_) { /* type=number */ }
      if (atStart) { e.preventDefault(); focusSearch(row.id); }
      return;
    }
    if (e.key === "ArrowDown") { e.preventDefault(); advanceRow(row); return; }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      const idx = rows.findIndex((r) => r.id === row.id);
      if (idx > 0) focusQty(rows[idx - 1].id);
      return;
    }
    if (e.key !== "Enter") return;
    advanceRow(row);
  }

  function addRow() { rows = [...rows, newRow()]; }
  function removeRow(id: number) {
    if (rows.length <= 1) return;
    rows = rows.filter((r) => r.id !== id);
  }

  async function kirim() {
    const store = selectedStore;
    if (!store) return showToast("Pilih toko tujuan dulu.", "error");
    const valid = rows.filter((r) => r.product && r.qty > 0);
    if (!valid.length) return showToast("Tidak ada item untuk dikirim.", "error");

    const tanpaBarcode = valid.filter((r) => !(r.product!.barcode ?? "").trim());
    if (tanpaBarcode.length) {
      return showToast(
        `Ada ${tanpaBarcode.length} barang tanpa barcode (mis. "${tanpaBarcode[0].product!.name}"). Barcode wajib untuk dikirim.`,
        "error",
        6000,
      );
    }

    busy = true;
    try {
      const batch = await api.shipSend(
        store.id,
        store.name,
        valid.map((r) => ({
          product_id: r.product!.id,
          barcode: (r.product!.barcode ?? "").trim(),
          name: r.product!.name,
          qty: r.qty,
        })),
        $currentUser?.username ?? null,
        catatan || null,
      );
      showToast(`${batch.no}: ${valid.length} item terkirim ke ${store.name}. Stok gudang berkurang.`, "success", 6000);
      markStockBatchesDirty();
      lastSaved = batch;
      rows = [newRow()];
      catatan = "";
      await loadProducts();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="page-head"><h1>Kirim ke Toko</h1></div>

<!-- Header: toko tujuan + waktu + catatan -->
<div class="trx-header card">
  <div class="trx-field" style="min-width:220px;">
    <label>Toko Tujuan</label>
    {#if storesLoading}
      <span class="text-dim" style="font-size:0.85rem;">Memuat toko…</span>
    {:else if stores.length}
      <select bind:value={storeId} style="min-width:220px;">
        {#each stores as s (s.id)}<option value={s.id}>{s.name} ({s.code})</option>{/each}
      </select>
    {:else}
      <button onclick={loadStores}>🔄 Muat daftar toko</button>
    {/if}
    {#if storesError}
      <span class="store-err" title={storesError}>⚠️ Gagal memuat toko — cek Pengaturan → Pengiriman.</span>
    {/if}
  </div>
  <div class="trx-field">
    <label>Jam</label>
    <span class="meta-val mono">{formatTime(clock.now)}</span>
  </div>
  <div class="trx-field" style="flex:1;">
    <label>Catatan Pengiriman</label>
    <input bind:value={catatan} placeholder="opsional (mis. no. surat jalan)" />
  </div>
</div>

<!-- Keranjang kirim -->
<div class="card" style="padding:0; overflow:hidden; margin-bottom:0.8rem;">
  <table class="batch-table">
    <thead>
      <tr>
        <th style="width:2rem;">No</th>
        <th>Kode/Nama Item</th>
        <th style="width:60px;">Satuan</th>
        <th style="width:110px;">Stok</th>
        <th style="width:120px;">Jumlah Kirim</th>
        <th style="width:2rem;"></th>
      </tr>
    </thead>
    <tbody>
      {#each rows as row (row.id)}
        <tr>
          <td class="text-dim mono" style="text-align:center;">{rows.indexOf(row) + 1}</td>
          <td style="position:relative;">
            <input
              class="cell-input"
              data-search-row={row.id}
              placeholder="Scan barcode atau ketik nama…"
              bind:value={row.search}
              oninput={() => { row.dropOpen = true; rows = [...rows]; }}
              onkeydown={(e) => onRowKey(e, row)}
              onfocus={() => { row.dropOpen = true; rows = [...rows]; }}
              onblur={() => setTimeout(() => { row.dropOpen = false; rows = [...rows]; }, 200)}
            />
            {#if row.dropOpen && filtered(row).length > 0}
              <div class="row-drop">
                {#each filtered(row) as p (p.id)}
                  <button class="rd-row" onmousedown={() => selectProduct(row, p)}>
                    <span>{p.name}</span>
                    <span class="text-dim" style="font-size:0.78rem;">stok {formatQty(p.stock_qty)}</span>
                  </button>
                {/each}
              </div>
            {/if}
          </td>
          <td class="text-dim">{row.product?.unit ?? "—"}</td>
          <td class="text-dim mono">{row.product ? formatQty(row.product.stock_qty) : "—"}</td>
          <td>
            <input
              class="cell-input mono"
              data-qty-row={row.id}
              type="number"
              min="0.01"
              step="0.01"
              bind:value={row.qty}
              onkeydown={(e) => onQtyKey(e, row)}
            />
          </td>
          <td>
            <button class="del-btn" title="Hapus baris" onclick={() => removeRow(row.id)} disabled={rows.length <= 1}>✕</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<div class="bottom-bar">
  <button onclick={addRow}>➕ Tambah Baris</button>
  <button class="btn-primary" disabled={busy || !selectedStore} onclick={kirim}>
    🚚 Kirim ke {selectedStore ? selectedStore.name : "Toko"}
  </button>
  <button disabled={!lastSaved} onclick={() => (showPrint = true)} title={lastSaved ? "" : "Kirim dulu sebelum mencetak"}>
    🖨️ Cetak Surat Jalan
  </button>
  <span class="text-dim" style="margin-left:auto; font-size:0.82rem;">Toko menerima lewat menu "Pull" di POS-nya.</span>
</div>
<ShortcutBar items={[
  { key: "F9", label: "Kirim", action: kirim, disabled: busy || !selectedStore },
  { key: "F6", label: "Tambah Baris", action: addRow },
]} />

{#if showPrint && lastSaved}
  <StockDocPrint detail={lastSaved} onClose={() => (showPrint = false)} />
{/if}

{#if popupRow}
  {@const targetRow = popupRow}
  <ProductSearchPopup
    initialQuery={targetRow.search}
    onClose={() => (popupRow = null)}
    onPick={(p) => { selectProduct(targetRow, p); popupRow = null; }}
  />
{/if}

<style>
  .trx-header { display: flex; align-items: flex-end; gap: 1.5rem; flex-wrap: wrap; margin-bottom: 0.8rem; }
  .meta-val { font-size: 0.95rem; font-weight: 700; }
  .trx-field { display: flex; flex-direction: column; gap: 0.2rem; }
  .trx-field label { font-size: 0.78rem; color: var(--text-dim); margin: 0; }
  .store-err { font-size: 0.72rem; color: var(--danger); margin-top: 0.2rem; }

  .batch-table { width: 100%; border-collapse: collapse; }
  .batch-table thead th {
    background: var(--baby-blue-bg); padding: 0.5rem 0.6rem;
    font-size: 0.78rem; font-weight: 650; color: var(--text-dim);
    text-transform: uppercase; letter-spacing: 0.04em;
    border-bottom: 1px solid var(--border); position: sticky; top: 0;
  }
  .batch-table tbody td { padding: 0.3rem 0.5rem; border-bottom: 1px solid var(--border); vertical-align: middle; }
  .batch-table tbody tr:last-child td { border-bottom: none; }
  .cell-input { width: 100%; border: 1px solid transparent; background: transparent; padding: 0.3rem 0.4rem; border-radius: 4px; }
  .cell-input:focus { border-color: var(--primary); background: var(--white); outline: none; }

  .row-drop {
    position: absolute; left: 0; right: 0; top: 100%; z-index: 100;
    background: var(--white); border: 1px solid var(--border);
    border-radius: 0 0 var(--radius) var(--radius);
    box-shadow: var(--shadow); max-height: 200px; overflow-y: auto;
  }
  .rd-row {
    display: flex; justify-content: space-between; align-items: center;
    width: 100%; text-align: left; border: none; border-radius: 0;
    border-bottom: 1px solid var(--border); padding: 0.4rem 0.7rem;
    font-size: 0.85rem; font-weight: 500;
  }
  .rd-row:last-child { border-bottom: none; }
  .del-btn { padding: 0.15rem 0.35rem; color: var(--danger); border-color: transparent; background: transparent; }
  .del-btn:disabled { opacity: 0.2; }
</style>
