<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR, formatDateTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { openTab } from "$lib/stores/tabs";
  import type { Transaction, TransactionDetail } from "$lib/types";
  import Receipt from "$lib/components/Receipt.svelte";

  let transactions = $state<Transaction[]>([]);
  let selectedId = $state<string | null>(null);
  let detail = $state<TransactionDetail | null>(null);
  let storeName = $state("GALAXYAS POS");
  let footer = $state("");

  const selected = $derived(transactions.find((t) => t.id === selectedId) ?? null);
  const totalToday = $derived(
    transactions
      .filter((t) => new Date(t.created_at).toDateString() === new Date().toDateString())
      .reduce((s, t) => s + t.total, 0),
  );

  async function load() {
    try {
      transactions = await api.listTransactions(300);
      const s = await api.getSettings();
      storeName = s.store_name || storeName;
      footer = s.receipt_footer || "";
    } catch (e) {
      toastError(e);
    }
  }
  onMount(load);

  async function lihat() {
    if (!selected) return showToast("Pilih transaksi dulu.", "info");
    try {
      detail = await api.getTransaction(selected.id);
    } catch (e) {
      toastError(e);
    }
  }
  async function hapus() {
    if (!selected) return showToast("Pilih transaksi dulu.", "info");
    if (!confirm(`Hapus transaksi ${selected.invoice_no}? Stok akan dikembalikan.`)) return;
    try {
      await api.deleteTransaction(selected.id);
      showToast("Transaksi dihapus, stok dikembalikan.", "success");
      selectedId = null;
      await load();
    } catch (e) {
      toastError(e);
    }
  }
</script>

<div class="page-head">
  <h1>Daftar Kasir</h1>
  <span class="text-dim">Penjualan hari ini: <strong class="mono">{formatIDR(totalToday)}</strong></span>
</div>

<div class="card" style="padding:0; overflow:hidden;">
  <div style="max-height:calc(100vh - 320px); overflow:auto;">
    <table>
      <thead>
        <tr><th>Invoice</th><th>Waktu</th><th>Kasir</th><th>Metode</th><th class="text-right">Total</th></tr>
      </thead>
      <tbody>
        {#each transactions as t (t.id)}
          <tr onclick={() => (selectedId = t.id)} style={selectedId === t.id ? "background:var(--baby-blue-soft);" : ""}>
            <td class="mono">{t.invoice_no}</td>
            <td>{formatDateTime(t.created_at)}</td>
            <td>{t.cashier_id}</td>
            <td>{t.payment_method}</td>
            <td class="text-right mono">{formatIDR(t.total)}</td>
          </tr>
        {:else}
          <tr><td colspan="5" class="text-dim">Belum ada transaksi.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<div class="bottom-bar">
  <button onclick={lihat} disabled={!selected}>🧾 Lihat Struk</button>
  <button class="btn-danger" onclick={hapus} disabled={!selected}>🗑️ Hapus</button>
  <button class="btn-primary" onclick={() => openTab({ viewKey: "laporan-penjualan", title: "Laporan Penjualan", icon: "💰", singleton: true })}>📊 Laporan</button>
</div>

{#if detail}
  <Receipt {detail} {storeName} {footer} onClose={() => (detail = null)} />
{/if}
