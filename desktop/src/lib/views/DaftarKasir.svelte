<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { api } from "$lib/api";
  import { formatIDR, formatDateTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentMonthBounds } from "$lib/monthPager";
  import { todayIso } from "$lib/dateTime";
  import { currentUser } from "$lib/stores/auth";
  import { openTab, tabs } from "$lib/stores/tabs";
  import { transactionsDirty } from "$lib/stores/txSignal";
  import type { Transaction, TransactionDetail } from "$lib/types";
  import Receipt from "$lib/components/Receipt.svelte";
  import MonthPager from "$lib/components/MonthPager.svelte";
  import PrintableReportModal from "$lib/components/PrintableReportModal.svelte";
  import KasirDetailReport from "$lib/components/KasirDetailReport.svelte";

  const PAGE_SIZE = 50;

  let transactions = $state<Transaction[]>([]);
  let total = $state(0);
  let page = $state(0);
  let selectedId = $state<string | null>(null);
  let detail = $state<TransactionDetail | null>(null);
  let showLaporan = $state(false);
  let storeName = $state("GALAXYAS POS");
  let footer = $state("");
  let from = $state("");
  let to = $state("");
  let sortDir = $state<"asc" | "desc">("desc");
  let totalToday = $state(0);

  const selected = $derived(transactions.find((t) => t.id === selectedId) ?? null);
  const totalPages = $derived(Math.max(1, Math.ceil(total / PAGE_SIZE)));
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
      const res = await api.listTransactionsPage(from, to, PAGE_SIZE, page * PAGE_SIZE);
      transactions = res.items;
      total = res.total;
      const s = await api.getSettings();
      storeName = s.store_name || storeName;
      footer = s.receipt_footer || "";
    } catch (e) {
      toastError(e);
    }
  }
  function reload() {
    page = 0;
    load();
  }
  function goPage(delta: number) {
    const next = page + delta;
    if (next < 0 || next >= totalPages) return;
    page = next;
    load();
  }
  async function loadTodayTotal() {
    try {
      const today = todayIso();
      const todayTxs = await api.listTransactions(today, today, 5000);
      totalToday = todayTxs.reduce((s, t) => s + t.total, 0);
    } catch (e) {
      toastError(e);
    }
  }
  onMount(() => {
    [from, to] = currentMonthBounds();
    load();
    loadTodayTotal();
  });

  // Reload otomatis setelah transaksi diedit dari tab "Edit Kasir".
  let firstDirtyCheck = true;
  $effect(() => {
    $transactionsDirty;
    if (firstDirtyCheck) {
      firstDirtyCheck = false;
      return;
    }
    reload();
    loadTodayTotal();
  });

  function tambah() {
    const n = get(tabs).filter((t) => t.viewKey === "kasir-pos").length + 1;
    openTab({ viewKey: "kasir-pos", title: `Kasir ${n}`, icon: "🛒", singleton: false });
  }
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
      await loadTodayTotal();
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
  <MonthPager bind:from bind:to onchange={reload} />
</div>

<div class="card" style="padding:0; overflow:hidden;">
  <div style="max-height:calc(100vh - 360px); overflow:auto;">
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
  <div class="pager">
    <span class="text-dim" style="font-size:0.82rem;">
      {total.toLocaleString("id-ID")} transaksi · Hal {page + 1} / {totalPages}
    </span>
    <div class="row" style="gap:0.3rem;">
      <button disabled={page === 0} onclick={() => goPage(-1)}>‹ Sebelumnya</button>
      <button disabled={page + 1 >= totalPages} onclick={() => goPage(1)}>Berikutnya ›</button>
    </div>
  </div>
</div>

<div class="bottom-bar">
  <button class="btn-primary" onclick={tambah}>➕ Tambah</button>
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
  .pager {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.5rem 0.9rem; border-top: 1px solid var(--border);
  }
</style>
