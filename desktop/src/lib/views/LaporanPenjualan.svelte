<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR } from "$lib/format";
  import { toastError } from "$lib/toast";
  import { printReport } from "$lib/print";
  import type { StockMovement, Transaction } from "$lib/types";

  type Gran = "harian" | "bulanan" | "tahunan";
  let gran = $state<Gran>("harian");
  let from = $state("");
  let to = $state("");
  let txs = $state<Transaction[]>([]);
  let sales = $state<StockMovement[]>([]);

  async function load() {
    try {
      [txs, sales] = await Promise.all([
        api.listTransactions(5000),
        api.listStockMovements("sale", null, null, 5000),
      ]);
    } catch (e) {
      toastError(e);
    }
  }
  onMount(() => {
    presetBulanIni();
    load();
  });

  const dateStr = (iso: string) => {
    const d = new Date(iso);
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  };
  const inRange = (iso: string) => {
    const ds = dateStr(iso);
    return (!from || ds >= from) && (!to || ds <= to);
  };

  function presetHariIni() {
    const t = dateStr(new Date().toISOString());
    from = t;
    to = t;
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

  const filtered = $derived(txs.filter((t) => inRange(t.created_at)));
  const filteredSales = $derived(sales.filter((s) => inRange(s.created_at)));

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
    for (const t of filtered) {
      const k = bucketKey(t.created_at);
      const b = map.get(k) ?? { count: 0, total: 0, discount: 0 };
      b.count += 1;
      b.total += t.total;
      b.discount += t.discount;
      map.set(k, b);
    }
    return [...map.entries()].sort((a, b) => (a[0] < b[0] ? 1 : -1));
  });

  const byMethod = $derived.by(() => {
    const map = new Map<string, { count: number; total: number }>();
    for (const t of filtered) {
      const b = map.get(t.payment_method) ?? { count: 0, total: 0 };
      b.count += 1;
      b.total += t.total;
      map.set(t.payment_method, b);
    }
    return [...map.entries()].sort((a, b) => b[1].total - a[1].total);
  });

  const byCashier = $derived.by(() => {
    const map = new Map<string, { count: number; total: number }>();
    for (const t of filtered) {
      const b = map.get(t.cashier_id) ?? { count: 0, total: 0 };
      b.count += 1;
      b.total += t.total;
      map.set(t.cashier_id, b);
    }
    return [...map.entries()].sort((a, b) => b[1].total - a[1].total);
  });

  const topItems = $derived.by(() => {
    const map = new Map<string, number>();
    for (const s of filteredSales) map.set(s.product_name, (map.get(s.product_name) ?? 0) + s.qty);
    return [...map.entries()].sort((a, b) => b[1] - a[1]).slice(0, 10);
  });

  const grandTotal = $derived(filtered.reduce((s, t) => s + t.total, 0));
  const grandDiscount = $derived(filtered.reduce((s, t) => s + t.discount, 0));
</script>

<div class="page-head">
  <h1>Laporan Penjualan</h1>
  <div class="row no-print">
    {#each ["harian", "bulanan", "tahunan"] as g}
      <button class:btn-primary={gran === g} onclick={() => (gran = g as Gran)}>{g}</button>
    {/each}
    <button onclick={printReport}>🖨️ Print</button>
  </div>
</div>

<div class="card no-print" style="margin-bottom:1rem;">
  <div class="row" style="flex-wrap:wrap; gap:0.8rem;">
    <div><label>Dari</label><input type="date" bind:value={from} /></div>
    <div><label>Sampai</label><input type="date" bind:value={to} /></div>
    <div style="align-self:flex-end;" class="row">
      <button onclick={presetHariIni}>Hari Ini</button>
      <button onclick={presetBulanIni}>Bulan Ini</button>
      <button onclick={presetTahunIni}>Tahun Ini</button>
      <button onclick={resetRange}>Semua</button>
    </div>
  </div>
</div>

<div class="row" style="gap:1rem; margin-bottom:1rem;">
  <div class="card" style="flex:1;"><div class="text-dim">Transaksi</div><h2 class="mono">{filtered.length}</h2></div>
  <div class="card" style="flex:1;"><div class="text-dim">Total Penjualan</div><h2 class="mono">{formatIDR(grandTotal)}</h2></div>
  <div class="card" style="flex:1;"><div class="text-dim">Total Diskon</div><h2 class="mono">{formatIDR(grandDiscount)}</h2></div>
  <div class="card" style="flex:1;"><div class="text-dim">Rata-rata</div><h2 class="mono">{formatIDR(filtered.length ? grandTotal / filtered.length : 0)}</h2></div>
</div>

<div class="grid-2" style="align-items:start;">
  <div class="card" style="padding:0; overflow:hidden;">
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

  <div class="card" style="padding:0; overflow:hidden;">
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

  <div class="card" style="padding:0; overflow:hidden;">
    <div style="padding:0.7rem 0.9rem;"><b>Per Metode Pembayaran</b></div>
    <table>
      <thead><tr><th>Metode</th><th class="text-right">Trx</th><th class="text-right">Total</th></tr></thead>
      <tbody>
        {#each byMethod as [m, b] (m)}
          <tr><td>{m}</td><td class="text-right mono">{b.count}</td><td class="text-right mono">{formatIDR(b.total)}</td></tr>
        {:else}<tr><td colspan="3" class="text-dim">Tidak ada data.</td></tr>{/each}
      </tbody>
    </table>
  </div>

  <div class="card" style="padding:0; overflow:hidden;">
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
</div>
