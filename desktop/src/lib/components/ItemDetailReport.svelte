<script lang="ts">
  import { api } from "$lib/api";
  import { formatIDR, formatDateTime } from "$lib/format";
  import { toastError } from "$lib/toast";
  import type { SalesItemDetailRow } from "$lib/types";

  let { from, to, brands }: { from: string; to: string; brands: string[] } = $props();

  let rows = $state<SalesItemDetailRow[]>([]);

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
</script>

<div class="card" style="padding:0; overflow:hidden;">
  <table>
    <thead>
      <tr>
        <th>Invoice</th><th>Tanggal</th><th>Kasir</th><th>Barang</th><th>Merek</th>
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
          <td class="text-dim">{r.brand ?? "-"}</td>
          <td class="text-right mono">{r.qty}</td>
          <td class="text-right mono">{formatIDR(r.price)}</td>
          <td class="text-right mono">{formatIDR(r.discount)}</td>
          <td class="text-right mono fw-bold">{formatIDR(r.net)}</td>
        </tr>
      {:else}
        <tr><td colspan="9" class="text-dim">Tidak ada data.</td></tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .fw-bold { font-weight: 700; }
</style>
