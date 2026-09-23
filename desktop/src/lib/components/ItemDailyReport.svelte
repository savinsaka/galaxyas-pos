<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR } from "$lib/format";
  import { toastError } from "$lib/toast";
  import { formatPeriodLabel } from "$lib/dateTime";
  import TemplateReport from "$lib/components/TemplateReport.svelte";
  import { hasActiveTemplate, loadActiveTemplate } from "$lib/report/storage";
  import type { ReportContext } from "$lib/report/kinds";
  import type { ReportTemplate } from "$lib/report/template";
  import type { DailySalesRow } from "$lib/types";

  let { from, to, brands }: { from: string; to: string; brands: string[] } = $props();

  let rows = $state<DailySalesRow[]>([]);
  let activeTpl = $state<ReportTemplate | null>(null);
  onMount(() => {
    hasActiveTemplate("item-daily").then(async (has) => {
      if (has) activeTpl = await loadActiveTemplate("item-daily");
    });
  });

  async function load() {
    const f = from || "0001-01-01";
    const t = to || "9999-12-31";
    try {
      rows = await api.dailySalesReport(f, t, brands);
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

  const totalQty = $derived(rows.reduce((s, r) => s + r.qty, 0));
  const totalDiscount = $derived(rows.reduce((s, r) => s + r.discount, 0));
  const totalNet = $derived(rows.reduce((s, r) => s + r.net, 0));

  const tplCtx = $derived<ReportContext>({
    kind: "item-daily",
    title: "Laporan Item Per Hari",
    subtitle: brands.length ? `${formatPeriodLabel(from, to)} · Merek: ${brands.join(", ")}` : formatPeriodLabel(from, to),
    meta: "",
    scalars: {
      title: "Laporan Item Per Hari",
      subtitle: brands.length ? `${formatPeriodLabel(from, to)} · Merek: ${brands.join(", ")}` : formatPeriodLabel(from, to),
      meta: "",
    },
    datasets: {
      rows: rows.map((r) => ({ day: r.day, qty: r.qty, gross: r.gross, discount: r.discount, net: r.net })),
    },
  });
</script>

{#if activeTpl}
  <TemplateReport template={activeTpl} ctx={tplCtx} />
{:else}
<div class="card" style="padding:0; overflow:hidden;">
  <table>
    <thead>
      <tr><th>Tanggal</th><th class="text-right">Qty</th><th class="text-right">Gross</th><th class="text-right">Diskon</th><th class="text-right">Total</th></tr>
    </thead>
    <tbody>
      {#each rows as r (r.day)}
        <tr>
          <td class="mono">{r.day}</td>
          <td class="text-right mono">{r.qty}</td>
          <td class="text-right mono">{formatIDR(r.gross)}</td>
          <td class="text-right mono">{formatIDR(r.discount)}</td>
          <td class="text-right mono fw-bold">{formatIDR(r.net)}</td>
        </tr>
      {:else}
        <tr><td colspan="5" class="text-dim">Tidak ada data.</td></tr>
      {/each}
    </tbody>
    {#if rows.length}
      <tfoot>
        <tr>
          <td class="fw-bold">Total</td>
          <td class="text-right mono fw-bold">{totalQty}</td>
          <td></td>
          <td class="text-right mono fw-bold">{formatIDR(totalDiscount)}</td>
          <td class="text-right mono fw-bold">{formatIDR(totalNet)}</td>
        </tr>
      </tfoot>
    {/if}
  </table>
</div>
{/if}

<style>
  .fw-bold { font-weight: 700; }
  tfoot td { border-top: 2px solid var(--border); padding: 0.5rem 0.6rem; }
</style>
