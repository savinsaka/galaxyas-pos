<script lang="ts">
  import * as XLSX from "xlsx";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import type { DedupeResult } from "$lib/types";

  const HEADERS = [
    "nama",
    "barcode",
    "kategori",
    "merek",
    "satuan",
    "harga_pokok",
    "harga_jual",
    "default_diskon",
    "aktif",
    "stok",
  ];
  const EXAMPLE = [
    ["Indomie Goreng", "8992388101010", "Mie Instan", "Indofood", "pcs", 2800, 3500, 0, "ya", 50],
    ["Aqua 600ml", "8993675001020", "Minuman", "Aqua", "botol", 3000, 4000, 0, "ya", 24],
  ];

  interface ImportLogEntry {
    row: number;
    name: string;
    status: "created" | "duplicate" | "error";
    reason?: string;
  }

  let preview = $state<Record<string, unknown>[]>([]);
  let importing = $state(false);
  let importProgress = $state({ done: 0, total: 0 });
  let importLog = $state<ImportLogEntry[]>([]);
  let deduping = $state(false);
  let dedupeResult = $state<DedupeResult | null>(null);

  function parseAktif(v: unknown): boolean {
    const s = String(v ?? "").toLowerCase().trim();
    if (s === "") return true;
    return ["1", "ya", "aktif", "true", "y"].includes(s);
  }

  async function onFile(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    try {
      const buf = await file.arrayBuffer();
      const wb = XLSX.read(buf, { type: "array" });
      const ws = wb.Sheets[wb.SheetNames[0]];
      preview = XLSX.utils.sheet_to_json(ws, { defval: "" });
      showToast(`${preview.length} baris terbaca. Periksa lalu klik Import.`, "info");
    } catch (e) {
      toastError(e);
    }
  }

  async function doImport() {
    if (!preview.length) return;
    importing = true;
    importLog = [];
    importProgress = { done: 0, total: preview.length };

    // Peta barcode -> nama barang existing (termasuk non-aktif), untuk deteksi
    // duplikasi otomatis. Barcode yang sudah terdaftar (di DB atau baris lain
    // di file ini) ditolak, bukan ditimpa — import ini hanya untuk barang baru.
    const existingByBarcode = new Map<string, string>(); // barcode -> nama
    try {
      const existing = await api.listProducts("", true);
      for (const p of existing) {
        if (p.barcode && p.barcode.trim()) existingByBarcode.set(p.barcode.trim().toLowerCase(), p.name);
      }
    } catch (e) {
      toastError(e);
    }

    let created = 0;
    let duplicate = 0;
    let failed = 0;

    for (let i = 0; i < preview.length; i++) {
      const row = preview[i];
      const r = row as Record<string, unknown>;
      const name = String(r.nama ?? "").trim();
      const barcode = String(r.barcode ?? "").trim();
      importProgress = { done: i, total: preview.length };

      if (!name) {
        failed++;
        importLog.push({ row: i + 2, name: name || "(tanpa nama)", status: "error", reason: "Nama kosong" });
        continue;
      }
      const existingName = barcode ? existingByBarcode.get(barcode.toLowerCase()) : undefined;
      if (existingName) {
        duplicate++;
        importLog.push({
          row: i + 2,
          name,
          status: "duplicate",
          reason: `Barcode "${barcode}" sudah terdaftar untuk "${existingName}" — dilewati`,
        });
        continue;
      }

      try {
        const saved = await api.saveProduct({
          id: null,
          name,
          barcode: barcode || null,
          category: String(r.kategori ?? "") || null,
          brand: String(r.merek ?? "") || null,
          unit: String(r.satuan ?? "") || null,
          cost_price: Number(r.harga_pokok) || 0,
          sell_price: Number(r.harga_jual) || 0,
          default_discount: Number(r.default_diskon) || 0,
          is_active: parseAktif(r.aktif),
        });
        // Set stok awal jika kolom stok diisi
        const stokVal = Number(r.stok);
        if (stokVal > 0) {
          await api.setStock(saved.id, stokVal);
        }
        if (barcode) existingByBarcode.set(barcode.toLowerCase(), name);
        created++;
        importLog.push({ row: i + 2, name, status: "created" });
      } catch (e) {
        failed++;
        importLog.push({ row: i + 2, name, status: "error", reason: e instanceof Error ? e.message : String(e) });
      }
    }

    importProgress = { done: preview.length, total: preview.length };
    importing = false;
    preview = [];
    const parts = [`${created} baru`];
    if (duplicate) parts.push(`${duplicate} ditolak (duplikasi barcode)`);
    if (failed) parts.push(`${failed} gagal`);
    showToast(`Import selesai: ${parts.join(", ")}.`, duplicate || failed ? "error" : "success", 6000);
  }

  async function runDedupe() {
    if (!confirm("Cari barang dengan barcode kembar dan non-aktifkan yang lama (data terbaru dipertahankan)?")) return;
    deduping = true;
    try {
      dedupeResult = await api.dedupeProducts();
      showToast(
        dedupeResult.groups
          ? `${dedupeResult.groups} barcode kembar ditemukan, ${dedupeResult.removed} barang lama dinon-aktifkan.`
          : "Tidak ada barcode kembar ditemukan.",
        "success",
      );
    } catch (e) {
      toastError(e);
    } finally {
      deduping = false;
    }
  }

  async function doExport() {
    try {
      const products = await api.listProducts("", true);
      const data = products.map((p) => ({
        nama: p.name,
        barcode: p.barcode ?? "",
        kategori: p.category ?? "",
        merek: p.brand ?? "",
        satuan: p.unit ?? "",
        harga_pokok: p.cost_price,
        harga_jual: p.sell_price,
        default_diskon: p.default_discount,
        aktif: p.is_active ? "ya" : "tidak",
        stok: p.stock_qty,
      }));
      const ws = XLSX.utils.json_to_sheet(data);
      const wb = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(wb, ws, "Barang");
      XLSX.writeFile(wb, "data-barang.xlsx");
      showToast("Data diekspor ke data-barang.xlsx", "success");
    } catch (e) {
      toastError(e);
    }
  }

  async function openTemplate() {
    try {
      const ws = XLSX.utils.aoa_to_sheet([HEADERS, ...EXAMPLE]);
      // Proteksi sheet (best-effort) + file di-set read-only oleh backend.
      (ws as Record<string, unknown>)["!protect"] = {
        selectLockedCells: true,
        selectUnlockedCells: true,
      };
      const wb = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(wb, ws, "Template");
      const out = XLSX.write(wb, { type: "array", bookType: "xlsx" }) as ArrayBuffer;
      const bytes = Array.from(new Uint8Array(out));
      const path = await api.writeTempFile("Template-Batch-Barang.xlsx", bytes);
      await openPath(path);
      showToast("Template dibuka di Excel (read-only). Pilih Save As untuk mengisi & menyimpan.", "info", 7000);
    } catch (e) {
      toastError(e);
    }
  }

  async function copyTemplate() {
    const tsv = [HEADERS.join("\t"), ...EXAMPLE.map((r) => r.join("\t"))].join("\n");
    try {
      await navigator.clipboard.writeText(tsv);
      showToast("Template disalin ke clipboard. Tempel ke Excel.", "success");
    } catch {
      showToast("Gagal menyalin. Salin manual dari tabel di bawah.", "error");
    }
  }
