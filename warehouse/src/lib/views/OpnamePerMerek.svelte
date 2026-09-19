<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { api } from "$lib/api";
  import { formatQty, formatTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import { createLiveClock } from "$lib/liveClock.svelte";
  import { todayIso, combineDateAndTime } from "$lib/dateTime";
  import { setTabDirty, clearTabDirty } from "$lib/stores/tabGuard";
  import type { Brand } from "$lib/types";

  let { tabId }: { tabId?: string } = $props();

  const clock = createLiveClock();
  onDestroy(() => clock.stop());
  onDestroy(() => { if (tabId) clearTabDirty(tabId); });

  interface OpnameRow {
    product_id: string;
    barcode: string | null;
    name: string;
    unit: string | null;
    stock_qty: number;
    fisik: number;
  }

  let brands = $state<Brand[]>([]);
  let selectedBrand = $state("");
  let rows = $state<OpnameRow[]>([]);
  let loading = $state(false);
  let search = $state("");
  let keterangan = $state("");
  let tanggal = $state(todayIso());
  let saving = $state(false);
  let saveProgress = $state({ done: 0, total: 0 });

  const changedCount = $derived(rows.filter((r) => r.fisik !== r.stock_qty).length);
  const isDirty = $derived(changedCount > 0);
  $effect(() => {
    if (tabId) setTabDirty(tabId, selectedBrand !== "" && isDirty);
  });

  const filteredRows = $derived(
    search.trim()
      ? rows.filter(
          (r) =>
            r.name.toLowerCase().includes(search.toLowerCase()) ||
            (r.barcode ?? "").toLowerCase().includes(search.toLowerCase()),
        )
      : rows,
  );

  onMount(async () => {
    try {
      brands = await api.listBrands();
    } catch (e) { toastError(e); }
  });

  async function onBrandChange() {
    search = "";
    if (!selectedBrand) { rows = []; return; }
    loading = true;
    try {
      const page = await api.listProductsPage({
        brand: selectedBrand,
        includeInactive: true,
        sortBy: "name",
        sortDir: "asc",
        limit: 100000,
        offset: 0,
      });
      rows = page.items.map((p) => ({
        product_id: p.id,
        barcode: p.barcode,
        name: p.name,
        unit: p.unit,
        stock_qty: p.stock_qty,
        fisik: p.stock_qty,
      }));
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }

  function setFisik(row: OpnameRow, val: number) {
    row.fisik = Number.isFinite(val) && val >= 0 ? val : 0;
    rows = [...rows];
  }

  /** Fokus + pilih seluruh isi input Fisik baris tertentu (mouse maupun keyboard). */
  async function focusFisik(productId: string) {
    await tick();
    const el = document.querySelector<HTMLInputElement>(`[data-fisik-row="${productId}"]`);
    el?.focus();
    el?.select();
  }

  /** Panah atas/kiri = baris sebelumnya, panah bawah/kanan = baris berikutnya —
   * bukan increment/decrement bawaan input number (di-preventDefault). */
  function onFisikKey(e: KeyboardEvent, row: OpnameRow) {
    if (!["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(e.key)) return;
    e.preventDefault();
    const idx = filteredRows.findIndex((r) => r.product_id === row.product_id);
    if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
      if (idx > 0) focusFisik(filteredRows[idx - 1].product_id);
    } else {
      if (idx < filteredRows.length - 1) focusFisik(filteredRows[idx + 1].product_id);
    }
  }

  async function simpanSemua() {
    if (!selectedBrand) return showToast("Pilih merek dulu.", "info");
    const toSave = rows.filter((r) => r.fisik !== r.stock_qty);
    if (toSave.length === 0) return showToast("Tidak ada perubahan (selisih) untuk disimpan.", "info");
    saving = true;
    saveProgress = { done: 0, total: toSave.length };
    const createdAt = combineDateAndTime(tanggal, clock.now);
    let okCount = 0;
    let errCount = 0;
    for (let i = 0; i < toSave.length; i++) {
      const r = toSave[i];
      saveProgress = { done: i, total: toSave.length };
      try {
        await api.createStockMovement({
          product_id: r.product_id,
          kind: "opname",
          qty: r.fisik,
          note: keterangan || "Stok opname (per merek)",
          user_id: $currentUser?.username ?? null,
          created_at: createdAt,
        });
        okCount++;
      } catch (e) {
        errCount++;
      }
    }
    saveProgress = { done: toSave.length, total: toSave.length };
    saving = false;
    showToast(
      `Opname merek "${selectedBrand}" selesai: ${okCount} barang (selisih) tersimpan${errCount ? `, ${errCount} gagal` : ""}.`,
      errCount ? "error" : "success",
      6000,
    );
    tanggal = todayIso();
    await onBrandChange();
  }
</script>

<div class="view-flex">
<div class="page-head">
  <h1>Opname per Merek</h1>
</div>

<div class="card" style="margin-bottom:0.8rem; flex-shrink:0;">
  <div class="row" style="align-items:flex-end; gap:0.8rem; flex-wrap:wrap;">
    <div class="trx-field">
      <label>Merek</label>
      <select bind:value={selectedBrand} onchange={onBrandChange} style="width:220px;">
        <option value="">— Pilih Merek —</option>
        {#each brands as b (b.id)}<option value={b.name}>{b.name}</option>{/each}
      </select>
    </div>
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
    <div class="trx-field" style="flex:1; min-width:200px;">
      <label>Keterangan (semua baris)</label>
      <input bind:value={keterangan} placeholder="opsional" />
    </div>
    {#if rows.length}
      <div class="trx-field">
        <label>Cari di Daftar</label>
        <input bind:value={search} placeholder="nama / barcode…" style="width:200px;" />
      </div>
    {/if}
  </div>
</div>

{#if saving}
  <div class="import-block" style="flex-shrink:0;">
    <div class="import-progress-bar">
      <div class="import-progress-fill" style="width:{saveProgress.total ? (saveProgress.done / saveProgress.total) * 100 : 0}%"></div>
    </div>
    <p class="text-dim" style="font-size:0.8rem; margin-top:0.3rem;">
      Menyimpan {saveProgress.done} / {saveProgress.total} barang…
    </p>
  </div>
{/if}

<div class="card list-card">
  <div class="list-scroll">
    <table>
      <thead>
        <tr>
          <th style="width:140px;">Barcode</th>
          <th>Nama Item</th>
          <th class="text-right" style="width:100px;">Stok</th>
          <th class="text-right" style="width:130px;">Fisik</th>
          <th class="text-right" style="width:100px;">Selisih</th>
        </tr>
      </thead>
      <tbody>
        {#each filteredRows as r (r.product_id)}
          {@const selisih = r.fisik - r.stock_qty}
          <tr>
            <td class="mono text-dim">{r.barcode ?? "-"}</td>
            <td>{r.name}{#if r.unit} <span class="text-dim">({r.unit})</span>{/if}</td>
            <td class="text-right mono">{formatQty(r.stock_qty)}</td>
            <td>
              <input
                class="mono cell-num"
                type="number" min="0" step="0.01"
                data-fisik-row={r.product_id}
                value={r.fisik}
                oninput={(e) => setFisik(r, +e.currentTarget.value)}
                onkeydown={(e) => onFisikKey(e, r)}
                onfocus={(e) => e.currentTarget.select()}
              />
            </td>
            <td class="text-right mono fw-bold" style="color: {selisih === 0 ? 'inherit' : selisih > 0 ? 'var(--success)' : 'var(--danger)'};">
              {selisih >= 0 ? "+" : ""}{formatQty(selisih)}
            </td>
          </tr>
        {:else}
          <tr>
            <td colspan="5" class="text-dim" style="text-align:center; padding:1.5rem 0;">
              {loading ? "Memuat…" : selectedBrand ? "Tidak ada barang untuk merek ini." : "Pilih merek dulu untuk memulai opname."}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  <div class="pager">
    <span class="text-dim" style="font-size:0.82rem;">
      {rows.length.toLocaleString("id-ID")} barang{search.trim() ? ` (${filteredRows.length} ditampilkan)` : ""}{changedCount ? ` · ${changedCount} ada selisih` : ""}
    </span>
    <button class="btn-primary" disabled={saving || !selectedBrand || changedCount === 0} onclick={simpanSemua}>
      💾 Simpan Perubahan ({changedCount})
    </button>
  </div>
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
  .trx-field { display: flex; flex-direction: column; gap: 0.2rem; }
  .trx-field label { font-size: 0.78rem; color: var(--text-dim); margin: 0; }
  .info-val { font-weight: 600; }
  .fw-bold { font-weight: 700; }
  .cell-num { width: 100%; text-align: right; padding: 0.25rem 0.4rem; }

  .import-block {
    border: 1px dashed var(--border-strong); border-radius: 8px;
    padding: 0.6rem 0.7rem; margin-bottom: 0.6rem; background: var(--baby-blue-bg);
  }
  .import-progress-bar { height: 8px; border-radius: 999px; background: var(--white); overflow: hidden; }
  .import-progress-fill { height: 100%; background: var(--primary); transition: width 0.15s linear; }
</style>
