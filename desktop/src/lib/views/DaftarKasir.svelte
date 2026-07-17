<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR, formatDateTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentMonthBounds } from "$lib/monthPager";
  import { currentUser } from "$lib/stores/auth";
  import { openTab } from "$lib/stores/tabs";
  import { transactionsDirty } from "$lib/stores/txSignal";
  import type { Transaction, TransactionDetail } from "$lib/types";
  import Receipt from "$lib/components/Receipt.svelte";
  import MonthPager from "$lib/components/MonthPager.svelte";
  import PrintableReportModal from "$lib/components/PrintableReportModal.svelte";
  import KasirDetailReport from "$lib/components/KasirDetailReport.svelte";

  let transactions = $state<Transaction[]>([]);
  let selectedId = $state<string | null>(null);
  let detail = $state<TransactionDetail | null>(null);
  let showLaporan = $state(false);
  let storeName = $state("GALAXYAS POS");
  let footer = $state("");
  let from = $state("");
  let to = $state("");
  let sortDir = $state<"asc" | "desc">("desc");

  const selected = $derived(transactions.find((t) => t.id === selectedId) ?? null);
  const totalToday = $derived(
    transactions
      .filter((t) => new Date(t.created_at).toDateString() === new Date().toDateString())
      .reduce((s, t) => s + t.total, 0),
  );
  const sortedTransactions = $derived(
    [...transactions].sort((a, b) =>
      sortDir === "asc" ? a.created_at.localeCompare(b.created_at) : b.created_at.localeCompare(a.created_at),
    ),
  );
  function toggleSort() {
    sortDir = sortDir === "asc" ? "desc" : "asc";
  }

  async function load() {
    try {
      transactions = await api.listTransactions(from, to, 1000);
      const s = await api.getSettings();
      storeName = s.store_name || storeName;
      footer = s.receipt_footer || "";
    } catch (e) {
      toastError(e);
    }
  }
  onMount(() => {
    [from, to] = currentMonthBounds();
    load();
  });

  // Reload otomatis setelah transaksi diedit dari tab "Edit Kasir".
  let firstDirtyCheck = true;
  $effect(() => {
    $transactionsDirty;
    if (firstDirtyCheck) {
      firstDirtyCheck = false;
      return;
    }
    load();
  });

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
  function edit() {
    if (!selected) return showToast("Pilih transaksi dulu.", "info");
    openTab({
      viewKey: "edit-kasir",
      title: `Edit Kasir — ${selected.invoice_no}`,
      icon: "✏️",
      singleton: true,
      props: { transactionId: selected.id },
    });
  }
</script>

<div class="page-head">
  <h1>Daftar Kasir</h1>
  <span class="text-dim">Penjualan hari ini: <strong class="mono">{formatIDR(totalToday)}</strong></span>
</div>

<div class="card" style="margin-bottom:0.8rem;">
  <MonthPager bind:from bind:to onchange={load} />
</div>

<div class="card" style="padding:0; overflow:hidden;">
  <div style="max-height:calc(100vh - 320px); overflow:auto;">
    <table>
      <thead>
        <tr>
          <th>Invoice</th>
          <th class="sortable" onclick={toggleSort}>Waktu{sortDir === "asc" ? " ↑" : " ↓"}</th>
          <th>Kasir</th><th>Metode</th><th class="text-right">Total</th>
        </tr>
      </thead>
      <tbody>
        {#each sortedTransactions as t (t.id)}
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
  {#if $currentUser?.role === "admin"}
    <button onclick={edit} disabled={!selected}>✏️ Edit</button>
  {/if}
  <button class="btn-danger" onclick={hapus} disabled={!selected}>🗑️ Hapus</button>
  <button class="btn-primary" onclick={() => (showLaporan = true)}>📊 Laporan</button>
</div>

{#if detail}
  <Receipt {detail} {storeName} {footer} onClose={() => (detail = null)} />
{/if}

{#if showLaporan}
  <PrintableReportModal title="Laporan Kasir Detail" onClose={() => (showLaporan = false)}>
    {#snippet children()}
      <KasirDetailReport />
    {/snippet}
  </PrintableReportModal>
{/if}

<style>
  .sortable { cursor: pointer; user-select: none; }
  .sortable:hover { color: var(--primary); }
</style>
