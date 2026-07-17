<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { api } from "$lib/api";
  import { formatQty, formatTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import { createLiveClock } from "$lib/liveClock.svelte";
  import { todayIso, combineDateAndTime } from "$lib/dateTime";
  import type { ProductWithStock, StockKind } from "$lib/types";
  import ProductSearchPopup from "$lib/components/ProductSearchPopup.svelte";

  let { kind, title }: { kind: StockKind; title: string } = $props();

  const verb = kind === "in" ? "Masuk" : "Keluar";

  const clock = createLiveClock();
  onDestroy(() => clock.stop());

  interface BatchRow {
    id: number;
    search: string;
    product: ProductWithStock | null;
    qty: number;
    keterangan: string;
    dropOpen: boolean;
  }

  let products = $state<ProductWithStock[]>([]);
  let tanggal = $state(todayIso());
  let noTrx = $state("Auto");
  let catatan = $state("");
  let nextId = 1;
  let rows = $state<BatchRow[]>([newRow()]);
  let busy = $state(false);
  let popupRow = $state<BatchRow | null>(null);

  function newRow(): BatchRow {
    return { id: nextId++, search: "", product: null, qty: 1, keterangan: "", dropOpen: false };
  }

  const filtered = (row: BatchRow) => {
    const q = row.search.trim().toLowerCase();
    if (!q) return [];
    return products.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        (p.barcode ?? "").toLowerCase().includes(q),
    ).slice(0, 8);
  };

  async function loadProducts() {
    try {
      products = await api.listProducts("", true);
    } catch (e) { toastError(e); }
  }
  onMount(loadProducts);

  async function onRowKey(e: KeyboardEvent, row: BatchRow) {
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

  function selectProduct(row: BatchRow, p: ProductWithStock) {
    row.product = p;
    row.search = p.name;
    row.dropOpen = false;
    rows = [...rows];
    focusQty(row.id);
  }

  /** Enter di kolom Jumlah: kalau baris ini yang terakhir, tambah baris baru & fokus ke situ;
   * kalau bukan, lompat ke baris berikutnya. Alur scan berkelanjutan (poin 8). */
  function onQtyKey(e: KeyboardEvent, row: BatchRow) {
    if (e.key !== "Enter") return;
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

  function addRow() {
    rows = [...rows, newRow()];
  }

  function removeRow(id: number) {
    if (rows.length <= 1) return;
    rows = rows.filter((r) => r.id !== id);
  }

  async function simpan() {
    const valid = rows.filter((r) => r.product && r.qty > 0);
    if (!valid.length) return showToast("Tidak ada item valid untuk disimpan.", "error");
    busy = true;
    try {
      const batch = await api.createStockMovementBatch({
        kind: kind as "in" | "out",
        note: catatan || null,
        user_id: $currentUser?.username ?? null,
        created_at: combineDateAndTime(tanggal, clock.now),
        items: valid.map((r) => ({
          product_id: r.product!.id,
          qty: r.qty,
          note: r.keterangan || null,
        })),
      });
      showToast(`${batch.no}: ${valid.length} item ${verb.toLowerCase()} tersimpan.`, "success");
      rows = [newRow()];
      catatan = "";
      tanggal = todayIso();
      await loadProducts();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="page-head"><h1>{title}</h1></div>

<!-- Header transaksi -->
<div class="trx-header card">
  <div class="trx-meta">
    <span class="meta-lbl">No. Transaksi</span>
    <span class="meta-val mono">{noTrx}</span>
  </div>
  <div class="trx-field">
    <label>Tanggal</label>
    {#if $currentUser?.role === "admin"}
      <input type="date" max={todayIso()} bind:value={tanggal} style="width:150px;" title="Admin bisa mundurkan tanggal untuk transaksi yang terlewat" />
    {:else}
      <span class="meta-val mono">{new Date(tanggal).toLocaleDateString("id-ID", { day:"2-digit", month:"short", year:"numeric" })}</span>
    {/if}
  </div>
  <div class="trx-field">
    <label>Jam</label>
    <span class="meta-val mono">{formatTime(clock.now)}</span>
  </div>
  <div class="trx-field" style="flex:1;">
    <label>Keterangan Umum</label>
    <input bind:value={catatan} placeholder="opsional" />
  </div>
</div>

<!-- Tabel batch -->
<div class="card" style="padding:0; overflow:hidden; margin-bottom:0.8rem;">
  <table class="batch-table">
    <thead>
      <tr>
        <th style="width:2rem;">No</th>
        <th>Kode/Nama Item</th>
        <th style="width:60px;">Satuan</th>
        <th style="width:110px;">Jumlah {verb}</th>
        <th>Keterangan Baris</th>
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
            <input class="cell-input" bind:value={row.keterangan} placeholder="—" />
          </td>
          <td>
            <button
              class="del-btn"
              title="Hapus baris"
              onclick={() => removeRow(row.id)}
              disabled={rows.length <= 1}
            >✕</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<div class="bottom-bar">
  <button onclick={addRow}>➕ Tambah Baris</button>
  <button class="btn-primary" disabled={busy} onclick={simpan}>
    💾 Simpan Semua Item {verb}
  </button>
  <span class="text-dim" style="margin-left:auto; font-size:0.82rem;">Riwayat &amp; edit ada di menu "Daftar Item {verb}".</span>
</div>

{#if popupRow}
  {@const targetRow = popupRow}
  <ProductSearchPopup
    initialQuery={targetRow.search}
    onClose={() => (popupRow = null)}
    onPick={(p) => { selectProduct(targetRow, p); popupRow = null; }}
  />
{/if}

<style>
  .trx-header {
    display: flex; align-items: flex-end; gap: 1.5rem;
    flex-wrap: wrap; margin-bottom: 0.8rem;
  }
  .trx-meta { display: flex; flex-direction: column; gap: 0.05rem; }
  .meta-lbl { font-size: 0.7rem; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.05em; }
  .meta-val { font-size: 0.95rem; font-weight: 700; }
  .trx-field { display: flex; flex-direction: column; gap: 0.2rem; }
  .trx-field label { font-size: 0.78rem; color: var(--text-dim); margin: 0; }

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

  .del-btn {
    padding: 0.15rem 0.35rem; color: var(--danger);
    border-color: transparent; background: transparent;
  }
  .del-btn:disabled { opacity: 0.2; }
</style>
