<script lang="ts">
  import { api } from "$lib/api";
  import { formatIDR } from "$lib/format";
  import { toastError } from "$lib/toast";
  import { onMount } from "svelte";
  import BrandMultiSelect from "$lib/components/BrandMultiSelect.svelte";
  import SummaryTable from "$lib/components/SummaryTable.svelte";
  import { totalsByMethod } from "$lib/payment";
  import { REPORT_TYPES, defaultConfig, loadReportDesign, blockOrder, blockHidden, type ReportDesignConfig } from "$lib/reportDesign";
  import type { Brand, BrandSalesRow, Expense, ProductSalesRow, StockMovement, Transaction } from "$lib/types";

  let { from, to }: { from: string; to: string } = $props();

  const BLOCKS = REPORT_TYPES.find((t) => t.key === "sales-recap")!.blocks;
  let design = $state<ReportDesignConfig>(defaultConfig(BLOCKS));
  onMount(() => {
    loadReportDesign("sales-recap", BLOCKS).then((d) => (design = d));
  });

  type Gran = "harian" | "bulanan" | "tahunan";
  let gran = $state<Gran>("harian");
  let txs = $state<Transaction[]>([]);
  let sales = $state<StockMovement[]>([]);
  let expenses = $state<Expense[]>([]);
  let allBrands = $state<Brand[]>([]);
  let selectedBrands = $state<Set<string>>(new Set());
  let productReport = $state<ProductSalesRow[]>([]);
  let brandReport = $state<BrandSalesRow[]>([]);
  let allProductReport = $state<ProductSalesRow[]>([]);

  async function load() {
    const f = from || "0001-01-01";
    const t = to || "9999-12-31";
    const brandFilter = [...selectedBrands];
    try {
      [txs, sales, allBrands, productReport, brandReport, allProductReport, expenses] = await Promise.all([
        api.listTransactions(from || null, to || null, 5000),
        api.listStockMovements("sale", from || null, to || null, 5000),
        api.listBrands(),
        api.productSalesReport(f, t, brandFilter),
        api.brandSalesReport(f, t, brandFilter),
        api.productSalesReport(f, t, []),
        api.listExpenses(from || null, to || null),
      ]);
    } catch (e) {
      toastError(e);
    }
  }

  $effect(() => {
    // Muat ulang setiap kali rentang tanggal atau filter merek berubah.
    from;
    to;
    selectedBrands;
    load();
  });

  function bucketKey(iso: string): string {
    const d = new Date(iso);
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    if (gran === "harian") return `${y}-${m}-${day}`;
    if (gran === "bulanan") return `${y}-${m}`;
    return `${y}`;
  }

  const buckets = $derived.by(() => {
    const map = new Map<string, { count: number; total: number; discount: number }>();
    for (const t of txs) {
      const k = bucketKey(t.created_at);
      const b = map.get(k) ?? { count: 0, total: 0, discount: 0 };
      b.count += 1;
      b.total += t.total;
      b.discount += t.discount;
      map.set(k, b);
    }
    return [...map.entries()].sort((a, b) => (a[0] < b[0] ? 1 : -1));
  });

  // Pembayaran "Kombinasi" dipecah ke Tunai & QRIS sesuai rincian bayarnya —
  // di laporan yang dihitung uangnya, bukan status pembayarannya.
  const byMethod = $derived([...totalsByMethod(txs).entries()].sort((a, b) => b[1].total - a[1].total));

  const byCashier = $derived.by(() => {
    const map = new Map<string, { count: number; total: number }>();
    for (const t of txs) {
      const b = map.get(t.cashier_id) ?? { count: 0, total: 0 };
      b.count += 1;
      b.total += t.total;
      map.set(t.cashier_id, b);
    }
    return [...map.entries()].sort((a, b) => b[1].total - a[1].total);
  });

  const topItems = $derived.by(() => {
    const map = new Map<string, number>();
    for (const s of sales) map.set(s.product_name, (map.get(s.product_name) ?? 0) + s.qty);
    return [...map.entries()].sort((a, b) => b[1] - a[1]).slice(0, 10);
  });

  const grandTotal = $derived(txs.reduce((s, t) => s + t.total, 0));
  const grandDiscount = $derived(txs.reduce((s, t) => s + t.discount, 0));

  // ── Laba/Rugi: revenue - HPP (harga pokok) - pengeluaran operasional ──
  const totalCogs = $derived(allProductReport.reduce((s, r) => s + r.cogs, 0));
  const totalRevenue = $derived(allProductReport.reduce((s, r) => s + r.net, 0));
  const grossProfit = $derived(totalRevenue - totalCogs);
  const totalExpenses = $derived(expenses.reduce((s, e) => s + e.amount, 0));
  const netProfit = $derived(grossProfit - totalExpenses);
