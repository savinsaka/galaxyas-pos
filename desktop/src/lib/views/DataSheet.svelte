<script lang="ts">
  import { onMount } from "svelte";
  import * as XLSX from "xlsx";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { api } from "$lib/api";
  import { confirmDialog } from "$lib/dialog";
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

  /** Kolom Excel → field produk. */
  const COL_FIELD: Record<string, keyof ProductInput> = {
    nama: "name", kategori: "category", merek: "brand", satuan: "unit",
    harga_pokok: "cost_price", harga_jual: "sell_price", default_diskon: "default_discount", aktif: "is_active",
  };
  const NUMERIC_COLS = new Set(["harga_pokok", "harga_jual", "default_diskon"]);

  interface BatchLogEntry { row: number; name: string; reason: string }

  let batchFieldKey = $state("all");
  const batchFieldOption = $derived(BATCH_FIELD_OPTIONS.find((o) => o.key === batchFieldKey)!);
  let batchImporting = $state(false);
  let batchAnalyzing = $state(false);
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

  /** Terapkan satu kolom Excel ke input. Kembalikan pesan kalau nilainya tidak sah.
   *  Sel angka yang kosong = tidak diubah (dulu jadi 0 — rawan harga nol). */
  function applyBatchColumn(input: ProductInput, col: string, raw: unknown): string | null {
    if (NUMERIC_COLS.has(col)) {
      if (raw === "" || raw == null) return null;
      const n = typeof raw === "number" ? raw : Number(String(raw).trim());
      if (!Number.isFinite(n) || n < 0) return `Nilai ${col} "${raw}" bukan angka`;
      if (col === "harga_pokok") input.cost_price = n;
      else if (col === "harga_jual") input.sell_price = n;
      else input.default_discount = n;
      return null;
    }
    switch (col) {
      case "nama": { const v = String(raw ?? "").trim(); if (v) input.name = v; break; }
      case "kategori": input.category = String(raw ?? "").trim() || null; break;
      case "merek": input.brand = String(raw ?? "").trim() || null; break;
      case "satuan": input.unit = String(raw ?? "").trim() || null; break;
      case "aktif": input.is_active = parseAktif(raw); break;
    }
    return null;
  }

  /** Samakan "" / null / undefined supaya tidak dianggap berubah. */
  function norm(v: unknown): string {
    return v == null ? "" : String(v).trim();
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

  // ---------- Preview batch: dianalisa dulu, baru ditulis setelah "Terapkan" ----------
  interface PreviewChange { row: number; before: ProductInput; after: ProductInput; changed: string[] }
  interface PreviewProblem { row: number; barcode: string; data: string; reason: string }
  interface BatchPreview {
    fileName: string;
    option: BatchFieldOption;
    changes: PreviewChange[];
    same: number;
    notFound: number;
    emptyBarcode: number;
    duplicate: number;
    invalid: number;
    problems: PreviewProblem[];
  }
  const PREVIEW_LIMIT = 500;
  let batchPreview = $state<BatchPreview | null>(null);
  let previewTab = $state<"changes" | "problems">("changes");

  function rowData(r: Record<string, unknown>): string {
    return Object.entries(r)
      .filter(([k, v]) => k !== "barcode" && norm(v) !== "")
      .map(([k, v]) => `${k}: ${v}`)
      .join(" · ");
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

    const option = batchFieldOption;
    const fields = option.columns.filter((c) => c !== "barcode");
    const missing = fields.filter((c) => !(c in parsed[0]));
    if (missing.length === fields.length) {
      return showToast(`Kolom ${fields.join(", ")} tidak ada di file. Pastikan "Field yang diubah" sama dengan template yang dipakai.`, "error", 8000);
    }

    batchAnalyzing = true;
    batchImportLog = [];
    batchImportProgress = { done: 0, total: parsed.length };
    const pv: BatchPreview = {
      fileName: file.name, option, changes: [], same: 0, notFound: 0, emptyBarcode: 0, duplicate: 0, invalid: 0, problems: [],
    };
    const seen = new Map<string, number>();
    try {
      for (let i = 0; i < parsed.length; i++) {
        const r = parsed[i];
        const row = i + 2;
        batchImportProgress = { done: i, total: parsed.length };
        const barcode = String(r.barcode ?? "").trim();
        if (!barcode) {
          // Baris benar-benar kosong (sisa format Excel) dilewati diam-diam.
          if (rowData(r)) {
            pv.emptyBarcode++;
            pv.problems.push({ row, barcode: "(kosong)", data: rowData(r), reason: "Barcode kosong" });
          }
          continue;
        }
        if (seen.has(barcode)) {
          pv.duplicate++;
          pv.problems.push({ row, barcode, data: rowData(r), reason: `Barcode dobel (sama dengan baris #${seen.get(barcode)}) — dilewati` });
          continue;
        }
        seen.set(barcode, row);
        const p = await api.findByBarcode(barcode);
        if (!p) {
          pv.notFound++;
          pv.problems.push({ row, barcode, data: rowData(r), reason: "Barang tidak ditemukan" });
          continue;
        }
        const before = productToInput(p);
        const after = { ...before };
        const errors = fields.map((c) => applyBatchColumn(after, c, r[c])).filter((x): x is string => !!x);
        if (errors.length) {
          pv.invalid++;
          pv.problems.push({ row, barcode, data: rowData(r), reason: errors.join("; ") });
          continue;
        }
        const changed = fields.filter((c) => norm(before[COL_FIELD[c]]) !== norm(after[COL_FIELD[c]]));
        if (changed.length) pv.changes.push({ row, before, after, changed });
        else pv.same++;
      }
    } catch (err) {
      toastError(err);
      return;
    } finally {
      batchAnalyzing = false;
    }
    previewTab = pv.changes.length ? "changes" : "problems";
    batchPreview = pv;
  }

  async function applyBatch() {
    const pv = batchPreview;
    if (!pv || !pv.changes.length) return;
    batchPreview = null;
    batchImporting = true;
    // Baris yang dilewati ikut dicatat supaya tetap terlihat setelah preview ditutup.
    const log: BatchLogEntry[] = pv.problems.map((p) => ({ row: p.row, name: p.barcode, reason: p.reason }));
    batchImportProgress = { done: 0, total: pv.changes.length };
    let okCount = 0;
    for (let i = 0; i < pv.changes.length; i++) {
      const c = pv.changes[i];
      batchImportProgress = { done: i, total: pv.changes.length };
      try {
        await api.saveProduct(c.after);
        okCount++;
      } catch (err) {
        log.push({ row: c.row, name: c.after.barcode ?? c.after.name, reason: err instanceof Error ? err.message : String(err) });
      }
    }
    batchImportProgress = { done: pv.changes.length, total: pv.changes.length };
    batchImporting = false;
    batchImportLog = log.sort((a, b) => a.row - b.row);
    const failed = log.length - pv.problems.length;
    showToast(
      `Batch update selesai: ${okCount} barang diperbarui${failed ? `, ${failed} gagal disimpan` : ""}${pv.problems.length ? `, ${pv.problems.length} baris dilewati` : ""}.`,
      failed ? "error" : "success",
      6000,
    );
    await load();
  }

  const COL_LABEL: Record<string, string> = {
    nama: "Nama", kategori: "Kategori", merek: "Merek", satuan: "Satuan",
    harga_pokok: "H.Pokok", harga_jual: "H.Jual", default_diskon: "Diskon", aktif: "Aktif",
  };
  const PREVIEW_COLS = ["nama", "kategori", "merek", "satuan", "harga_pokok", "harga_jual", "default_diskon", "aktif"];

  function fmtCell(col: string, v: unknown): string {
    if (col === "aktif") return v ? "Ya" : "Tidak";
    if (NUMERIC_COLS.has(col)) return Number(v ?? 0).toLocaleString("id-ID");
    return norm(v) || "-";
  }

  // ---------- Daftar & edit langsung ----------
  let rows = $state<ProductWithStock[]>([]);
  let total = $state(0);
  let page = $state(0);
  /** Isi kotak cari (yang diketik) vs pencarian yang sedang dimuat. */
  let searchInput = $state("");
  let search = $state("");
  let brands = $state<Brand[]>([]);
  /** Merek yang sedang dimuat. Terpilih = mode simpan per lembar. */
  let brandFilter = $state("");
  let savingId = $state<string | null>(null);
  let savingAll = $state(false);
  let loading = $state(false);

  const EDIT_KEYS = ["name", "category", "brand", "unit", "cost_price", "sell_price", "default_discount", "is_active"] as const;
  /** Nilai asli tiap baris saat dimuat / terakhir disimpan. */
  let orig = $state<Record<string, ProductWithStock>>({});

  function isDirty(r: ProductWithStock): boolean {
    const o = orig[r.id];
    return !!o && EDIT_KEYS.some((k) => norm(r[k]) !== norm(o[k]));
  }

  const dirtyRows = $derived(rows.filter(isDirty));
  const pageMode = $derived(brandFilter !== "");
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
      orig = Object.fromEntries(res.items.map((p) => [p.id, { ...p }]));
      rows = res.items;
      total = res.total;
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }

  /** Ada perubahan yang belum disimpan → minta konfirmasi sebelum dibuang. */
  async function confirmDiscard(): Promise<boolean> {
    const n = dirtyRows.length;
    if (!n) return true;
    return confirmDialog(
      `${n} baris yang diubah di lembar ini belum disimpan dan akan hilang. Lanjutkan?`,
      { title: "Perubahan belum disimpan", okText: "Buang perubahan", cancelText: "Kembali", danger: true, icon: "⚠️" },
    );
  }

  const debouncedSearch = debounce(async () => {
    if (searchInput === search) return;
    if (!(await confirmDiscard())) {
      searchInput = search;
      return;
    }
    search = searchInput;
    page = 0;
    load();
  }, 300);

  async function onBrandChange(e: Event) {
    const sel = e.currentTarget as HTMLSelectElement;
    const next = sel.value;
    if (!(await confirmDiscard())) {
      sel.value = brandFilter;
      return;
    }
    brandFilter = next;
    page = 0;
    load();
  }

  async function goPage(delta: number) {
    const next = page + delta;
    if (next < 0 || next >= totalPages) return;
    if (!(await confirmDiscard())) return;
    page = next;
    load();
  }

  async function refresh() {
    if (await confirmDiscard()) load();
  }

  onMount(async () => {
    load();
    try {
      brands = await api.listBrands();
    } catch (e) {
      toastError(e);
    }
  });

  function revertRow(r: ProductWithStock) {
    const o = orig[r.id];
    if (!o) return;
    for (const k of EDIT_KEYS) (r as unknown as Record<string, unknown>)[k] = o[k];
  }

  async function persist(r: ProductWithStock) {
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
    orig[r.id] = { ...$state.snapshot(r) };
  }

  async function saveRow(r: ProductWithStock) {
    savingId = r.id;
    try {
      await persist(r);
      showToast(`"${r.name}" tersimpan.`, "success");
    } catch (e) {
      toastError(e);
    } finally {
      savingId = null;
    }
  }

  async function saveAll() {
    const list = [...dirtyRows];
    if (!list.length) return;
    savingAll = true;
    const failed: string[] = [];
    for (const r of list) {
      try {
        await persist(r);
      } catch (e) {
        failed.push(`${r.name}: ${e instanceof Error ? e.message : String(e)}`);
      }
    }
    savingAll = false;
    if (failed.length) {
      showToast(`${list.length - failed.length} baris tersimpan, ${failed.length} gagal (tetap ditandai):\n${failed.join("\n")}`, "error", 8000);
    } else {
      showToast(`${list.length} baris tersimpan.`, "success");
    }
  }

  async function cancelAll() {
    const n = dirtyRows.length;
    if (!n) return;
    if (!(await confirmDialog(`Batalkan ${n} perubahan di lembar ini?`, { title: "Batalkan perubahan?", okText: "Ya, batalkan", cancelText: "Kembali" }))) return;
    for (const r of dirtyRows) revertRow(r);
  }
