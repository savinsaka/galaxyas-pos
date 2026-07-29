<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { api } from "$lib/api";
  import { formatQty, formatTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import { createLiveClock } from "$lib/liveClock.svelte";
  import { todayIso, combineDateAndTime } from "$lib/dateTime";
  import { setTabDirty, clearTabDirty } from "$lib/stores/tabGuard";
  import { activeTabId } from "$lib/stores/tabs";
  import type { Brand, ProductWithStock } from "$lib/types";
  import ShortcutBar from "$lib/components/ShortcutBar.svelte";

  let { tabId }: { tabId?: string } = $props();

  const clock = createLiveClock();
  onDestroy(() => clock.stop());
  onDestroy(() => { if (tabId) clearTabDirty(tabId); });

  interface OpRow {
    id: number;
    search: string;
    product: ProductWithStock | null;
    fisik: number;
    dropOpen: boolean;
  }

  let brands = $state<Brand[]>([]);
  let selectedBrand = $state("");
  /** Semua barang merek terpilih (aktif + non-aktif) — dasar pencarian & hitung yang akan dinolkan. */
  let brandProducts = $state<ProductWithStock[]>([]);
  let loadingBrand = $state(false);
  let tanggal = $state(todayIso());
  let keterangan = $state("");
  let busy = $state(false);
  let showConfirm = $state(false);

  let nextId = 1;
  function newRow(): OpRow {
    return { id: nextId++, search: "", product: null, fisik: 0, dropOpen: false };
  }
  let rows = $state<OpRow[]>([newRow()]);

  const filledRows = $derived(rows.filter((r) => r.product !== null));
  const countedIds = $derived(new Set(filledRows.map((r) => r.product!.id)));
  /** Barang merek ini yang tidak dihitung TAPI stoknya masih ada → akan dinolkan. */
  const toZero = $derived(
    brandProducts.filter((p) => !countedIds.has(p.id) && p.stock_qty !== 0),
  );

  $effect(() => {
    if (tabId) setTabDirty(tabId, filledRows.length > 0);
  });

  onMount(async () => {
    try {
      brands = await api.listBrands();
    } catch (e) { toastError(e); }
  });

  // F9 = Simpan, F6 = Tambah Baris — sama tombol dengan Item Masuk/Keluar.
  function onGlobalKey(e: KeyboardEvent) {
    if (tabId && $activeTabId !== tabId) return;
    if (e.key === "F9") {
      e.preventDefault();
      if (!busy && !showConfirm) mintaKonfirmasi();
    } else if (e.key === "F6") {
      e.preventDefault();
      if (!showConfirm) addRow();
    }
  }
  onMount(() => window.addEventListener("keydown", onGlobalKey));
  onDestroy(() => window.removeEventListener("keydown", onGlobalKey));

  async function loadBrandProducts() {
    if (!selectedBrand) { brandProducts = []; return; }
    loadingBrand = true;
    try {
      // Filter merek dikerjakan server (bukan saring hasil ber-limit di klien)
      // supaya merek dengan ratusan barang tidak terpotong.
      const page = await api.listProductsPage({
        brand: selectedBrand,
        includeInactive: true,
        sortBy: "name",
        sortDir: "asc",
        limit: 100000,
        offset: 0,
      });
      brandProducts = page.items;
    } catch (e) {
      toastError(e);
      brandProducts = [];
    } finally {
      loadingBrand = false;
    }
  }

  /** Merek yang datanya sedang dimuat — dipakai mengembalikan pilihan select
   * kalau kasir membatalkan konfirmasi ganti merek. */
  let loadedBrand = $state("");

  async function onBrandChange() {
    if (filledRows.length && !confirm("Ganti merek akan mengosongkan tabel opname yang sudah diisi. Lanjut?")) {
      selectedBrand = loadedBrand;
      return;
    }
    rows = [newRow()];
    loadedBrand = selectedBrand;
    await loadBrandProducts();
  }

  const filtered = (row: OpRow) => {
    const q = row.search.trim().toLowerCase();
    if (!q) return [];
    return brandProducts
      .filter(
        (p) =>
          p.name.toLowerCase().includes(q) ||
          (p.barcode ?? "").toLowerCase().includes(q),
      )
      .slice(0, 8);
  };

  async function focusFisik(rowId: number) {
    await tick();
    const el = document.querySelector<HTMLInputElement>(`[data-fisik-row="${rowId}"]`);
    el?.focus();
    el?.select();
  }
  async function focusSearch(rowId: number) {
    await tick();
    const el = document.querySelector<HTMLInputElement>(`[data-search-row="${rowId}"]`);
    el?.focus();
    el?.select();
  }

  function selectProduct(row: OpRow, p: ProductWithStock) {
    // Barang yang sama sudah ada di baris lain → jangan bikin baris kembar,
    // langsung lompat ke baris itu supaya kasir tinggal membetulkan angkanya.
    const dup = rows.find((r) => r.id !== row.id && r.product?.id === p.id);
    if (dup) {
      row.search = "";
      row.product = null;
      row.dropOpen = false;
      rows = [...rows];
      showToast(`"${p.name}" sudah ada di baris ${rows.indexOf(dup) + 1}.`, "info");
      focusFisik(dup.id);
      return;
    }
    row.product = p;
    row.search = p.name;
    row.fisik = p.stock_qty;
    row.dropOpen = false;
    rows = [...rows];
    focusFisik(row.id);
  }

  /** Cari barcode di daftar merek dulu (offline, cepat, ikut barang non-aktif).
   * Baru kalau tidak ketemu tanya server — supaya bisa bilang "beda merek". */
  async function resolveTerm(row: OpRow, term: string) {
    const t = term.trim().toLowerCase();
    const byBarcode = brandProducts.find((p) => (p.barcode ?? "").toLowerCase() === t);
    if (byBarcode) { selectProduct(row, byBarcode); return; }

    let outside: ProductWithStock | null = null;
    try {
      outside = await api.findByBarcode(term.trim());
    } catch (_) { /* abaikan, lanjut cari nama */ }
    if (outside) {
      showToast(
        `Barang beda merek: "${outside.name}" merek ${outside.brand ?? "(tanpa merek)"}, bukan ${selectedBrand}.`,
        "error",
        5000,
      );
      row.search = "";
      rows = [...rows];
      focusSearch(row.id);
      return;
    }

    const f = filtered(row);
    if (f.length === 1) { selectProduct(row, f[0]); return; }
    if (f.length === 0) {
      showToast(`"${term.trim()}" tidak ada di merek ${selectedBrand}.`, "error");
      return;
    }
    row.dropOpen = true;
    rows = [...rows];
  }

  async function onSearchKey(e: KeyboardEvent, row: OpRow) {
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
      if (input.selectionStart === input.value.length && row.product) {
        e.preventDefault();
        focusFisik(row.id);
      }
      return;
    }
    if (e.key === "Delete" && !row.search.trim() && !row.product) {
      e.preventDefault();
      removeRow(row.id);
      return;
    }
    if (e.key !== "Enter") return;
    if (!selectedBrand) { showToast("Pilih merek dulu.", "info"); return; }
    const term = row.search.trim();
    if (!term) return;
    await resolveTerm(row, term);
  }

  /** Enter/panah-bawah di kolom Fisik: baris berikutnya, atau baris baru kalau ini yang terakhir. */
  function advanceRow(row: OpRow) {
    if (!row.product) return;
    const idx = rows.findIndex((r) => r.id === row.id);
    if (idx === rows.length - 1) {
      const nr = newRow();
      rows = [...rows, nr];
      focusSearch(nr.id);
    } else {
      focusSearch(rows[idx + 1].id);
    }
  }

  function onFisikKey(e: KeyboardEvent, row: OpRow) {
    if (e.key === "ArrowLeft") {
      const input = e.currentTarget as HTMLInputElement;
      let atStart = true;
      try {
        atStart = input.selectionStart === 0;
      } catch (_) { /* type=number tidak selalu izinkan baca selectionStart */ }
      if (atStart) {
        e.preventDefault();
        focusSearch(row.id);
      }
      return;
    }
    if (e.key === "ArrowDown" || e.key === "Enter") {
      e.preventDefault();
      advanceRow(row);
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      const idx = rows.findIndex((r) => r.id === row.id);
      if (idx > 0) focusFisik(rows[idx - 1].id);
    }
  }

  function addRow() {
    rows = [...rows, newRow()];
  }

  function removeRow(id: number) {
    if (rows.length <= 1) { rows = [newRow()]; return; }
    rows = rows.filter((r) => r.id !== id);
  }

  function mintaKonfirmasi() {
    if (!selectedBrand) return showToast("Pilih merek dulu.", "info");
    if (loadingBrand) return showToast("Daftar barang merek masih dimuat…", "info");
    showConfirm = true;
  }

  async function simpan() {
    showConfirm = false;
    busy = true;
    try {
      const res = await api.createOpnameSpecial({
        brand: selectedBrand,
        note: keterangan || null,
        user_id: $currentUser?.username ?? null,
        created_at: combineDateAndTime(tanggal, clock.now),
        items: filledRows.map((r) => ({ product_id: r.product!.id, qty: r.fisik })),
      });
      showToast(
        `Opname spesial merek "${res.brand}" selesai: ${res.counted} barang dihitung, ${res.zeroed} barang dinolkan.`,
        "success",
        7000,
      );
      rows = [newRow()];
      keterangan = "";
      tanggal = todayIso();
      await loadBrandProducts();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="page-head"><h1>Opname Spesial</h1></div>

<!-- Header: merek + waktu + keterangan -->
<div class="trx-header card">
  <div class="trx-field">
    <label for="os-brand">Merek</label>
    <select id="os-brand" bind:value={selectedBrand} onchange={onBrandChange} style="width:220px;">
      <option value="">— Pilih Merek —</option>
      {#each brands as b (b.id)}<option value={b.name}>{b.name}</option>{/each}
    </select>
  </div>
  <div class="trx-field">
    <label for="os-tgl">Tanggal</label>
    {#if $currentUser?.role === "admin"}
      <input id="os-tgl" type="date" max={todayIso()} bind:value={tanggal} style="width:150px;" title="Admin bisa mundurkan tanggal untuk opname yang terlewat" />
    {:else}
      <span class="meta-val mono">{new Date(tanggal).toLocaleDateString("id-ID", { day:"2-digit", month:"short", year:"numeric" })}</span>
    {/if}
  </div>
  <div class="trx-field">
    <label for="os-jam">Jam</label>
    <span id="os-jam" class="meta-val mono">{formatTime(clock.now)}</span>
  </div>
  <div class="trx-field" style="flex:1; min-width:200px;">
    <label for="os-ket">Keterangan</label>
    <input id="os-ket" bind:value={keterangan} placeholder="opsional" />
  </div>
</div>

<div class="warn-block">
  <strong>⚠️ Perhatian:</strong>
  {#if selectedBrand}
    semua barang merek <b>{selectedBrand}</b> yang <b>tidak</b> ada di tabel ini akan dianggap habis —
    stoknya jadi <b>0</b> saat disimpan.
    {#if loadingBrand}
      <span class="text-dim">Memuat daftar barang merek…</span>
    {:else}
      <span class="text-dim">
        Merek ini punya {brandProducts.length.toLocaleString("id-ID")} barang ·
        {filledRows.length} dihitung · <b>{toZero.length} akan dinolkan</b>.
      </span>
    {/if}
  {:else}
    pilih merek dulu. Barang merek itu yang tidak di-scan akan dinolkan stoknya saat disimpan.
  {/if}
</div>

<!-- Tabel opname -->
<div class="card" style="padding:0; overflow:hidden; margin-bottom:0.8rem;">
  <table class="batch-table">
    <thead>
      <tr>
        <th style="width:2rem;">No</th>
        <th>Kode/Nama Item</th>
        <th style="width:60px;">Satuan</th>
        <th class="text-right" style="width:100px;">Stok</th>
        <th style="width:120px;">Fisik</th>
        <th class="text-right" style="width:100px;">Selisih</th>
        <th style="width:2rem;"></th>
      </tr>
    </thead>
    <tbody>
      {#each rows as row (row.id)}
        {@const selisih = row.product ? row.fisik - row.product.stock_qty : 0}
        <tr>
          <td class="text-dim mono" style="text-align:center;">{rows.indexOf(row) + 1}</td>
          <td style="position:relative;">
            <input
              class="cell-input"
              data-search-row={row.id}
              placeholder={selectedBrand ? "Scan barcode atau ketik nama…" : "Pilih merek dulu…"}
              disabled={!selectedBrand}
              bind:value={row.search}
              oninput={() => { row.dropOpen = true; rows = [...rows]; }}
              onkeydown={(e) => onSearchKey(e, row)}
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
          <td class="text-right mono text-dim">{row.product ? formatQty(row.product.stock_qty) : "—"}</td>
          <td>
            <input
              class="cell-input mono"
              data-fisik-row={row.id}
              type="number" min="0" step="0.01"
              disabled={!row.product}
              bind:value={row.fisik}
              onkeydown={(e) => onFisikKey(e, row)}
              onfocus={(e) => e.currentTarget.select()}
            />
          </td>
          <td
            class="text-right mono fw-bold"
            style="color: {!row.product || selisih === 0 ? 'inherit' : selisih > 0 ? 'var(--success)' : 'var(--danger)'};"
          >
            {row.product ? `${selisih >= 0 ? "+" : ""}${formatQty(selisih)}` : "—"}
          </td>
          <td>
            <button class="del-btn" title="Hapus baris" onclick={() => removeRow(row.id)}>✕</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<div class="bottom-bar">
  <button onclick={addRow} disabled={!selectedBrand}>➕ Tambah Baris</button>
  <button class="btn-primary" disabled={busy || !selectedBrand} onclick={mintaKonfirmasi}>
    💾 Simpan Opname Spesial
  </button>
  <span class="text-dim" style="margin-left:auto; font-size:0.82rem;">
    {#if selectedBrand}
      {filledRows.length} barang dihitung · {toZero.length} akan dinolkan
    {:else}
      Riwayat opname ada di menu "Opname".
    {/if}
  </span>
</div>
<ShortcutBar items={[
  { key: "F9", label: "Simpan Opname Spesial", action: mintaKonfirmasi, disabled: busy || !selectedBrand },
  { key: "F6", label: "Tambah Baris", action: addRow, disabled: !selectedBrand },
]} />

{#if showConfirm}
  <div class="modal-backdrop" onclick={() => (showConfirm = false)} role="presentation">
    <div class="modal confirm-modal" onclick={(e) => e.stopPropagation()} role="presentation">
      <h2 style="margin-top:0;">Simpan Opname Spesial merek "{selectedBrand}"?</h2>
      <p style="margin:0.2rem 0 0.6rem;">
        <b>{filledRows.length}</b> barang akan diset sesuai hitungan fisik, dan
        <b style="color:var(--danger);">{toZero.length}</b> barang merek ini yang tidak dihitung
        akan <b>dinolkan stoknya</b>. Opname tidak bisa dibatalkan.
      </p>
      {#if toZero.length}
        <div class="zero-list">
          {#each toZero as p (p.id)}
            <div class="zero-row">
              <span>{p.name}{#if !p.is_active} <span class="text-dim">(non-aktif)</span>{/if}</span>
              <span class="mono text-dim">{formatQty(p.stock_qty)} → 0</span>
            </div>
          {/each}
        </div>
      {/if}
      <div class="row" style="justify-content:flex-end; gap:0.5rem; margin-top:0.9rem;">
        <button onclick={() => (showConfirm = false)}>Batal</button>
        <button class="btn-primary" disabled={busy} onclick={simpan}>
          {busy ? "Menyimpan…" : "Ya, Simpan & Nolkan"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .trx-header {
    display: flex; align-items: flex-end; gap: 1.5rem;
    flex-wrap: wrap; margin-bottom: 0.8rem;
  }
  .trx-field { display: flex; flex-direction: column; gap: 0.2rem; }
  .trx-field label { font-size: 0.78rem; color: var(--text-dim); margin: 0; }
  .meta-val { font-size: 0.95rem; font-weight: 700; }

  .warn-block {
    border: 1px solid var(--danger); border-radius: 8px;
    background: color-mix(in srgb, var(--danger) 8%, transparent);
    padding: 0.55rem 0.75rem; margin-bottom: 0.8rem; font-size: 0.85rem;
  }

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
  .fw-bold { font-weight: 700; }

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

  .confirm-modal { max-width: 520px; width: 92vw; padding: 1rem 1.1rem; }
  .zero-list {
    max-height: 240px; overflow-y: auto;
    border: 1px solid var(--border); border-radius: 8px;
  }
  .zero-row {
    display: flex; justify-content: space-between; gap: 0.6rem;
    padding: 0.28rem 0.6rem; font-size: 0.82rem;
    border-bottom: 1px solid var(--border);
  }
  .zero-row:last-child { border-bottom: none; }
</style>