</script>

<div class="page-head"><h1>Import / Export Barang</h1></div>

<div class="grid-2" style="align-items:start;">
  <div class="card">
    <h2>📥 Batch Tambah Barang (Import Excel)</h2>
    <p class="text-dim">
      1) Buka template → <b>Save As</b> ke file kamu → isi data →
      2) pilih file untuk meng-import banyak barang sekaligus.<br/>
      Kolom <b>stok</b> otomatis mengisi stok awal jika diisi dengan angka &gt; 0.
    </p>
    <div style="background:var(--baby-blue-bg); border:1px dashed var(--border-strong); border-radius:8px; padding:0.8rem; margin-bottom:0.8rem;">
      <button class="btn-primary" onclick={openTemplate}>📄 Buka Template Excel (terkunci)</button>
      <div class="text-dim" style="font-size:0.76rem; margin-top:0.4rem;">
        Template dibuka mode <b>read-only</b> agar formatnya tidak rusak. Gunakan <b>Save As</b> untuk mengisinya.
      </div>
    </div>
    <label>Pilih file hasil isian</label>
    <input type="file" accept=".xlsx,.xls,.csv" onchange={onFile} />
    {#if preview.length}
      <p style="margin-top:0.8rem;">{preview.length} baris siap di-import.</p>
      <div style="max-height:240px; overflow:auto; border:1px solid var(--border); border-radius:8px;">
        <table>
          <thead><tr>{#each HEADERS as h}<th>{h}</th>{/each}</tr></thead>
          <tbody>
            {#each preview.slice(0, 50) as row}
              <tr>{#each HEADERS as h}<td>{(row as Record<string, unknown>)[h] ?? ""}</td>{/each}</tr>
            {/each}
          </tbody>
        </table>
      </div>
      <button class="btn-primary" style="margin-top:0.7rem;" disabled={importing} onclick={doImport}>
        {importing ? "Mengimport…" : `Import ${preview.length} Barang`}
      </button>
    {/if}

    {#if importing || importLog.length}
      <div style="margin-top:0.9rem;">
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
          {@const errCount = importLog.filter((l) => l.status === "error" || l.status === "duplicate").length}
          <div class="import-log-head">
            <span>📋 Log Import ({importLog.length} baris)</span>
            {#if errCount}<span class="log-err-badge">{errCount} bermasalah</span>{/if}
          </div>
          <div class="import-log">
            {#each importLog.filter((l) => l.status === "error" || l.status === "duplicate") as l}
              <div class="log-row log-{l.status}">
                <span class="mono">#{l.row}</span>
                <span>{l.name}</span>
                <span class="text-dim">{l.reason}</span>
              </div>
            {:else}
              <div class="log-row text-dim">Tidak ada baris bermasalah.</div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <div class="card">
    <h2>🧹 Bersihkan Barcode Duplikat</h2>
    <p class="text-dim">
      Non-aktifkan otomatis barang dengan barcode yang sama (menyisakan data terbaru), sehingga tidak
      muncul dobel di kasir. Riwayat transaksi lama tidak terpengaruh.
    </p>
    <button disabled={deduping} onclick={runDedupe}>{deduping ? "Memeriksa…" : "🔍 Cari & Bersihkan Duplikat"}</button>
    {#if dedupeResult}
      <p style="margin-top:0.6rem; font-size:0.85rem;">
        {dedupeResult.groups} grup barcode kembar, {dedupeResult.removed} barang dinon-aktifkan.
      </p>
      {#if dedupeResult.details.length}
        <div style="max-height:160px; overflow:auto; border:1px solid var(--border); border-radius:8px; margin-top:0.4rem;">
          <table>
            <thead><tr><th>Barcode</th><th>Dipertahankan</th><th class="text-right">Dinon-aktifkan</th></tr></thead>
            <tbody>
              {#each dedupeResult.details as d}
                <tr><td class="mono">{d.barcode}</td><td>{d.kept_name}</td><td class="text-right">{d.removed_count}</td></tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    {/if}
  </div>

  <div class="card">
    <h2>📤 Export</h2>
    <p class="text-dim">Unduh seluruh data barang (termasuk stok) ke file Excel.</p>
    <button class="btn-primary" onclick={doExport}>Export ke Excel</button>

    <h2 style="margin-top:1.5rem;">📋 Template</h2>
    <p class="text-dim">
      Template ini <b>tidak untuk disimpan</b> — cukup <b>salin</b> lalu tempel ke Excel kamu,
      agar tidak salah format saat menyimpan.
    </p>
    <button onclick={copyTemplate}>Salin Template ke Clipboard</button>
    <div style="margin-top:0.7rem; overflow:auto; border:1px solid var(--border); border-radius:8px;">
      <table>
        <thead><tr>{#each HEADERS as h}<th>{h}</th>{/each}</tr></thead>
        <tbody>
          {#each EXAMPLE as row}
            <tr>{#each row as cell}<td class="mono">{cell}</td>{/each}</tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</div>

<style>
  .import-progress-bar {
    height: 8px;
    border-radius: 999px;
    background: var(--baby-blue-bg);
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
  .log-duplicate { background: color-mix(in srgb, var(--warning) 14%, transparent); }
</style>