</script>

<div class="view-flex">
<div class="page-head">
  <h1>Data Sheet</h1>
  <div class="row" style="flex-wrap:wrap;">
    <input placeholder="Cari nama / barcode…" bind:value={searchInput} oninput={debouncedSearch} style="width:220px;" />
    <select value={brandFilter} onchange={onBrandChange} style="width:150px;">
      <option value="">Semua Merek</option>
      {#each brands as b}<option value={b.name}>{b.name}</option>{/each}
    </select>
    <button class="btn-ghost" title="Muat ulang" onclick={refresh}>↻ Refresh</button>
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
    <input type="file" accept=".xlsx,.xls,.csv" onchange={onBatchImportFile} disabled={batchImporting || batchAnalyzing} />
  </div>
  {#if batchImporting || batchAnalyzing || batchImportLog.length}
    <div style="margin-top:0.6rem;">
      {#if batchImporting || batchAnalyzing}
        <div class="import-progress-bar">
          <div class="import-progress-fill" style="width:{batchImportProgress.total ? (batchImportProgress.done / batchImportProgress.total) * 100 : 0}%"></div>
        </div>
        <p class="text-dim" style="font-size:0.8rem; margin-top:0.3rem;">
          {batchImportProgress.done} / {batchImportProgress.total} baris
          {batchAnalyzing ? "diperiksa (belum ada yang diubah)…" : "disimpan…"}
        </p>
      {/if}
      {#if batchImportLog.length}
        <div class="import-log-head">
          <span>📋 Log Import</span>
          <span class="log-err-badge">{batchImportLog.length} bermasalah / dilewati</span>
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
  {#if pageMode}
    <div class="sheet-bar" class:has={dirtyRows.length > 0}>
      <span class="sheet-info">
        Merek <b>{brandFilter}</b> — simpan per lembar.
        {#if dirtyRows.length}<b>{dirtyRows.length} baris berubah</b> di lembar ini.{:else}Belum ada perubahan.{/if}
      </span>
      <button class="btn-ghost" disabled={!dirtyRows.length || savingAll} onclick={cancelAll}>Batal</button>
      <button class="btn-primary" disabled={!dirtyRows.length || savingAll} onclick={saveAll}>
        {savingAll ? "Menyimpan…" : `💾 Simpan Semua${dirtyRows.length ? ` (${dirtyRows.length})` : ""}`}
      </button>
    </div>
  {/if}
  <div class="list-scroll">
    <table>
      <thead>
        <tr>
          <th style="width:140px;">Barcode</th>
          <th>Nama</th><th>Kategori</th><th>Merek</th><th>Satuan</th>
          <th class="text-right">H.Pokok</th><th class="text-right">H.Jual</th>
          <th class="text-right">Diskon</th><th>Aktif</th>
          {#if !pageMode}<th></th>{/if}
        </tr>
      </thead>
      <tbody>
        {#each rows as r (r.id)}
          {@const dirty = isDirty(r)}
          <tr class:dirty>
            <td class="mono text-dim">{r.barcode ?? "-"}</td>
            <td><input bind:value={r.name} /></td>
            <td><input bind:value={r.category} /></td>
            <td><input bind:value={r.brand} /></td>
            <td><input bind:value={r.unit} style="width:70px;" /></td>
            <td><input class="mono" inputmode="decimal" value={formatMoneyInput(r.cost_price)} oninput={(e) => onMoneyInput(e, (n) => (r.cost_price = n))} style="width:100px; text-align:right;" /></td>
            <td><input class="mono" inputmode="decimal" value={formatMoneyInput(r.sell_price)} oninput={(e) => onMoneyInput(e, (n) => (r.sell_price = n))} style="width:100px; text-align:right;" /></td>
            <td><input class="mono" inputmode="decimal" value={formatMoneyInput(r.default_discount)} oninput={(e) => onMoneyInput(e, (n) => (r.default_discount = n))} style="width:90px; text-align:right;" /></td>
            <td><input type="checkbox" bind:checked={r.is_active} style="width:auto;" /></td>
            {#if !pageMode}
              <td class="row-actions">
                <button class="btn-primary" disabled={!dirty || savingId === r.id} onclick={() => saveRow(r)}
                  title={dirty ? "Simpan baris ini" : "Belum ada perubahan di baris ini"}>Simpan</button>
                <button class="btn-ghost" disabled={!dirty || savingId === r.id} onclick={() => revertRow(r)}
                  title="Kembalikan nilai sebelum diubah">Batal</button>
              </td>
            {/if}
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
<p class="text-dim" style="font-size:0.78rem; flex-shrink:0;">
  Barcode tidak bisa diubah di sini.
  {#if pageMode}Merek dipilih: ubah beberapa baris lalu klik <b>Simpan Semua</b> — pindah lembar sebelum disimpan akan membuang perubahan.
  {:else}Ubah kolom lalu klik <b>Simpan</b> di baris itu. Pilih merek untuk menyimpan satu lembar sekaligus.{/if}
</p>
</div>

{#if batchPreview}
  {@const pv = batchPreview}
  <div class="modal-backdrop" role="presentation">
    <div class="modal bp-modal" role="dialog" aria-label="Preview Batch Edit">
      <div class="bp-head">
        <h2>Preview Batch Edit</h2>
        <span class="text-dim" style="font-size:0.82rem;">{pv.fileName} · Field: <b>{pv.option.label}</b> · belum ada yang diubah</span>
      </div>
      <div class="bp-chips">
        <span class="chip chip-ok">{pv.changes.length} akan diubah</span>
        <span class="chip">{pv.same} sama (tidak berubah)</span>
        <span class="chip" class:chip-bad={pv.notFound > 0}>{pv.notFound} tidak ditemukan</span>
        {#if pv.emptyBarcode}<span class="chip chip-bad">{pv.emptyBarcode} barcode kosong</span>{/if}
        {#if pv.duplicate}<span class="chip chip-bad">{pv.duplicate} barcode dobel</span>{/if}
        {#if pv.invalid}<span class="chip chip-bad">{pv.invalid} nilai tidak sah</span>{/if}
      </div>
      <div class="bp-tabs">
        <button class:on={previewTab === "changes"} onclick={() => (previewTab = "changes")}>Akan diubah ({pv.changes.length})</button>
        <button class:on={previewTab === "problems"} onclick={() => (previewTab = "problems")}>
          Tidak ditemukan / bermasalah ({pv.problems.length})
        </button>
      </div>
      <div class="bp-scroll">
        {#if previewTab === "changes"}
          <table class="bp-table">
            <thead>
              <tr>
                <th>#</th><th>Barcode</th>
                {#each PREVIEW_COLS as c}
                  <th class:edited={pv.option.columns.includes(c)} class:text-right={NUMERIC_COLS.has(c)}>{COL_LABEL[c]}</th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each pv.changes.slice(0, PREVIEW_LIMIT) as c (c.row)}
                <tr>
                  <td class="mono text-dim">{c.row}</td>
                  <td class="mono">{c.before.barcode}</td>
                  {#each PREVIEW_COLS as col}
                    {@const f = COL_FIELD[col]}
                    {#if c.changed.includes(col)}
                      <td class="chg" class:text-right={NUMERIC_COLS.has(col)}>
                        <span class="old">{fmtCell(col, c.before[f])}</span>
                        <span class="arrow">→</span>
                        <span class="new">{fmtCell(col, c.after[f])}</span>
                      </td>
                    {:else}
                      <td class:text-right={NUMERIC_COLS.has(col)}>{fmtCell(col, c.after[f])}</td>
                    {/if}
                  {/each}
                </tr>
              {:else}
                <tr><td colspan="10" class="text-dim">Tidak ada barang yang berubah.</td></tr>
              {/each}
            </tbody>
          </table>
          {#if pv.changes.length > PREVIEW_LIMIT}
            <p class="text-dim bp-more">…dan {pv.changes.length - PREVIEW_LIMIT} baris lain (semua tetap diterapkan).</p>
          {/if}
        {:else}
          <table class="bp-table">
            <thead><tr><th>Baris Excel</th><th>Barcode</th><th>Isi di file</th><th>Alasan</th></tr></thead>
            <tbody>
              {#each pv.problems as p (p.row)}
                <tr>
                  <td class="mono">#{p.row}</td>
                  <td class="mono">{p.barcode}</td>
                  <td class="text-dim">{p.data || "-"}</td>
                  <td class="reason">{p.reason}</td>
                </tr>
              {:else}
                <tr><td colspan="4" class="text-dim">Semua baris ditemukan. 👍</td></tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
      {#if pv.problems.length}
        <div class="bp-warn">
          ⚠️ {pv.problems.length} baris <b>tidak akan diproses</b> — cek tab "Tidak ditemukan / bermasalah" sebelum menerapkan.
        </div>
      {/if}
      <div class="bp-foot">
        <button class="btn-ghost" onclick={() => (batchPreview = null)}>Batal</button>
        <button class="btn-primary" disabled={!pv.changes.length} onclick={applyBatch}>
          Terapkan {pv.changes.length} perubahan
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .view-flex { height:100%; min-height:0; display:flex; flex-direction:column; }
  .list-card { padding:0; overflow:hidden; flex:1; min-height:0; display:flex; flex-direction:column; }
  .list-scroll { flex:1; min-height:0; overflow:auto; }
  .pager {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.5rem 0.9rem; border-top: 1px solid var(--border); flex-shrink:0;
  }

  /* Baris yang sudah diubah tapi belum disimpan. */
  tr.dirty td { background: color-mix(in srgb, var(--warning) 14%, transparent); }
  tr.dirty td:first-child { box-shadow: inset 3px 0 0 var(--warning); }
  .row-actions { white-space: nowrap; }
  .row-actions button { padding: 0.3rem 0.6rem; }
  .row-actions button + button { margin-left: 0.25rem; }

  .sheet-bar {
    display: flex; align-items: center; gap: 0.5rem; flex-shrink: 0;
    padding: 0.5rem 0.9rem; border-bottom: 1px solid var(--border);
    background: var(--baby-blue-bg); font-size: 0.85rem;
  }
  .sheet-bar.has { background: color-mix(in srgb, var(--warning) 18%, var(--panel)); }
  .sheet-info { flex: 1; }

  .import-progress-bar { height: 8px; border-radius: 999px; background: var(--baby-blue-bg); overflow: hidden; }
  .import-progress-fill { height: 100%; background: var(--primary); transition: width 0.15s linear; }
  .import-log-head { display: flex; align-items: center; gap: 0.5rem; font-weight: 650; font-size: 0.85rem; margin-top: 0.8rem; margin-bottom: 0.3rem; }
  .log-err-badge { font-size: 0.7rem; font-weight: 700; padding: 0.1rem 0.4rem; border-radius: 999px; background: var(--danger); color: #fff; }
  .import-log { max-height: 200px; overflow-y: auto; border: 1px solid var(--border); border-radius: 8px; }
  .log-row { display: grid; grid-template-columns: 3.5rem 1fr 2fr; gap: 0.5rem; padding: 0.3rem 0.6rem; font-size: 0.8rem; border-bottom: 1px solid var(--border); }
  .log-row:last-child { border-bottom: none; }
  .log-error { background: color-mix(in srgb, var(--danger) 10%, transparent); }

  /* ----- Preview batch ----- */
  .bp-modal {
    width: 1150px; max-width: 95vw; max-height: 90vh;
    display: flex; flex-direction: column; gap: 0.6rem; color: var(--text);
  }
  .bp-head h2 { margin: 0 0 0.15rem; font-size: 1.15rem; }
  .bp-chips { display: flex; gap: 0.4rem; flex-wrap: wrap; }
  .chip {
    font-size: 0.78rem; padding: 0.2rem 0.6rem; border-radius: 999px;
    background: var(--baby-blue-bg); border: 1px solid var(--border);
  }
  .chip-ok { background: color-mix(in srgb, var(--success) 18%, var(--panel)); font-weight: 650; }
  .chip-bad { background: color-mix(in srgb, var(--danger) 16%, var(--panel)); font-weight: 650; }
  .bp-tabs { display: flex; gap: 0.3rem; border-bottom: 1px solid var(--border); }
  .bp-tabs button { border-radius: 6px 6px 0 0; font-size: 0.82rem; }
  .bp-tabs button.on { background: var(--primary); color: var(--white); }
  .bp-scroll { flex: 1; min-height: 160px; overflow: auto; border: 1px solid var(--border); border-radius: 8px; }
  .bp-table { font-size: 0.8rem; }
  .bp-table th { position: sticky; top: 0; z-index: 1; background: var(--baby-blue-bg); white-space: nowrap; }
  .bp-table th.edited { box-shadow: inset 0 -3px 0 var(--primary); }
  .bp-table td { white-space: nowrap; max-width: 240px; overflow: hidden; text-overflow: ellipsis; }
  .bp-table td.chg { background: color-mix(in srgb, var(--warning) 20%, transparent); }
  .old { text-decoration: line-through; color: var(--text-dim); }
  .arrow { margin: 0 0.25rem; color: var(--text-dim); }
  .new { font-weight: 700; }
  .reason { color: var(--danger); font-weight: 600; white-space: normal; }
  .bp-more { font-size: 0.78rem; padding: 0.4rem 0.6rem; margin: 0; }
  .bp-warn {
    font-size: 0.82rem; padding: 0.45rem 0.7rem; border-radius: 8px;
    background: color-mix(in srgb, var(--warning) 20%, var(--panel));
  }
  .bp-foot { display: flex; justify-content: flex-end; gap: 0.5rem; }
</style>
