<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import * as XLSX from "xlsx";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { api } from "$lib/api";
  import { formatQty } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import { todayIso, nowHHMM, combineDateAndClock } from "$lib/dateTime";
  import { setTabDirty, clearTabDirty } from "$lib/stores/tabGuard";
  import { activeTabId } from "$lib/stores/tabs";
  import type { ProductWithStock, TimeOpnameRow } from "$lib/types";
  import ShortcutBar from "$lib/components/ShortcutBar.svelte";

  let { tabId }: { tabId?: string } = $props();

  onDestroy(() => { if (tabId) clearTabDirty(tabId); });

  // Template opname diseragamkan: barcode · fisik · keterangan (opsional).
  const IMPORT_HEADERS = ["barcode", "fisik", "keterangan"];
  const IMPORT_EXAMPLE = [
    ["8992388101010", 48, "opsional"],
    ["8993675001020", 20, ""],
  ];

  interface OpRow {
    id: number;
    search: string;
    product: ProductWithStock | null;
    fisik: number;
    keterangan: string;
    dropOpen: boolean;
  }
  interface ImportLogEntry {
    row: number;
    name: string;
    reason: string;
  }

  const isAdmin = $derived($currentUser?.role === "admin");

  let products = $state<ProductWithStock[]>([]);
  let tanggal = $state(todayIso());
  let jam = $state(nowHHMM());
  let keterangan = $state("");
  let busy = $state(false);
  let previewing = $state(false);
  let showConfirm = $state(false);
  let importLog = $state<ImportLogEntry[]>([]);
  /** Hasil pratinjau dari backend, dikunci per product_id. */
  let preview = $state<Record<string, TimeOpnameRow>>({});

  let nextId = 1;
  function newRow(): OpRow {
    return { id: nextId++, search: "", product: null, fisik: 0, keterangan: "", dropOpen: false };
  }
  let rows = $state<OpRow[]>([newRow()]);

  const filledRows = $derived(rows.filter((r) => r.product !== null));
  const waktuIso = $derived(combineDateAndClock(tanggal, jam));
  const waktuLabel = $derived(
    waktuIso
      ? new Date(waktuIso).toLocaleString("id-ID", {
          day: "2-digit", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit",
        })
      : "—",
  );
  const waktuMasaDepan = $derived(!!waktuIso && new Date(waktuIso).getTime() > Date.now());
  const bisaSimpan = $derived(isAdmin && !busy && !waktuMasaDepan && !!waktuIso && filledRows.length > 0);

  $effect(() => {
    if (tabId) setTabDirty(tabId, filledRows.length > 0);
  });

  // Ganti tanggal/jam = seluruh pratinjau tidak berlaku lagi (buku & stok akhir
  // dihitung dari titik waktu itu). Kosongkan supaya tidak ada angka basi.
  $effect(() => {
    tanggal;
    jam;
    preview = {};
  });

  onMount(async () => {
    try {
      products = await api.listProducts("", true);
    } catch (e) { toastError(e); }
  });

  // F9 = Simpan, F6 = Tambah Baris — sama seperti Opname Spesial.
  function onGlobalKey(e: KeyboardEvent) {
    if (tabId && $activeTabId !== tabId) return;
    if (e.key === "F9") {
      e.preventDefault();
      if (bisaSimpan && !showConfirm) mintaKonfirmasi();
    } else if (e.key === "F6") {
      e.preventDefault();
      if (!showConfirm) addRow();
    }
  }
  onMount(() => window.addEventListener("keydown", onGlobalKey));
  onDestroy(() => window.removeEventListener("keydown", onGlobalKey));

  const filtered = (row: OpRow) => {
    const q = row.search.trim().toLowerCase();
    if (!q) return [];
    return products
      .filter((p) => p.name.toLowerCase().includes(q) || (p.barcode ?? "").toLowerCase().includes(q))
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

  async function resolveTerm(row: OpRow, term: string) {
    const t = term.trim().toLowerCase();
    const byBarcode = products.find((p) => (p.barcode ?? "").toLowerCase() === t);
    if (byBarcode) { selectProduct(row, byBarcode); return; }
    const f = filtered(row);
    if (f.length === 1) { selectProduct(row, f[0]); return; }
    if (f.length === 0) { showToast(`"${term.trim()}" tidak ditemukan.`, "error"); return; }
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
    if (e.key === "Delete" && !row.search.trim() && !row.product) {
      e.preventDefault();
      removeRow(row.id);
      return;
    }
    if (e.key !== "Enter") return;
    const term = row.search.trim();
    if (!term) return;
    await resolveTerm(row, term);
  }

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
    if (e.key === "ArrowDown" || e.key === "Enter") {
      e.preventDefault();
      advanceRow(row);
    } else if (e.key === "ArrowUp") {
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

  function resetForm() {
    rows = [newRow()];
    keterangan = "";
    preview = {};
    importLog = [];
  }

  // ---------- Template & import Excel ----------

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
      const path = await api.writeTempFile("Template-Time-Opname.xlsx", bytes);
      await openPath(path);
      showToast("Template dibuka di Excel (read-only). Pilih Save As untuk mengisi & menyimpan.", "info", 7000);
    } catch (e) {
      toastError(e);
    }
  }

  /**
   * Import mengisi TABEL, bukan langsung menyimpan — waktu opname mundur tidak
   * bisa dibatalkan, jadi angkanya harus bisa dilihat & dipratinjau dulu.
   */
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

    importLog = [];
    const hasil: OpRow[] = [];
    for (let i = 0; i < parsed.length; i++) {
      const r = parsed[i];
      // Terima juga header template lama (kode_barcode / kode).
      const kode = String(r.barcode ?? r.kode_barcode ?? r.kode ?? "").trim();
      const fisikVal = Number(r.fisik);
      const ket = String(r.keterangan ?? "").trim();

      if (!kode) {
        importLog.push({ row: i + 2, name: "(kosong)", reason: "Barcode kosong" });
        continue;
      }
      if (!Number.isFinite(fisikVal) || fisikVal < 0) {
        importLog.push({ row: i + 2, name: kode, reason: "Fisik tidak valid" });
        continue;
      }
      const p = products.find((x) => (x.barcode ?? "").trim() === kode);
      if (!p) {
        importLog.push({ row: i + 2, name: kode, reason: "Barang tidak ditemukan" });
        continue;
      }
      const dup = hasil.find((x) => x.product?.id === p.id);
      if (dup) {
        importLog.push({ row: i + 2, name: p.name, reason: "Barcode dobel di file, baris ini dilewati" });
        continue;
      }
      hasil.push({
        id: nextId++,
        search: p.name,
        product: p,
        fisik: fisikVal,
        keterangan: ket,
        dropOpen: false,
      });
    }

    if (!hasil.length) {
      showToast("Tidak ada baris yang bisa dipakai dari file itu.", "error", 5000);
      return;
    }
    rows = [...hasil, newRow()];
    preview = {};
    showToast(
      `${hasil.length} baris masuk ke tabel${importLog.length ? `, ${importLog.length} baris bermasalah` : ""}. Cek pratinjau sebelum menyimpan.`,
      importLog.length ? "error" : "success",
      6000,
    );
    await hitungPratinjau();
  }

  // ---------- Pratinjau & simpan ----------

  function inputPayload() {
    return {
      created_at: waktuIso,
      note: keterangan || null,
      user_id: $currentUser?.username ?? null,
      items: filledRows.map((r) => ({
        product_id: r.product!.id,
        qty: r.fisik,
        note: r.keterangan || null,
      })),
    };
  }

  async function hitungPratinjau(): Promise<boolean> {
    if (!waktuIso) { showToast("Tanggal/jam opname belum lengkap.", "info"); return false; }
    if (waktuMasaDepan) { showToast("Waktu opname masih di masa depan.", "error"); return false; }
    if (!filledRows.length) { showToast("Belum ada barang yang diisi.", "info"); return false; }
    previewing = true;
    try {
      const hasil = await api.previewTimeOpname(inputPayload());
      preview = Object.fromEntries(hasil.map((h) => [h.product_id, h]));
      return true;
    } catch (e) {
      toastError(e);
      return false;
    } finally {
      previewing = false;
    }
  }

  async function mintaKonfirmasi() {
    if (!bisaSimpan) return;
    if (await hitungPratinjau()) showConfirm = true;
  }

  async function simpan() {
    showConfirm = false;
    busy = true;
    try {
      const res = await api.createTimeOpname(inputPayload());
      const digeser = res.rows.filter((r) => !r.overridden_by_later_opname).length;
      const ditimpa = res.rows.length - digeser;
      showToast(
        `Time opname ${waktuLabel} tersimpan: ${res.rows.length} barang${ditimpa ? `, ${ditimpa} stoknya tetap karena ada opname lebih baru` : ""}.`,
        "success",
        7000,
      );
      resetForm();
      jam = nowHHMM();
      // Stok barang sudah bergeser — muat ulang supaya kolom "stok" di
      // dropdown dan nilai awal Fisik tidak memakai angka lama.
      products = await api.listProducts("", true);
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  const previewRows = $derived(filledRows.map((r) => preview[r.product!.id]).filter(Boolean));
</script>

<div class="page-head">
  <h1>Time Opname</h1>
  <div class="row" style="align-items:center; gap:0.6rem; flex-wrap:wrap;">
    <button onclick={openImportTemplate}>📄 Unduh Template Excel</button>
    <label class="text-dim" for="to-file" style="font-size:0.82rem;">📁 Import Excel:</label>
    <input id="to-file" type="file" accept=".xlsx,.xls,.csv" onchange={onImportFile} disabled={!isAdmin} />
  </div>
</div>

<div class="info-block">
  Hasil hitung dicatat <b>pada waktu barang itu benar-benar dihitung</b>, lalu semua penjualan,
  item masuk, dan item keluar yang terjadi <b>sesudah</b> waktu itu dihitung ulang di atasnya —
  jadi stok hari ini ikut benar tanpa perlu menghitung ulang barangnya.
</div>

{#if !isAdmin}
  <div class="warn-block">
    <strong>🔐 Khusus admin.</strong> Menu ini menulis ulang riwayat stok, jadi hanya akun admin
    yang bisa menyimpannya. Untuk opname biasa pakai menu <b>Opname</b>.
  </div>
{/if}

<!-- Header: waktu opname + keterangan -->
<div class="trx-header card">
  <div class="trx-field">
    <label for="to-tgl">Tanggal Hitung</label>
    <input id="to-tgl" type="date" max={todayIso()} disabled={!isAdmin} bind:value={tanggal} style="width:150px;" />
  </div>
  <div class="trx-field">
    <label for="to-jam">Jam Hitung</label>
    <input id="to-jam" type="time" disabled={!isAdmin} bind:value={jam} style="width:120px;" />
  </div>
  <div class="trx-field" style="flex:1; min-width:200px;">
    <label for="to-ket">Keterangan</label>
    <input id="to-ket" disabled={!isAdmin} bind:value={keterangan} placeholder="mis. berkas opname kertas 12:05" />
  </div>
  <div class="trx-field">
    <span class="text-dim" style="font-size:0.78rem;">Dicatat pada</span>
    <span class="meta-val mono" class:bad={waktuMasaDepan}>{waktuLabel}</span>
  </div>
</div>

{#if waktuMasaDepan}
  <div class="warn-block">⚠️ Waktu yang dipilih masih di masa depan — opname tidak bisa disimpan.</div>
{/if}

{#if importLog.length}
  <div class="import-block">
    <div class="import-log-head">
      <span>📋 Log Import</span>
      <span class="log-err-badge">{importLog.length} bermasalah</span>
    </div>
    <div class="import-log">
      {#each importLog as l}
        <div class="log-row">
          <span class="mono">#{l.row}</span>
          <span>{l.name}</span>
          <span class="text-dim">{l.reason}</span>
        </div>
      {/each}
    </div>
  </div>
{/if}

<!-- Tabel opname -->
<div class="card" style="padding:0; overflow:hidden; margin-bottom:0.8rem;">
  <table class="batch-table">
    <thead>
      <tr>
        <th style="width:2rem;">No</th>
        <th>Kode/Nama Item</th>
        <th class="text-right" style="width:92px;">Buku @waktu</th>
        <th style="width:110px;">Fisik</th>
        <th class="text-right" style="width:88px;">Selisih</th>
        <th class="text-right" style="width:150px;">Stok Sekarang</th>
        <th style="width:150px;">Keterangan</th>
        <th style="width:2rem;"></th>
      </tr>
    </thead>
    <tbody>
      {#each rows as row (row.id)}
        {@const pv = row.product ? preview[row.product.id] : undefined}
        <tr>
          <td class="text-dim mono" style="text-align:center;">{rows.indexOf(row) + 1}</td>
          <td style="position:relative;">
            <input
              class="cell-input"
              data-search-row={row.id}
              placeholder="Scan barcode atau ketik nama…"
              disabled={!isAdmin}
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
          <td class="text-right mono text-dim">{pv ? formatQty(pv.book) : "—"}</td>
          <td>
            <input
              class="cell-input mono"
              data-fisik-row={row.id}
              type="number" min="0" step="0.01"
              disabled={!row.product || !isAdmin}
              bind:value={row.fisik}
              onkeydown={(e) => onFisikKey(e, row)}
              onfocus={(e) => e.currentTarget.select()}
            />
          </td>
          <td
            class="text-right mono fw-bold"
            style="color: {!pv || pv.diff === 0 ? 'inherit' : pv.diff > 0 ? 'var(--success)' : 'var(--danger)'};"
          >
            {pv ? `${pv.diff >= 0 ? "+" : ""}${formatQty(pv.diff)}` : "—"}
          </td>
          <td class="text-right mono">
            {#if pv}
              <span class="text-dim">{formatQty(pv.stock_now_before)}</span>
              <span class="text-dim">→</span>
              <b>{formatQty(pv.stock_now_after)}</b>
              {#if pv.overridden_by_later_opname}
                <span title="Barang ini sudah di-opname lagi setelah waktu tersebut, jadi stok sekarang mengikuti opname yang lebih baru.">⚠️</span>
              {/if}
            {:else}
              —
            {/if}
          </td>
          <td>
            <input class="cell-input" disabled={!row.product || !isAdmin} bind:value={row.keterangan} placeholder="opsional" />
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
  <button onclick={addRow} disabled={!isAdmin}>➕ Tambah Baris</button>
  <button onclick={hitungPratinjau} disabled={!isAdmin || previewing || !filledRows.length}>
    {previewing ? "Menghitung…" : "🔄 Hitung Pratinjau"}
  </button>
  <button class="btn-primary" disabled={!bisaSimpan} onclick={mintaKonfirmasi}>💾 Simpan Time Opname</button>
  <span class="text-dim" style="margin-left:auto; font-size:0.82rem;">
    {filledRows.length} barang · dicatat {waktuLabel}
  </span>
</div>
<ShortcutBar items={[
  { key: "F9", label: "Simpan Time Opname", action: mintaKonfirmasi, disabled: !bisaSimpan },
  { key: "F6", label: "Tambah Baris", action: addRow, disabled: !isAdmin },
]} />

{#if showConfirm}
  <div class="modal-backdrop" onclick={() => (showConfirm = false)} role="presentation">
    <div class="modal confirm-modal" onclick={(e) => e.stopPropagation()} role="presentation">
      <h2 style="margin-top:0;">Simpan opname per {waktuLabel}?</h2>
      <p style="margin:0.2rem 0 0.6rem; font-size:0.9rem;">
        <b>{previewRows.length}</b> barang akan dicatat sesuai hitungan fisik pada waktu itu, lalu
        semua mutasi sesudahnya dihitung ulang. <b>Tidak bisa dibatalkan.</b>
      </p>
      <div class="pv-list">
        {#each previewRows as r (r.product_id)}
          <div class="pv-row">
            <span class="pv-name">
              {r.product_name}
              {#if r.overridden_by_later_opname}
                <span title="Sudah ada opname lebih baru — stok sekarang tidak tergeser.">⚠️</span>
              {/if}
            </span>
            <span class="mono text-dim">buku {formatQty(r.book)} → fisik {formatQty(r.counted)}</span>
            <span class="mono">stok {formatQty(r.stock_now_before)} → <b>{formatQty(r.stock_now_after)}</b></span>
          </div>
        {/each}
      </div>
      <div class="row" style="justify-content:flex-end; gap:0.5rem; margin-top:0.9rem;">
        <button onclick={() => (showConfirm = false)}>Batal</button>
        <button class="btn-primary" disabled={busy} onclick={simpan}>
          {busy ? "Menyimpan…" : "Ya, Simpan"}
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
  .meta-val.bad { color: var(--danger); }

  .info-block {
    border: 1px solid var(--border); border-radius: 8px; background: var(--baby-blue-bg);
    padding: 0.55rem 0.75rem; margin-bottom: 0.8rem; font-size: 0.85rem;
  }
  .warn-block {
    border: 1px solid var(--danger); border-radius: 8px;
    background: color-mix(in srgb, var(--danger) 8%, transparent);
    padding: 0.55rem 0.75rem; margin-bottom: 0.8rem; font-size: 0.85rem;
  }

  .batch-table { width: 100%; border-collapse: collapse; }
  .batch-table thead th {
    background: var(--baby-blue-bg); padding: 0.5rem 0.6rem;
    font-size: 0.74rem; font-weight: 650; color: var(--text-dim);
    text-transform: uppercase; letter-spacing: 0.04em;
    border-bottom: 1px solid var(--border); position: sticky; top: 0;
  }
  .batch-table tbody td { padding: 0.3rem 0.5rem; border-bottom: 1px solid var(--border); vertical-align: middle; font-size: 0.85rem; }
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

  .import-block {
    border: 1px dashed var(--border-strong); border-radius: 8px;
    padding: 0.6rem 0.7rem; margin-bottom: 0.8rem; background: var(--baby-blue-bg);
  }
  .import-log-head {
    display: flex; align-items: center; gap: 0.5rem;
    font-weight: 650; font-size: 0.85rem; margin-bottom: 0.3rem;
  }
  .log-err-badge {
    font-size: 0.7rem; font-weight: 700; padding: 0.1rem 0.4rem;
    border-radius: 999px; background: var(--danger); color: #fff;
  }
  .import-log {
    max-height: 180px; overflow-y: auto;
    border: 1px solid var(--border); border-radius: 8px; background: var(--white);
  }
  .log-row {
    display: grid; grid-template-columns: 3.5rem 1fr 2fr; gap: 0.5rem;
    padding: 0.3rem 0.6rem; font-size: 0.8rem; border-bottom: 1px solid var(--border);
    background: color-mix(in srgb, var(--danger) 10%, transparent);
  }
  .log-row:last-child { border-bottom: none; }

  .confirm-modal { max-width: 620px; width: 92vw; padding: 1rem 1.1rem; }
  .pv-list { max-height: 260px; overflow-y: auto; border: 1px solid var(--border); border-radius: 8px; }
  .pv-row {
    display: grid; grid-template-columns: 1fr auto auto; gap: 0.8rem; align-items: center;
    padding: 0.3rem 0.6rem; font-size: 0.82rem; border-bottom: 1px solid var(--border);
  }
  .pv-row:last-child { border-bottom: none; }
  .pv-name { overflow-wrap: anywhere; }
</style>
