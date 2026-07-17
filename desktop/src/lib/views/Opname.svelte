<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { api } from "$lib/api";
  import { formatQty, formatDateTime, formatTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import { currentMonthBounds } from "$lib/monthPager";
  import { createLiveClock } from "$lib/liveClock.svelte";
  import { todayIso, combineDateAndTime } from "$lib/dateTime";
  import type { ProductWithStock, StockMovement } from "$lib/types";
  import MonthPager from "$lib/components/MonthPager.svelte";
  import ProductSearchPopup from "$lib/components/ProductSearchPopup.svelte";

  const clock = createLiveClock();
  onDestroy(() => clock.stop());

  let products = $state<ProductWithStock[]>([]);
  let history = $state<StockMovement[]>([]);
  let search = $state("");
  let selected = $state<ProductWithStock | null>(null);
  let fisik = $state(0);
  let keterangan = $state("");
  let busy = $state(false);
  let from = $state("");
  let to = $state("");
  let tanggal = $state(todayIso());
  let showPopup = $state(false);

  const selisih = $derived(selected ? fisik - selected.stock_qty : 0);
  const filtered = $derived(
    search.trim()
      ? products.filter(
          (p) =>
            p.name.toLowerCase().includes(search.toLowerCase()) ||
            (p.barcode ?? "").toLowerCase().includes(search.toLowerCase()),
        )
      : [],
  );

  async function load() {
    try {
      [products, history] = await Promise.all([
        api.listProducts("", true),
        api.listStockMovements("opname", from, to, 500),
      ]);
    } catch (e) { toastError(e); }
  }
  onMount(() => {
    [from, to] = currentMonthBounds();
    load();
  });

  async function onSearchKey(e: KeyboardEvent) {
    if (e.key !== "Enter") return;
    const term = search.trim();
    if (!term) return;
    // Coba cari via barcode dulu
    try {
      const p = await api.findByBarcode(term);
      if (p) { selectProduct(p); return; }
    } catch (_) { /* skip */ }
    if (filtered.length === 1) { selectProduct(filtered[0]); return; }
    showPopup = true;
  }

  function selectProduct(p: ProductWithStock) {
    selected = p;
    fisik = p.stock_qty;
    search = "";
    keterangan = "";
  }

  function resetForm() {
    selected = null;
    fisik = 0;
    keterangan = "";
    search = "";
  }

  async function simpan() {
    if (!selected) return;
    busy = true;
    try {
      await api.createStockMovement({
        product_id: selected.id,
        kind: "opname",
        qty: fisik,
        note: keterangan || "Stok opname",
        user_id: $currentUser?.username ?? null,
        created_at: combineDateAndTime(tanggal, clock.now),
      });
      showToast(`Opname ${selected.name} disimpan. Selisih: ${selisih >= 0 ? "+" : ""}${selisih}`, "success");
      resetForm();
      tanggal = todayIso();
      await load();
    } catch (e) { toastError(e); } finally { busy = false; }
  }
</script>

<div class="page-head"><h1>Stok Opname</h1></div>

<div class="op-grid">
  <!-- Form input -->
  <div class="card form-card">
    <h2>Input Opname</h2>

    <div class="row op-datetime">
      <div class="trx-field">
        <label>Tanggal</label>
        {#if $currentUser?.role === "admin"}
          <input type="date" max={todayIso()} bind:value={tanggal} style="width:150px;" title="Admin bisa mundurkan tanggal untuk opname yang terlewat" />
        {:else}
          <span class="info-val mono">{new Date(tanggal).toLocaleDateString("id-ID", { day:"2-digit", month:"short", year:"numeric" })}</span>
        {/if}
      </div>
      <div class="trx-field">
        <label>Jam</label>
        <span class="info-val mono">{formatTime(clock.now)}</span>
      </div>
    </div>

    <!-- Scan / cari item -->
    <label>Kode Item / Nama Barang</label>
    <div style="position:relative;">
      <input
        placeholder="Scan barcode atau ketik nama lalu Enter…"
        bind:value={search}
        onkeydown={onSearchKey}
        autocomplete="off"
      />
      {#if filtered.length > 0}
        <div class="search-drop">
          {#each filtered.slice(0, 8) as p (p.id)}
            <button class="sd-row" onclick={() => selectProduct(p)}>
              <span class="sd-name">{p.name}</span>
              <span class="sd-bc text-dim">{p.barcode ?? ""}</span>
              <span class="sd-stok text-dim">stok {formatQty(p.stock_qty)}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Data item terpilih -->
    <div class="item-info {selected ? '' : 'empty'}">
      {#if selected}
        <div class="info-row"><span class="info-lbl">Nama Item</span><span class="info-val">{selected.name}</span></div>
        <div class="info-row"><span class="info-lbl">Satuan</span><span class="info-val">{selected.unit ?? "—"}</span></div>
        <div class="info-row highlight"><span class="info-lbl">Buku (Sistem)</span><span class="info-val mono">{formatQty(selected.stock_qty)}</span></div>

        <div class="fisik-row">
          <label>Fisik (Hasil Hitung)</label>
          <input class="fisik-input mono" type="number" min="0" step="0.01" bind:value={fisik} />
        </div>

        <div class="info-row selisih {selisih >= 0 ? 'plus' : 'minus'}">
          <span class="info-lbl">Selisih</span>
          <span class="info-val mono fw-bold">{selisih >= 0 ? "+" : ""}{formatQty(selisih)}</span>
        </div>

        <label style="margin-top:0.6rem;">Keterangan</label>
        <input bind:value={keterangan} placeholder="opsional" />

        <div class="row" style="justify-content:flex-end; margin-top:1rem; gap:0.5rem;">
          <button onclick={resetForm}>Batal</button>
          <button class="btn-primary" disabled={busy} onclick={simpan}>Simpan Opname</button>
        </div>
      {:else}
        <div class="empty-msg text-dim">Scan barcode atau cari nama barang di atas untuk memulai opname.</div>
      {/if}
    </div>
  </div>

  <!-- Riwayat opname terakhir -->
  <div class="card" style="padding:0; overflow:hidden;">
    <div style="padding:0.7rem 0.9rem; border-bottom:1px solid var(--border); display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:0.5rem;">
      <span style="font-weight:650;">Riwayat Opname Terakhir</span>
      <MonthPager bind:from bind:to onchange={load} />
    </div>
    <div style="max-height:480px; overflow-y:auto;">
      <table>
        <thead>
          <tr>
            <th>Waktu</th><th>Barang</th>
            <th class="text-right">Fisik</th><th class="text-right">Stok Akhir</th>
            <th>Catatan</th>
          </tr>
        </thead>
        <tbody>
          {#each history as r (r.id)}
            <tr>
              <td style="white-space:nowrap;">{formatDateTime(r.created_at)}</td>
              <td>{r.product_name}</td>
              <td class="text-right mono">{formatQty(r.qty)}</td>
              <td class="text-right mono">{formatQty(r.stock_after)}</td>
              <td>{r.note ?? "—"}</td>
            </tr>
          {:else}
            <tr><td colspan="5" class="text-dim">Belum ada riwayat opname.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</div>

{#if showPopup}
  <ProductSearchPopup
    initialQuery={search}
    onClose={() => (showPopup = false)}
    onPick={(p) => { selectProduct(p); showPopup = false; }}
  />
{/if}

<style>
  .op-grid { display: grid; grid-template-columns: 400px 1fr; gap: 1rem; align-items: start; }
  .form-card { display: flex; flex-direction: column; gap: 0.5rem; }
  .op-datetime { gap: 1.2rem; margin-bottom: 0.3rem; }
  .trx-field { display: flex; flex-direction: column; gap: 0.2rem; }
  .trx-field label { font-size: 0.78rem; color: var(--text-dim); margin: 0; }

  .search-drop {
    position: absolute; left: 0; right: 0; top: 100%; z-index: 50;
    background: var(--white); border: 1px solid var(--border);
    border-top: none; border-radius: 0 0 var(--radius) var(--radius);
    box-shadow: var(--shadow); max-height: 220px; overflow-y: auto;
  }
  .sd-row {
    display: grid; grid-template-columns: 1fr auto auto; gap: 0.5rem;
    align-items: center; width: 100%; text-align: left;
    border: none; border-radius: 0; border-bottom: 1px solid var(--border);
    padding: 0.4rem 0.7rem; font-size: 0.85rem;
  }
  .sd-row:last-child { border-bottom: none; }
  .sd-name { font-weight: 600; }
  .sd-bc, .sd-stok { font-size: 0.78rem; }

  .item-info {
    border: 1px solid var(--border); border-radius: var(--radius);
    padding: 0.8rem; margin-top: 0.3rem; min-height: 140px;
  }
  .item-info.empty { display: flex; align-items: center; justify-content: center; }
  .empty-msg { font-size: 0.85rem; text-align: center; }

  .info-row { display: flex; justify-content: space-between; align-items: center; padding: 0.3rem 0; border-bottom: 1px solid var(--border); }
  .info-row:last-of-type { border-bottom: none; }
  .info-lbl { font-size: 0.82rem; color: var(--text-dim); }
  .info-val { font-weight: 600; }
  .info-row.highlight { background: var(--baby-blue-bg); margin: 0 -0.8rem; padding: 0.3rem 0.8rem; }
  .info-row.selisih { margin-top: 0.4rem; border-radius: 6px; padding: 0.4rem 0.8rem; }
  .info-row.selisih.plus { background: #e8f7ef; }
  .info-row.selisih.minus { background: #fdecea; }
  .info-row.selisih .info-val { font-size: 1.1rem; }
  .plus .info-val { color: var(--success); }
  .minus .info-val { color: var(--danger); }

  .fisik-row { margin: 0.6rem 0 0.3rem; }
  .fisik-row label { display: block; font-weight: 600; margin-bottom: 0.3rem; }
  .fisik-input { width: 100%; font-size: 1.1rem; font-weight: 700; text-align: right; }
  .fw-bold { font-weight: 700; }
</style>
