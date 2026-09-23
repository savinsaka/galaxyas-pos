<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR, formatDateTime } from "$lib/format";
  import { toastError } from "$lib/toast";
  import { formatPeriodLabel } from "$lib/dateTime";
  import TemplateReport from "$lib/components/TemplateReport.svelte";
  import { hasActiveTemplate, loadActiveTemplate } from "$lib/report/storage";
  import type { ReportContext } from "$lib/report/kinds";
  import type { ReportTemplate } from "$lib/report/template";
  import type { SalesItemDetailRow } from "$lib/types";

  let { from, to, brands }: { from: string; to: string; brands: string[] } = $props();

  let rows = $state<SalesItemDetailRow[]>([]);
  let activeTpl = $state<ReportTemplate | null>(null);
  onMount(() => {
    hasActiveTemplate("item-detail").then(async (has) => {
      if (has) activeTpl = await loadActiveTemplate("item-detail");
    });
  });

  async function load() {
    const f = from || "0001-01-01";
    const t = to || "9999-12-31";
    try {
      rows = await api.salesItemDetailReport(f, t, brands);
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
    kind: "item-detail",
    title: "Laporan Item Detail",
    subtitle: brands.length ? `${formatPeriodLabel(from, to)} · Merek: ${brands.join(", ")}` : formatPeriodLabel(from, to),
    meta: "",
    scalars: {
      title: "Laporan Item Detail",
      subtitle: brands.length ? `${formatPeriodLabel(from, to)} · Merek: ${brands.join(", ")}` : formatPeriodLabel(from, to),
      meta: "",
    },
    datasets: {
      rows: rows.map((r) => ({
        invoice_no: r.invoice_no,
        created_at: formatDateTime(r.created_at),
        cashier_id: r.cashier_id,
        name: r.name,
        barcode: r.barcode ?? "-",
        brand: r.brand ?? "-",
        qty: r.qty,
        price: r.price,
        discount: r.discount,
        net: r.net,
      })),
    },
  });
</script>

{#if activeTpl}
  <TemplateReport template={activeTpl} ctx={tplCtx} />
{:else}
<div class="card" style="padding:0; overflow:hidden;">
  <table>
    <thead>
      <tr>
        <th>Invoice</th><th>Tanggal</th><th>Kasir</th><th>Barang</th><th>Barcode</th><th>Merek</th>
        <th class="text-right">Qty</th><th class="text-right">Harga</th><th class="text-right">Diskon</th><th class="text-right">Total</th>
      </tr>
    </thead>
    <tbody>
      {#each rows as r, i (i)}
        <tr>
          <td class="mono">{r.invoice_no}</td>
          <td>{formatDateTime(r.created_at)}</td>
          <td>{r.cashier_id}</td>
          <td>{r.name}</td>
          <td class="mono text-dim">{r.barcode ?? "-"}</td>
          <td class="text-dim">{r.brand ?? "-"}</td>
          <td class="text-right mono">{r.qty}</td>
          <td class="text-right mono">{formatIDR(r.price)}</td>
          <td class="text-right mono">{formatIDR(r.discount)}</td>
          <td class="text-right mono fw-bold">{formatIDR(r.net)}</td>
        </tr>
      {:else}
        <tr><td colspan="10" class="text-dim">Tidak ada data.</td></tr>
      {/each}
    </tbody>
    {#if rows.length}
      <tfoot>
        <tr>
          <td colspan="6" class="fw-bold">Total Pendapatan ({rows.length.toLocaleString("id-ID")} baris)</td>
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
