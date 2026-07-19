<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import * as XLSX from "xlsx";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { api } from "$lib/api";
  import { formatQty, formatDateTime, formatTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import { currentMonthBounds } from "$lib/monthPager";
  import { createLiveClock } from "$lib/liveClock.svelte";
  import { todayIso, combineDateAndTime } from "$lib/dateTime";
  import type { ProductWithStock, StockMovement } from "$lib/types";
  import { setTabDirty, clearTabDirty } from "$lib/stores/tabGuard";
  import MonthPager from "$lib/components/MonthPager.svelte";
  import ProductSearchPopup from "$lib/components/ProductSearchPopup.svelte";

  let { tabId }: { tabId?: string } = $props();

  const clock = createLiveClock();
  onDestroy(() => clock.stop());
  onDestroy(() => { if (tabId) clearTabDirty(tabId); });

  const IMPORT_HEADERS = ["kode_barcode", "nama", "fisik", "keterangan"];
  const IMPORT_EXAMPLE = [
    ["8992388101010", "Indomie Goreng", 48, "opsional"],
    ["8993675001020", "Aqua 600ml", 20, ""],
  ];

  interface ImportLogEntry {
    row: number;
    name: string;
    reason: string;
  }

  let importing = $state(false);
  let importProgress = $state({ done: 0, total: 0 });
  let importLog = $state<ImportLogEntry[]>([]);

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
  $effect(() => {
    if (tabId) setTabDirty(tabId, selected !== null);
  });
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

  async function openImportTemplate() {
    try {
      const ws = XLSX.utils.aoa_to_sheet([IMPORT_HEADERS, ...IMPORT_EXAMPLE]);
      (ws as Record<string, unknown>)["!protect"] = {
        selectLockedCells: true,
        selectUnlockedCells: true,
      };
      const wb = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(wb, ws, "Template");
      const out = XLSX.write(wb, { type: "array", bookType: "xlsx" }) as ArrayBuffer;
      const bytes = Array.from(new Uint8Array(out));
      const path = await api.writeTempFile("Template-Stok-Opname.xlsx", bytes);
      await openPath(path);
      showToast("Template dibuka di Excel (read-only). Pilih Save As untuk mengisi & menyimpan.", "info", 7000);
    } catch (e) {
      toastError(e);
    }
  }

  async function onImportFile(e: Event) {
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

    importing = true;
    importLog = [];
    importProgress = { done: 0, total: parsed.length };

    let okCount = 0;
    for (let i = 0; i < parsed.length; i++) {
      const r = parsed[i];
      importProgress = { done: i, total: parsed.length };
      const kode = String(r.kode_barcode ?? r.kode ?? r.barcode ?? "").trim();
      const fisikVal = Number(r.fisik);
      const ket = String(r.keterangan ?? "").trim();

      if (!kode) {
        importLog.push({ row: i + 2, name: "(kosong)", reason: "Kode/barcode kosong" });
        continue;
      }
      if (!Number.isFinite(fisikVal) || fisikVal < 0) {
        importLog.push({ row: i + 2, name: kode, reason: "Fisik tidak valid" });
        continue;
      }
      try {
        const p = await api.findByBarcode(kode);
        if (!p) {
          importLog.push({ row: i + 2, name: kode, reason: "Barang tidak ditemukan" });
          continue;
        }
        await api.createStockMovement({
          product_id: p.id,
          kind: "opname",
          qty: fisikVal,
          note: ket || "Stok opname (import)",
          user_id: $currentUser?.username ?? null,
          created_at: combineDateAndTime(tanggal, clock.now),
        });
        okCount++;
      } catch (err) {
        importLog.push({ row: i + 2, name: kode, reason: err instanceof Error ? err.message : String(err) });
      }
    }
    importProgress = { done: parsed.length, total: parsed.length };
    importing = false;
    showToast(
      `Import opname selesai: ${okCount} baris tersimpan${importLog.length ? `, ${importLog.length} baris gagal` : ""}.`,
      importLog.length ? "error" : "success",
      6000,
    );
    tanggal = todayIso();
    await load();
  }
</script>

<div class="view-flex">
<div class="page-head"><h1>Stok Opname</h1></div>

<div class="op-grid">
  <!-- Form input -->
  <div class="card form-card" style="overflow-y:auto;">
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

    <!-- Import Excel -->
    <div class="import-block">
      <div class="row" style="align-items:center; gap:0.6rem; flex-wrap:wrap;">
        <button onclick={openImportTemplate}>📄 Unduh Template Excel</button>
        <label class="text-dim" style="font-size:0.82rem;">📁 Import Excel Opname:</label>
        <input type="file" accept=".xlsx,.xls,.csv" onchange={onImportFile} disabled={importing} />
      </div>
      {#if importing || importLog.length}
        <div style="margin-top:0.6rem;">
          {#if importing}
            <div class="import-progress-bar">
              <div
                class="import-progress-fill"
                style="width:{importProgress.total ? (importProgress.done / importProgress.total) * 100 : 0}%"
              ></div>
            </div>
            <p class="text-dim" style="font-size:0.8rem; margin-top:0.3rem;">
              {importProgress.done} / {importProgress.total} baris diproses…
            </p>
          {/if}
          {#if importLog.length}
            <div class="import-log-head">
              <span>📋 Log Import</span>
              <span class="log-err-badge">{importLog.length} bermasalah</span>
            </div>
            <div class="import-log">
              {#each importLog as l}
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
  <div class="card history-card">
    <div style="padding:0.7rem 0.9rem; border-bottom:1px solid var(--border); display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:0.5rem; flex-shrink:0;">
      <span style="font-weight:650;">Riwayat Opname Terakhir</span>
      <MonthPager bind:from bind:to onchange={load} />
    </div>
    <div class="history-scroll">
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
</div>

{#if showPopup}
  <ProductSearchPopup
    initialQuery={search}
    onClose={() => (showPopup = false)}
    onPick={(p) => { selectProduct(p); showPopup = false; }}
  />
{/if}

<style>
  .view-flex { height:100%; min-height:0; display:flex; flex-direction:column; }
  .op-grid { display: grid; grid-template-columns: 400px 1fr; gap: 1rem; flex:1; min-height:0; }
  .history-card { padding:0; overflow:hidden; display:flex; flex-direction:column; min-height:0; }
  .history-scroll { flex:1; min-height:0; overflow-y:auto; }
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

  .import-block {
    border: 1px dashed var(--border-strong); border-radius: 8px;
    padding: 0.6rem 0.7rem; margin-bottom: 0.6rem; background: var(--baby-blue-bg);
  }
  .import-progress-bar {
    height: 8px;
    border-radius: 999px;
    background: var(--white);
    overflow: hidden;
  }
  .import-progress-fill {
    height: 100%;
    background: var(--primary);
    transition: width 0.15s linear;
  }
  .import-log-head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 650;
    font-size: 0.85rem;
    margin-top: 0.8rem;
    margin-bottom: 0.3rem;
  }
  .log-err-badge {
    font-size: 0.7rem;
    font-weight: 700;
    padding: 0.1rem 0.4rem;
    border-radius: 999px;
    background: var(--danger);
    color: #fff;
  }
  .import-log {
    max-height: 200px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--white);
  }
  .log-row {
    display: grid;
    grid-template-columns: 3.5rem 1fr 2fr;
    gap: 0.5rem;
    padding: 0.3rem 0.6rem;
    font-size: 0.8rem;
    border-bottom: 1px solid var(--border);
  }
  .log-row:last-child { border-bottom: none; }
  .log-error { background: color-mix(in srgb, var(--danger) 10%, transparent); }
</style>
