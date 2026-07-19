<script lang="ts">
  import { onMount } from "svelte";
  import * as XLSX from "xlsx";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import { debounce } from "$lib/debounce";
  import { formatMoneyInput, onMoneyInput } from "$lib/moneyInput";
  import type { Brand, Product, ProductInput, ProductWithStock } from "$lib/types";

  const PAGE_SIZE = 50;

  // ---------- Batch edit via Excel (poin B2) ----------
  interface BatchFieldOption { key: string; label: string; columns: string[] }
  const BATCH_FIELD_OPTIONS: BatchFieldOption[] = [
    { key: "all", label: "Semua Field", columns: ["barcode", "nama", "kategori", "merek", "satuan", "harga_pokok", "harga_jual", "default_diskon", "aktif"] },
    { key: "nama", label: "Nama", columns: ["barcode", "nama"] },
    { key: "kategori", label: "Kategori", columns: ["barcode", "kategori"] },
    { key: "merek", label: "Merek", columns: ["barcode", "merek"] },
    { key: "satuan", label: "Satuan", columns: ["barcode", "satuan"] },
    { key: "harga_pokok", label: "Harga Pokok", columns: ["barcode", "harga_pokok"] },
    { key: "harga_jual", label: "Harga Jual", columns: ["barcode", "harga_jual"] },
    { key: "default_diskon", label: "Diskon Default", columns: ["barcode", "default_diskon"] },
    { key: "aktif", label: "Status Aktif", columns: ["barcode", "aktif"] },
  ];
  const EXAMPLE_VALUES: [Record<string, string | number>, Record<string, string | number>] = [
    { barcode: "8992388101010", nama: "Indomie Goreng", kategori: "Mie Instan", merek: "Indofood", satuan: "pcs", harga_pokok: 2800, harga_jual: 3500, default_diskon: 0, aktif: "ya" },
    { barcode: "8993675001020", nama: "Aqua 600ml", kategori: "Minuman", merek: "Aqua", satuan: "botol", harga_pokok: 3000, harga_jual: 4000, default_diskon: 0, aktif: "ya" },
  ];

  interface BatchLogEntry { row: number; name: string; reason: string }

  let batchFieldKey = $state("all");
  const batchFieldOption = $derived(BATCH_FIELD_OPTIONS.find((o) => o.key === batchFieldKey)!);
  let batchImporting = $state(false);
  let batchImportProgress = $state({ done: 0, total: 0 });
  let batchImportLog = $state<BatchLogEntry[]>([]);

  function parseAktif(v: unknown): boolean {
    const s = String(v ?? "").toLowerCase().trim();
    if (s === "") return true;
    return ["1", "ya", "aktif", "true", "y"].includes(s);
  }

  function productToInput(p: Product): ProductInput {
    return {
      id: p.id, name: p.name, barcode: p.barcode, category: p.category, brand: p.brand,
      unit: p.unit, sell_price: p.sell_price, cost_price: p.cost_price,
      default_discount: p.default_discount, is_active: p.is_active,
    };
  }
  function applyBatchColumn(input: ProductInput, col: string, raw: unknown) {
    switch (col) {
      case "nama": { const v = String(raw ?? "").trim(); if (v) input.name = v; break; }
      case "kategori": input.category = String(raw ?? "") || null; break;
      case "merek": input.brand = String(raw ?? "") || null; break;
      case "satuan": input.unit = String(raw ?? "") || null; break;
      case "harga_pokok": input.cost_price = Number(raw) || 0; break;
      case "harga_jual": input.sell_price = Number(raw) || 0; break;
      case "default_diskon": input.default_discount = Number(raw) || 0; break;
      case "aktif": input.is_active = parseAktif(raw); break;
    }
  }

  async function openBatchTemplate() {
    try {
      const cols = batchFieldOption.columns;
      const rows = EXAMPLE_VALUES.map((ex) => cols.map((c) => ex[c] ?? ""));
      const ws = XLSX.utils.aoa_to_sheet([cols, ...rows]);
      (ws as Record<string, unknown>)["!protect"] = { selectLockedCells: true, selectUnlockedCells: true };
      const wb = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(wb, ws, "Template");
      const out = XLSX.write(wb, { type: "array", bookType: "xlsx" }) as ArrayBuffer;
      const bytes = Array.from(new Uint8Array(out));
      const path = await api.writeTempFile(`Template-Batch-${batchFieldOption.key}.xlsx`, bytes);
      await openPath(path);
      showToast("Template dibuka di Excel (read-only). Pilih Save As untuk mengisi & menyimpan.", "info", 7000);
    } catch (e) {
      toastError(e);
    }
  }

  async function onBatchImportFile(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    let parsed: Record<string, unknown>[] = [];
    try {
      const buf = await file.arrayBuffer();
      const wb = XLSX.read(buf, { type: "array" });
      const ws = wb.Sheets[wb.SheetNames[0]];
      parsed = XLSX.utils.sheet_to_json(ws, { defval: "" });
    } catch (err) {
      toastError(err);
      input.value = "";
      return;
    }
    input.value = "";
    if (!parsed.length) return showToast("File kosong.", "info");

    batchImporting = true;
    batchImportLog = [];
    batchImportProgress = { done: 0, total: parsed.length };
    const fieldsToApply = batchFieldOption.columns.filter((c) => c !== "barcode");
    let okCount = 0;

    for (let i = 0; i < parsed.length; i++) {
      const r = parsed[i];
      batchImportProgress = { done: i, total: parsed.length };
      const barcode = String(r.barcode ?? "").trim();
      if (!barcode) {
        batchImportLog.push({ row: i + 2, name: "(kosong)", reason: "Barcode kosong" });
        continue;
      }
      try {
        const p = await api.findByBarcode(barcode);
        if (!p) {
          batchImportLog.push({ row: i + 2, name: barcode, reason: "Barang tidak ditemukan" });
          continue;
        }
        const productInput = productToInput(p);
        for (const col of fieldsToApply) applyBatchColumn(productInput, col, r[col]);
        await api.saveProduct(productInput);
        okCount++;
      } catch (err) {
        batchImportLog.push({ row: i + 2, name: barcode, reason: err instanceof Error ? err.message : String(err) });
      }
    }
    batchImportProgress = { done: parsed.length, total: parsed.length };
    batchImporting = false;
    showToast(
      `Batch update selesai: ${okCount} barang diperbarui${batchImportLog.length ? `, ${batchImportLog.length} baris gagal` : ""}.`,
      batchImportLog.length ? "error" : "success",
      6000,
    );
    await load();
  }

  let rows = $state<ProductWithStock[]>([]);
  let total = $state(0);
  let page = $state(0);
  let search = $state("");
  let brands = $state<Brand[]>([]);
  let brandFilter = $state("");
  let savingId = $state<string | null>(null);
  let loading = $state(false);

  const totalPages = $derived(Math.max(1, Math.ceil(total / PAGE_SIZE)));

  async function load() {
    loading = true;
    try {
      const res = await api.listProductsPage({
        search,
        includeInactive: true,
        brand: brandFilter || null,
        sortBy: "name",
        sortDir: "asc",
        limit: PAGE_SIZE,
        offset: page * PAGE_SIZE,
      });
      rows = res.items;
      total = res.total;
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }

  const debouncedSearch = debounce(() => {
    page = 0;
    load();
  }, 300);
  function onSearchInput() {
    debouncedSearch();
  }
  function onFilterChange() {
    page = 0;
    load();
  }
  function goPage(delta: number) {
    const next = page + delta;
    if (next < 0 || next >= totalPages) return;
    page = next;
    load();
  }

  onMount(async () => {
    load();
    try {
      brands = await api.listBrands();
    } catch (e) {
      toastError(e);
    }
  });

  async function saveRow(r: ProductWithStock) {
    savingId = r.id;
    try {
      await api.saveProduct({
        id: r.id,
        name: r.name,
        barcode: r.barcode, // tidak diubah
        category: r.category,
        brand: r.brand,
        unit: r.unit,
        sell_price: r.sell_price,
        cost_price: r.cost_price,
        default_discount: r.default_discount,
        is_active: r.is_active,
      });
      showToast(`"${r.name}" tersimpan.`, "success");
    } catch (e) {
      toastError(e);
    } finally {
      savingId = null;
    }
  }
</script>

<div class="view-flex">
<div class="page-head">
  <h1>Data Sheet</h1>
  <div class="row" style="flex-wrap:wrap;">
    <input placeholder="Cari nama / barcode…" bind:value={search} oninput={onSearchInput} style="width:220px;" />
    <select bind:value={brandFilter} onchange={onFilterChange} style="width:150px;">
      <option value="">Semua Merek</option>
      {#each brands as b}<option value={b.name}>{b.name}</option>{/each}
    </select>
    <button class="btn-ghost" title="Muat ulang" onclick={load}>↻ Refresh</button>
  </div>
</div>

<div class="card" style="margin-bottom:0.8rem; flex-shrink:0;">
  <div class="row" style="align-items:center; gap:0.6rem; flex-wrap:wrap;">
    <label style="margin:0;">Batch Edit — Field yang diubah:</label>
    <select bind:value={batchFieldKey} style="width:180px;">
      {#each BATCH_FIELD_OPTIONS as o}<option value={o.key}>{o.label}</option>{/each}
    </select>
    <button onclick={openBatchTemplate}>📄 Unduh Template</button>
    <label class="text-dim" style="font-size:0.82rem;">📁 Import Excel:</label>
    <input type="file" accept=".xlsx,.xls,.csv" onchange={onBatchImportFile} disabled={batchImporting} />
  </div>
  {#if batchImporting || batchImportLog.length}
    <div style="margin-top:0.6rem;">
      {#if batchImporting}
        <div class="import-progress-bar">
          <div class="import-progress-fill" style="width:{batchImportProgress.total ? (batchImportProgress.done / batchImportProgress.total) * 100 : 0}%"></div>
        </div>
        <p class="text-dim" style="font-size:0.8rem; margin-top:0.3rem;">
          {batchImportProgress.done} / {batchImportProgress.total} baris diproses…
        </p>
      {/if}
      {#if batchImportLog.length}
        <div class="import-log-head">
          <span>📋 Log Import</span>
          <span class="log-err-badge">{batchImportLog.length} bermasalah</span>
        </div>
        <div class="import-log">
          {#each batchImportLog as l}
            <div class="log-row log-error">
              <span class="mono">#{l.row}</span>
              <span>{l.name}</span>
              <span class="text-dim">{l.reason}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<div class="card list-card">
  <div class="list-scroll">
    <table>
      <thead>
        <tr>
          <th style="width:140px;">Barcode</th>
          <th>Nama</th><th>Kategori</th><th>Merek</th><th>Satuan</th>
          <th class="text-right">H.Pokok</th><th class="text-right">H.Jual</th>
          <th class="text-right">Diskon</th><th>Aktif</th><th></th>
        </tr>
      </thead>
      <tbody>
        {#each rows as r (r.id)}
          <tr>
            <td class="mono text-dim">{r.barcode ?? "-"}</td>
            <td><input bind:value={r.name} /></td>
            <td><input bind:value={r.category} /></td>
            <td><input bind:value={r.brand} /></td>
            <td><input bind:value={r.unit} style="width:70px;" /></td>
            <td><input class="mono" inputmode="decimal" value={formatMoneyInput(r.cost_price)} oninput={(e) => onMoneyInput(e, (n) => (r.cost_price = n))} style="width:100px; text-align:right;" /></td>
            <td><input class="mono" inputmode="decimal" value={formatMoneyInput(r.sell_price)} oninput={(e) => onMoneyInput(e, (n) => (r.sell_price = n))} style="width:100px; text-align:right;" /></td>
            <td><input class="mono" inputmode="decimal" value={formatMoneyInput(r.default_discount)} oninput={(e) => onMoneyInput(e, (n) => (r.default_discount = n))} style="width:90px; text-align:right;" /></td>
            <td><input type="checkbox" bind:checked={r.is_active} style="width:auto;" /></td>
            <td>
              <button class="btn-primary" disabled={savingId === r.id} onclick={() => saveRow(r)}>Simpan</button>
            </td>
          </tr>
        {:else}
          <tr><td colspan="10" class="text-dim">{loading ? "Memuat…" : "Belum ada barang."}</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
  <div class="pager">
    <span class="text-dim" style="font-size:0.82rem;">
      {total.toLocaleString("id-ID")} barang · Hal {page + 1} / {totalPages}
    </span>
    <div class="row" style="gap:0.3rem;">
      <button disabled={page === 0} onclick={() => goPage(-1)}>‹ Sebelumnya</button>
      <button disabled={page + 1 >= totalPages} onclick={() => goPage(1)}>Berikutnya ›</button>
    </div>
  </div>
</div>
<p class="text-dim" style="font-size:0.78rem; flex-shrink:0;">Barcode tidak bisa diubah di sini. Semua kolom lain bisa diedit langsung lalu klik <b>Simpan</b> per baris.</p>
</div>

<style>
  .view-flex { height:100%; min-height:0; display:flex; flex-direction:column; }
  .list-card { padding:0; overflow:hidden; flex:1; min-height:0; display:flex; flex-direction:column; }
  .list-scroll { flex:1; min-height:0; overflow:auto; }
  .pager {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.5rem 0.9rem; border-top: 1px solid var(--border); flex-shrink:0;
  }

  .import-progress-bar { height: 8px; border-radius: 999px; background: var(--baby-blue-bg); overflow: hidden; }
  .import-progress-fill { height: 100%; background: var(--primary); transition: width 0.15s linear; }
  .import-log-head { display: flex; align-items: center; gap: 0.5rem; font-weight: 650; font-size: 0.85rem; margin-top: 0.8rem; margin-bottom: 0.3rem; }
  .log-err-badge { font-size: 0.7rem; font-weight: 700; padding: 0.1rem 0.4rem; border-radius: 999px; background: var(--danger); color: #fff; }
  .import-log { max-height: 200px; overflow-y: auto; border: 1px solid var(--border); border-radius: 8px; }
  .log-row { display: grid; grid-template-columns: 3.5rem 1fr 2fr; gap: 0.5rem; padding: 0.3rem 0.6rem; font-size: 0.8rem; border-bottom: 1px solid var(--border); }
  .log-row:last-child { border-bottom: none; }
  .log-error { background: color-mix(in srgb, var(--danger) 10%, transparent); }
</style>
