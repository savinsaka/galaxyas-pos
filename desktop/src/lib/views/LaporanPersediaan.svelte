<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR, formatQty } from "$lib/format";
  import { toastError } from "$lib/toast";
  import { printReport } from "$lib/print";
  import type { ProductWithStock, StockMovement } from "$lib/types";

  type Gran = "harian" | "bulanan" | "tahunan";
  let gran = $state<Gran>("bulanan");
  let from = $state("");
  let to = $state("");
  let movements = $state<StockMovement[]>([]);
  let products = $state<ProductWithStock[]>([]);

  async function load() {
    try {
      [movements, products] = await Promise.all([
        api.listStockMovements(null, null, null, 5000),
        api.listProducts("", true),
      ]);
    } catch (e) {
      toastError(e);
    }
  }
  onMount(() => {
    presetTahunIni();
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

  const filtered = $derived(movements.filter((m) => inRange(m.created_at)));

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
    const map = new Map<string, { masuk: number; keluar: number; jual: number; opname: number }>();
    for (const mv of filtered) {
      const k = bucketKey(mv.created_at);
      const b = map.get(k) ?? { masuk: 0, keluar: 0, jual: 0, opname: 0 };
      if (mv.kind === "in") b.masuk += mv.qty;
      else if (mv.kind === "out") b.keluar += mv.qty;
      else if (mv.kind === "sale") b.jual += mv.qty;
      else if (mv.kind === "opname") b.opname += 1;
      map.set(k, b);
    }
    return [...map.entries()].sort((a, b) => (a[0] < b[0] ? 1 : -1));
  });

  const perItem = $derived.by(() => {
    const map = new Map<string, { masuk: number; keluar: number; jual: number }>();
    for (const mv of filtered) {
      const b = map.get(mv.product_name) ?? { masuk: 0, keluar: 0, jual: 0 };
      if (mv.kind === "in") b.masuk += mv.qty;
      else if (mv.kind === "out") b.keluar += mv.qty;
      else if (mv.kind === "sale") b.jual += mv.qty;
      map.set(mv.product_name, b);
    }
    return [...map.entries()].sort((a, b) => b[1].jual - a[1].jual);
  });

  const stockValueCost = $derived(products.reduce((s, p) => s + p.stock_qty * p.cost_price, 0));
  const stockValueSell = $derived(products.reduce((s, p) => s + p.stock_qty * p.sell_price, 0));
  const lowStock = $derived(products.filter((p) => p.stock_qty <= 5));
</script>

<div class="page-head">
  <h1>Laporan Persediaan</h1>
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
      <button onclick={presetBulanIni}>Bulan Ini</button>
      <button onclick={presetTahunIni}>Tahun Ini</button>
      <button onclick={resetRange}>Semua</button>
    </div>
  </div>
</div>

<div class="row" style="gap:1rem; margin-bottom:1rem;">
  <div class="card" style="flex:1;"><div class="text-dim">Nilai Stok (Pokok)</div><h2 class="mono">{formatIDR(stockValueCost)}</h2></div>
  <div class="card" style="flex:1;"><div class="text-dim">Nilai Stok (Jual)</div><h2 class="mono">{formatIDR(stockValueSell)}</h2></div>
  <div class="card" style="flex:1;"><div class="text-dim">Stok Menipis (≤5)</div><h2 class="mono">{lowStock.length}</h2></div>
</div>

<div class="grid-2" style="align-items:start;">
  <div class="card" style="padding:0; overflow:hidden;">
    <div style="padding:0.7rem 0.9rem;"><b>Pergerakan per Periode ({gran})</b></div>
    <table>
      <thead><tr><th>Periode</th><th class="text-right">Masuk</th><th class="text-right">Keluar</th><th class="text-right">Terjual</th><th class="text-right">Opname</th></tr></thead>
      <tbody>
        {#each buckets as [k, b] (k)}
          <tr><td class="mono">{k}</td><td class="text-right mono">{formatQty(b.masuk)}</td><td class="text-right mono">{formatQty(b.keluar)}</td><td class="text-right mono">{formatQty(b.jual)}</td><td class="text-right mono">{b.opname}</td></tr>
        {:else}<tr><td colspan="5" class="text-dim">Tidak ada data.</td></tr>{/each}
      </tbody>
    </table>
  </div>

  <div class="card" style="padding:0; overflow:hidden;">
    <div style="padding:0.7rem 0.9rem;"><b>Stok Menipis</b></div>
    <table>
      <thead><tr><th>Barang</th><th class="text-right">Stok</th></tr></thead>
      <tbody>
        {#each lowStock as p (p.id)}
          <tr><td>{p.name}</td><td class="text-right mono">{formatQty(p.stock_qty)} {p.unit ?? ""}</td></tr>
        {:else}<tr><td colspan="2" class="text-dim">Semua stok aman.</td></tr>{/each}
      </tbody>
    </table>
  </div>

  <div class="card" style="padding:0; overflow:hidden; grid-column:1/3;">
    <div style="padding:0.7rem 0.9rem;"><b>Rekap per Barang</b></div>
    <table>
      <thead><tr><th>Barang</th><th class="text-right">Masuk</th><th class="text-right">Keluar</th><th class="text-right">Terjual</th></tr></thead>
      <tbody>
        {#each perItem as [name, b] (name)}
          <tr><td>{name}</td><td class="text-right mono">{formatQty(b.masuk)}</td><td class="text-right mono">{formatQty(b.keluar)}</td><td class="text-right mono">{formatQty(b.jual)}</td></tr>
        {:else}<tr><td colspan="4" class="text-dim">Tidak ada data.</td></tr>{/each}
      </tbody>
    </table>
  </div>
</div>
