<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR } from "$lib/format";
  import { toastError } from "$lib/toast";
  import { REPORT_TYPES, defaultConfig, loadReportDesign, blockOrder, blockHidden, type ReportDesignConfig } from "$lib/reportDesign";
  import type { BrandSalesRow, ProductSalesRow } from "$lib/types";

  let { from, to, brands }: { from: string; to: string; brands: string[] } = $props();

  const BLOCKS = REPORT_TYPES.find((t) => t.key === "item-recap")!.blocks;
  let design = $state<ReportDesignConfig>(defaultConfig(BLOCKS));
  onMount(() => {
    loadReportDesign("item-recap", BLOCKS).then((d) => (design = d));
  });

  let productReport = $state<ProductSalesRow[]>([]);
  let brandReport = $state<BrandSalesRow[]>([]);

  async function load() {
    const f = from || "0001-01-01";
    const t = to || "9999-12-31";
    try {
      [productReport, brandReport] = await Promise.all([
        api.productSalesReport(f, t, brands),
        api.brandSalesReport(f, t, brands),
      ]);
    } catch (e) {
      toastError(e);
    }
  }

  $effect(() => {
    from;
    to;
    brands;
    load();
  });

  const totalQty = $derived(productReport.reduce((s, r) => s + r.qty, 0));
  const totalDiscount = $derived(productReport.reduce((s, r) => s + r.discount, 0));
  const totalNet = $derived(productReport.reduce((s, r) => s + r.net, 0));
</script>

<div class="grid-2" style="align-items:start;">
  {#if !blockHidden(design, "per_barang")}
    <div class="card" style="padding:0; overflow:hidden; order:{blockOrder(design, 'per_barang')};">
      <div style="padding:0.7rem 0.9rem;">
        <b>Per Barang{brands.length ? ` — ${brands.join(", ")}` : ""}</b>
      </div>
      <table>
        <thead><tr><th>Barang</th><th>Merek</th><th class="text-right">Qty</th><th class="text-right">Diskon</th><th class="text-right">Total</th></tr></thead>
        <tbody>
          {#each productReport as r (r.product_id)}
            <tr><td>{r.name}</td><td class="text-dim">{r.brand ?? "-"}</td><td class="text-right mono">{r.qty}</td><td class="text-right mono">{formatIDR(r.discount)}</td><td class="text-right mono">{formatIDR(r.net)}</td></tr>
          {:else}<tr><td colspan="5" class="text-dim">Tidak ada data.</td></tr>{/each}
        </tbody>
        {#if productReport.length}
          <tfoot>
            <tr><td colspan="2" class="fw-bold">Total</td><td class="text-right mono fw-bold">{totalQty}</td><td class="text-right mono fw-bold">{formatIDR(totalDiscount)}</td><td class="text-right mono fw-bold">{formatIDR(totalNet)}</td></tr>
          </tfoot>
        {/if}
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
</div>

<style>
  .fw-bold { font-weight: 700; }
  tfoot td { border-top: 2px solid var(--border); padding: 0.5rem 0.6rem; }
</style>
