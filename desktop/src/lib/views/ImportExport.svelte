<script lang="ts">
  import * as XLSX from "xlsx";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";

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
  ];
  const EXAMPLE = [
    ["Indomie Goreng", "8992388101010", "Mie Instan", "Indofood", "pcs", 2800, 3500, 0, "ya"],
    ["Aqua 600ml", "8993675001020", "Minuman", "Aqua", "botol", 3000, 4000, 0, "ya"],
  ];

  let preview = $state<Record<string, unknown>[]>([]);
  let importing = $state(false);

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
    let ok = 0;
    let fail = 0;
    for (const row of preview) {
      const r = row as Record<string, unknown>;
      const name = String(r.nama ?? "").trim();
      if (!name) {
        fail++;
        continue;
      }
      try {
        await api.saveProduct({
          id: null,
          name,
          barcode: String(r.barcode ?? "") || null,
          category: String(r.kategori ?? "") || null,
          brand: String(r.merek ?? "") || null,
          unit: String(r.satuan ?? "") || null,
          cost_price: Number(r.harga_pokok) || 0,
          sell_price: Number(r.harga_jual) || 0,
          default_discount: Number(r.default_diskon) || 0,
          is_active: parseAktif(r.aktif),
        });
        ok++;
      } catch {
        fail++;
      }
    }
    importing = false;
    preview = [];
    showToast(`Import selesai: ${ok} berhasil, ${fail} gagal.`, ok ? "success" : "error", 5000);
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
      1) Buka template terkunci di bawah → <b>Save As</b> ke file kamu → isi data →
      2) pilih file tersebut untuk meng-import banyak barang sekaligus.
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