</script>

<div class="row no-print" style="justify-content:flex-end; margin-bottom:0.6rem;">
  {#each ["harian", "bulanan", "tahunan"] as g}
    <button class:btn-primary={gran === g} onclick={() => (gran = g as Gran)}>{g}</button>
  {/each}
</div>

<div class="card no-print" style="margin-bottom:1rem;">
  <div class="text-dim" style="font-size:0.8rem; margin-bottom:0.4rem;">Filter Merek (gabungkan beberapa untuk laporan Per Barang)</div>
  <BrandMultiSelect {allBrands} bind:selected={selectedBrands} />
</div>

<div class="grid-2" style="align-items:start;">
  {#if !blockHidden(design, "ringkasan")}
    <div style="grid-column:1/-1; order:{blockOrder(design, 'ringkasan')};">
      <SummaryTable
        title="Ringkasan Penjualan"
        rows={[
          { label: "Transaksi", value: String(txs.length) },
          { label: "Total Penjualan", value: formatIDR(grandTotal) },
          { label: "Total Diskon", value: formatIDR(grandDiscount) },
          { label: "Rata-rata", value: formatIDR(txs.length ? grandTotal / txs.length : 0) },
        ]}
      />
    </div>
  {/if}

  {#if !blockHidden(design, "laba_rugi")}
    <div style="grid-column:1/-1; order:{blockOrder(design, 'laba_rugi')};">
      <SummaryTable
        title="Laba / Rugi"
        rows={[
          { label: "HPP (Modal Barang)", value: formatIDR(totalCogs) },
          { label: "Laba Kotor", value: formatIDR(grossProfit) },
          { label: "Pengeluaran Operasional", value: `−${formatIDR(totalExpenses)}` },
          { label: "Laba Bersih", value: formatIDR(netProfit), bold: true, danger: netProfit < 0 },
        ]}
      />
    </div>
  {/if}

  {#if !blockHidden(design, "per_barang")}
    <div class="card" style="padding:0; overflow:hidden; order:{blockOrder(design, 'per_barang')};">
      <div style="padding:0.7rem 0.9rem;">
        <b>Per Barang{selectedBrands.size ? ` — ${[...selectedBrands].join(", ")}` : ""}</b>
      </div>
      <table>
        <thead><tr><th>Barang</th><th>Merek</th><th class="text-right">Qty</th><th class="text-right">Diskon</th><th class="text-right">Total</th></tr></thead>
        <tbody>
          {#each productReport as r (r.product_id)}
            <tr><td>{r.name}</td><td class="text-dim">{r.brand ?? "-"}</td><td class="text-right mono">{r.qty}</td><td class="text-right mono">{formatIDR(r.discount)}</td><td class="text-right mono">{formatIDR(r.net)}</td></tr>
          {:else}<tr><td colspan="5" class="text-dim">Tidak ada data.</td></tr>{/each}
        </tbody>
      </table>
    </div>
  {/if}

  {#if !blockHidden(design, "per_merek")}
    <div class="card" style="padding:0; overflow:hidden; order:{blockOrder(design, 'per_merek')};">
      <div style="padding:0.7rem 0.9rem;"><b>Per Merek</b></div>
      <table>
        <thead><tr><th>Merek</th><th class="text-right">Qty</th><th class="text-right">Diskon</th><th class="text-right">Total</th></tr></thead>
        <tbody>
          {#each brandReport as r (r.brand)}
            <tr><td>{r.brand}</td><td class="text-right mono">{r.qty}</td><td class="text-right mono">{formatIDR(r.discount)}</td><td class="text-right mono">{formatIDR(r.net)}</td></tr>
          {:else}<tr><td colspan="4" class="text-dim">Tidak ada data.</td></tr>{/each}
        </tbody>
      </table>
    </div>
  {/if}

  {#if !blockHidden(design, "per_periode")}
    <div class="card" style="padding:0; overflow:hidden; order:{blockOrder(design, 'per_periode')};">
      <div style="padding:0.7rem 0.9rem;"><b>Per Periode ({gran})</b></div>
      <table>
        <thead><tr><th>Periode</th><th class="text-right">Trx</th><th class="text-right">Diskon</th><th class="text-right">Total</th></tr></thead>
        <tbody>
          {#each buckets as [k, b] (k)}
            <tr><td class="mono">{k}</td><td class="text-right mono">{b.count}</td><td class="text-right mono">{formatIDR(b.discount)}</td><td class="text-right mono">{formatIDR(b.total)}</td></tr>
          {:else}<tr><td colspan="4" class="text-dim">Tidak ada data.</td></tr>{/each}
        </tbody>
      </table>
    </div>
  {/if}

  {#if !blockHidden(design, "item_terlaris")}
    <div class="card" style="padding:0; overflow:hidden; order:{blockOrder(design, 'item_terlaris')};">
      <div style="padding:0.7rem 0.9rem;"><b>Item Terlaris (qty)</b></div>
      <table>
        <thead><tr><th>Barang</th><th class="text-right">Qty</th></tr></thead>
        <tbody>
          {#each topItems as [name, qty] (name)}
            <tr><td>{name}</td><td class="text-right mono">{qty}</td></tr>
          {:else}<tr><td colspan="2" class="text-dim">Tidak ada data.</td></tr>{/each}
        </tbody>
      </table>
    </div>
  {/if}

  {#if !blockHidden(design, "per_metode")}
    <div class="card" style="padding:0; overflow:hidden; order:{blockOrder(design, 'per_metode')};">
      <div style="padding:0.7rem 0.9rem;"><b>Per Metode Pembayaran</b></div>
      <table>
        <thead><tr><th>Metode</th><th class="text-right">Trx</th><th class="text-right">Total</th></tr></thead>
        <tbody>
          {#each byMethod as [m, b] (m)}
            <tr><td>{m}</td><td class="text-right mono">{b.count}</td><td class="text-right mono">{formatIDR(b.total)}</td></tr>
          {:else}<tr><td colspan="3" class="text-dim">Tidak ada data.</td></tr>{/each}
        </tbody>
      </table>
      <div class="text-dim" style="padding:0.5rem 0.9rem; font-size:0.78rem;">
        Pembayaran kombinasi dipecah ke Tunai &amp; QRIS, jadi 1 transaksi kombinasi terhitung di kedua baris.
      </div>
    </div>
  {/if}

  {#if !blockHidden(design, "per_kasir")}
    <div class="card" style="padding:0; overflow:hidden; order:{blockOrder(design, 'per_kasir')};">
      <div style="padding:0.7rem 0.9rem;"><b>Per Kasir</b></div>
      <table>
        <thead><tr><th>Kasir</th><th class="text-right">Trx</th><th class="text-right">Total</th></tr></thead>
        <tbody>
          {#each byCashier as [c, b] (c)}
            <tr><td>{c}</td><td class="text-right mono">{b.count}</td><td class="text-right mono">{formatIDR(b.total)}</td></tr>
          {:else}<tr><td colspan="3" class="text-dim">Tidak ada data.</td></tr>{/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>
