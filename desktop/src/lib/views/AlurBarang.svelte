<script lang="ts">
  import { tick } from "svelte";
  import { api } from "$lib/api";
  import { formatQty, formatDateTime } from "$lib/format";
  import { toastError } from "$lib/toast";
  import { printElement } from "$lib/print";
  import { debounce } from "$lib/debounce";
  import { formatPeriodLabel, todayIso } from "$lib/dateTime";
  import ProductSearchPopup from "$lib/components/ProductSearchPopup.svelte";
  import type { ProductWithStock, StockFlowDetail, StockFlowRow, StockKind } from "$lib/types";

  type Mode = "item" | "all";

  let mode = $state<Mode>("item");
  let from = $state("");
  let to = $state("");

  // Mode "Per Barang"
  let search = $state("");
  let showPopup = $state(false);
  let selectedId = $state<string | null>(null);
  let detail = $state<StockFlowDetail | null>(null);

  // Mode "Semua Barang"
  let recap = $state<StockFlowRow[]>([]);
  let searchAll = $state("");
  let searchApplied = $state("");
  const applySearch = debounce((v: string) => (searchApplied = v), 300);

  const PAGE_SIZE = 100;
  let page = $state(0);
  /** Saat true, seluruh baris dirender (dipakai sesaat sebelum klik Print). */
  let renderAll = $state(false);

  const dateStr = (iso: string) => {
    const d = new Date(iso);
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  };
  function presetHariIni() {
    from = todayIso();
    to = todayIso();
  }
  function presetBulanIni() {
    const d = new Date();
    from = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-01`;
    to = dateStr(d.toISOString());
  }
  function presetTahunIni() {
    const d = new Date();
    from = `${d.getFullYear()}-01-01`;
    to = dateStr(d.toISOString());
  }
  function resetRange() {
    from = "";
    to = "";
  }
  presetBulanIni();

  const rangeFrom = $derived(from || "0001-01-01");
  const rangeTo = $derived(to || "9999-12-31");
  const periodLabel = $derived(formatPeriodLabel(from, to));

  // ---------- Mode Per Barang ----------

  async function loadDetail(productId: string, f: string, t: string) {
    try {
      detail = await api.stockFlowDetail(productId, f, t);
    } catch (e) {
      toastError(e);
    }
  }

  $effect(() => {
    const id = selectedId;
    if (!id) {
      detail = null;
      return;
    }
    loadDetail(id, rangeFrom, rangeTo);
  });

  async function onSearchKey(e: KeyboardEvent) {
    if (e.key !== "Enter") return;
    const term = search.trim();
    if (!term) return;
    // Barcode dulu (hasil scan), baru jatuh ke popup pencarian nama.
    try {
      const p = await api.findByBarcode(term);
      if (p) {
        pick(p);
        return;
      }
    } catch (_) { /* lanjut ke popup */ }
    showPopup = true;
  }

  function pick(p: ProductWithStock) {
    selectedId = p.id;
    search = "";
  }

  function openFromRecap(r: StockFlowRow) {
    selectedId = r.product_id;
    mode = "item";
  }

  const KIND_LABEL: Record<StockKind, string> = {
    in: "Masuk",
    out: "Keluar",
    sale: "Terjual",
    opname: "Opname",
  };

  /**
   * Buku besar: tiap mutasi dilengkapi delta terhadap stok sebelumnya. Untuk
   * opname delta tidak bisa dibaca dari `qty` (opname menyetel stok absolut),
   * jadi dihitung dari selisih stok sebelum vs sesudah.
   */
  const ledger = $derived.by(() => {
    if (!detail) return [];
    let prev = detail.opening;
    return detail.rows.map((mv) => {
      const delta =
        mv.kind === "in" ? mv.qty : mv.kind === "opname" ? mv.stock_after - prev : -mv.qty;
      prev = mv.stock_after;
      return { mv, delta };
    });
  });

  const sum = (kind: StockKind) =>
    (detail?.rows ?? []).filter((m) => m.kind === kind).reduce((s, m) => s + m.qty, 0);
  const totalMasuk = $derived(sum("in"));
  const totalKeluar = $derived(sum("out"));
  const totalTerjual = $derived(sum("sale"));
  const closing = $derived(
    detail ? (detail.rows.length ? detail.rows[detail.rows.length - 1].stock_after : detail.opening) : 0,
  );
  const adjustment = $derived(
    detail ? closing - (detail.opening + totalMasuk - totalKeluar - totalTerjual) : 0,
  );

  // ---------- Mode Semua Barang ----------

  async function loadRecap(f: string, t: string, q: string) {
    try {
      recap = await api.stockFlowRecap(f, t, q.trim() || null);
      page = 0;
    } catch (e) {
      toastError(e);
    }
  }

  $effect(() => {
    if (mode !== "all") return;
    loadRecap(rangeFrom, rangeTo, searchApplied);
  });

  const totalPages = $derived(Math.max(1, Math.ceil(recap.length / PAGE_SIZE)));
  const recapShown = $derived(
    renderAll ? recap : recap.slice(page * PAGE_SIZE, page * PAGE_SIZE + PAGE_SIZE),
  );
  function goPage(delta: number) {
    const next = page + delta;
    if (next < 0 || next >= totalPages) return;
    page = next;
  }

  const recapTotal = $derived.by(() =>
    recap.reduce(
      (acc, r) => ({
        opening: acc.opening + r.opening,
        masuk: acc.masuk + r.masuk,
        keluar: acc.keluar + r.keluar,
        terjual: acc.terjual + r.terjual,
        adjustment: acc.adjustment + r.adjustment,
        closing: acc.closing + r.closing,
      }),
      { opening: 0, masuk: 0, keluar: 0, terjual: 0, adjustment: 0, closing: 0 },
    ),
  );

  /** Cetak seluruh baris (bukan cuma halaman aktif) sebagai laporan A4. */
  async function print() {
    renderAll = true;
    await tick();
    printElement("printable-page", "Alur Barang");
    renderAll = false;
  }
</script>

<div class="view-flex">
  <div id="printable-page" class="view-flex">
    <div class="page-head">
      <div>
        <h1>Alur Barang</h1>
        <div class="text-dim print-period" style="font-size:0.8rem;">{periodLabel}</div>
      </div>
      <div class="row no-print">
        <button class:btn-primary={mode === "item"} onclick={() => (mode = "item")}>Per Barang</button>
        <button class:btn-primary={mode === "all"} onclick={() => (mode = "all")}>Semua Barang</button>
        <button onclick={print}>🖨️ Print</button>
      </div>
    </div>

    <!-- Filter + kotak scan digabung satu baris supaya tabel di bawah dapat ruang maksimal. -->
    <div class="card filter-card no-print">
      <div class="filter-row">
        <div class="f-field"><label>Dari</label><input type="date" bind:value={from} /></div>
        <div class="f-field"><label>Sampai</label><input type="date" bind:value={to} /></div>
        <div class="row presets">
          <button onclick={presetHariIni}>Hari Ini</button>
          <button onclick={presetBulanIni}>Bulan Ini</button>
          <button onclick={presetTahunIni}>Tahun Ini</button>
          <button onclick={resetRange}>Semua</button>
        </div>
        {#if mode === "item"}
          <div class="f-field grow">
            <label>Kode Item / Nama Barang</label>
            <input
              placeholder="Scan barcode atau ketik nama lalu Enter…"
              bind:value={search}
              onkeydown={onSearchKey}
              autocomplete="off"
            />
          </div>
        {:else}
          <div class="f-field grow">
            <label>Cari Barang</label>
            <input
              placeholder="Nama atau barcode…"
              bind:value={searchAll}
              oninput={() => applySearch(searchAll)}
              autocomplete="off"
            />
          </div>
        {/if}
      </div>
    </div>

    {#if mode === "item"}
      {#if detail}
        <div class="card info-card">
          <div class="info-line">
            <span class="prod-name">{detail.name}</span>
            <span class="chip">Barcode <b class="mono">{detail.barcode || "—"}</b></span>
            <span class="chip">Merek <b>{detail.brand || "—"}</b></span>
            <span class="chip">Satuan <b>{detail.unit || "—"}</b></span>
            <span class="chip">Stok Sekarang <b class="mono">{formatQty(detail.current_stock)}</b></span>
          </div>

          <div class="flow-strip">
            <div class="flow-box"><span class="fb-lbl">Stok Awal</span><span class="fb-val mono">{formatQty(detail.opening)}</span></div>
            <div class="flow-box plus"><span class="fb-lbl">Masuk</span><span class="fb-val mono">+{formatQty(totalMasuk)}</span></div>
            <div class="flow-box minus"><span class="fb-lbl">Keluar</span><span class="fb-val mono">−{formatQty(totalKeluar)}</span></div>
            <div class="flow-box minus"><span class="fb-lbl">Terjual</span><span class="fb-val mono">−{formatQty(totalTerjual)}</span></div>
            <div class="flow-box {adjustment >= 0 ? 'plus' : 'minus'}">
              <span class="fb-lbl">Penyesuaian</span>
              <span class="fb-val mono">{adjustment >= 0 ? "+" : "−"}{formatQty(Math.abs(adjustment))}</span>
            </div>
            <div class="flow-box total"><span class="fb-lbl">Stok Akhir</span><span class="fb-val mono">{formatQty(closing)}</span></div>
          </div>
        </div>

        <div class="card list-card">
          <div class="list-scroll">
            <table>
              <thead>
                <tr>
                  <th>Waktu</th><th>Jenis</th><th class="text-right">Perubahan</th>
                  <th class="text-right">Stok Setelah</th><th>User</th><th>Keterangan</th>
                </tr>
              </thead>
              <tbody>
                {#each ledger as { mv, delta } (mv.id)}
                  <tr>
                    <td style="white-space:nowrap;">{formatDateTime(mv.created_at)}</td>
                    <td><span class="badge k-{mv.kind}">{KIND_LABEL[mv.kind]}</span></td>
                    <td class="text-right mono {delta >= 0 ? 'up' : 'down'}">
                      {delta >= 0 ? "+" : "−"}{formatQty(Math.abs(delta))}
                    </td>
                    <td class="text-right mono">{formatQty(mv.stock_after)}</td>
                    <td>{mv.user_id ?? "—"}</td>
                    <td class="text-dim">{mv.note ?? "—"}</td>
                  </tr>
                {:else}
                  <tr><td colspan="6" class="text-dim">Tidak ada pergerakan pada periode ini.</td></tr>
                {/each}
              </tbody>
            </table>
          </div>
          <div class="pager">
            <span class="text-dim" style="font-size:0.82rem;">
              {detail.rows.length.toLocaleString("id-ID")} pergerakan pada periode ini
            </span>
          </div>
        </div>
      {:else}
        <div class="card empty-card text-dim">
          Scan barcode atau cari nama barang di atas untuk melihat alur stoknya.
        </div>
      {/if}
    {:else}
      <div class="card list-card">
        <div class="list-scroll">
          <table>
            <thead>
              <tr>
                <th>Barang</th><th>Barcode</th><th>Merek</th>
                <th class="text-right">Stok Awal</th><th class="text-right">Masuk</th>
                <th class="text-right">Keluar</th><th class="text-right">Terjual</th>
                <th class="text-right">Penyesuaian</th><th class="text-right">Stok Akhir</th>
              </tr>
            </thead>
            <tbody>
              {#each recapShown as r (r.product_id)}
                <tr class="clickable" onclick={() => openFromRecap(r)} title="Lihat alur barang ini">
                  <td>{r.name}</td>
                  <td class="mono text-dim">{r.barcode || "—"}</td>
                  <td class="text-dim">{r.brand || "—"}</td>
                  <td class="text-right mono">{formatQty(r.opening)}</td>
                  <td class="text-right mono">{formatQty(r.masuk)}</td>
                  <td class="text-right mono">{formatQty(r.keluar)}</td>
                  <td class="text-right mono">{formatQty(r.terjual)}</td>
                  <td class="text-right mono">{r.adjustment ? `${r.adjustment > 0 ? "+" : "−"}${formatQty(Math.abs(r.adjustment))}` : "—"}</td>
                  <td class="text-right mono fw-bold">{formatQty(r.closing)}</td>
                </tr>
              {:else}
                <tr><td colspan="9" class="text-dim">Tidak ada data pada periode ini.</td></tr>
              {/each}
            </tbody>
            {#if recap.length}
              <tfoot>
                <tr>
                  <td colspan="3" class="fw-bold">Total ({recap.length.toLocaleString("id-ID")} barang)</td>
                  <td class="text-right mono fw-bold">{formatQty(recapTotal.opening)}</td>
                  <td class="text-right mono fw-bold">{formatQty(recapTotal.masuk)}</td>
                  <td class="text-right mono fw-bold">{formatQty(recapTotal.keluar)}</td>
                  <td class="text-right mono fw-bold">{formatQty(recapTotal.terjual)}</td>
                  <td class="text-right mono fw-bold">{recapTotal.adjustment >= 0 ? "+" : "−"}{formatQty(Math.abs(recapTotal.adjustment))}</td>
                  <td class="text-right mono fw-bold">{formatQty(recapTotal.closing)}</td>
                </tr>
              </tfoot>
            {/if}
          </table>
        </div>
        <div class="pager no-print">
          <span class="text-dim" style="font-size:0.82rem;">
            {recap.length.toLocaleString("id-ID")} barang · Hal {page + 1} / {totalPages}
          </span>
          <div class="row" style="gap:0.3rem;">
            <button disabled={page === 0} onclick={() => goPage(-1)}>‹ Sebelumnya</button>
            <button disabled={page + 1 >= totalPages} onclick={() => goPage(1)}>Berikutnya ›</button>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>

{#if showPopup}
  <ProductSearchPopup
    initialQuery={search}
    onClose={() => (showPopup = false)}
    onPick={(p) => { pick(p); showPopup = false; }}
  />
{/if}

<style>
  .view-flex { height: 100%; min-height: 0; display: flex; flex-direction: column; }
  .list-card { padding: 0; overflow: hidden; flex: 1; min-height: 0; display: flex; flex-direction: column; }
  .list-scroll { flex: 1; min-height: 0; overflow: auto; }
  .pager {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.4rem 0.9rem; border-top: 1px solid var(--border); flex-shrink: 0;
  }
  .empty-card { display: flex; align-items: center; justify-content: center; min-height: 120px; font-size: 0.9rem; }

  /* Blok atas dibuat serapat mungkin — sisa tinggi dipakai tabel. */
  .page-head { margin-bottom: 0.5rem; }
  .page-head h1 { font-size: 1.3rem; }

  .filter-card { padding: 0.5rem 0.7rem; margin-bottom: 0.5rem; flex-shrink: 0; }
  .filter-row { display: flex; flex-wrap: wrap; align-items: flex-end; gap: 0.5rem 0.7rem; }
  .f-field { display: flex; flex-direction: column; gap: 0.15rem; }
  .f-field.grow { flex: 1 1 260px; min-width: 200px; }
  .f-field label { font-size: 0.74rem; color: var(--text-dim); margin: 0; }
  .f-field input { margin: 0; }
  .presets { gap: 0.3rem; }

  .info-card { padding: 0.5rem 0.7rem; margin-bottom: 0.5rem; flex-shrink: 0; display: flex; flex-direction: column; gap: 0.45rem; }
  .info-line { display: flex; flex-wrap: wrap; align-items: baseline; gap: 0.3rem 0.9rem; }
  .prod-name { font-size: 1rem; font-weight: 700; }
  .chip { font-size: 0.78rem; color: var(--text-dim); }
  .chip b { color: var(--text); font-weight: 600; }

  .flow-strip { display: flex; flex-wrap: wrap; gap: 0.4rem; }
  .flow-box {
    flex: 1 1 110px; display: flex; align-items: baseline; justify-content: space-between; gap: 0.5rem;
    border: 1px solid var(--border); border-radius: var(--radius); padding: 0.25rem 0.55rem;
    background: var(--baby-blue-bg);
  }
  .fb-lbl { font-size: 0.72rem; color: var(--text-dim); }
  .fb-val { font-size: 0.95rem; font-weight: 700; }
  .flow-box.plus .fb-val { color: var(--success); }
  .flow-box.minus .fb-val { color: var(--danger); }
  .flow-box.total { background: var(--baby-blue-soft); border-color: var(--border-strong); }

  /* Baris dirapatkan supaya lebih banyak pergerakan kelihatan tanpa scroll. */
  .list-scroll :global(th),
  .list-scroll :global(td) { padding: 0.32rem 0.7rem; }

  .badge.k-in { color: var(--success); border-color: var(--success); }
  .badge.k-out, .badge.k-sale { color: var(--danger); border-color: var(--danger); }
  .badge.k-opname { color: var(--primary-dark); border-color: var(--border-strong); }
  .up { color: var(--success); }
  .down { color: var(--danger); }
  .fw-bold { font-weight: 700; }
  .clickable { cursor: pointer; }
  tfoot td { border-top: 2px solid var(--border); padding: 0.5rem 0.6rem; }

  /* Periode hanya relevan di kertas; di layar sudah ada filter tanggalnya. */
  .print-period { display: none; }
  :global(#print-root) .print-period { display: block !important; }
</style>
