<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR, formatDateTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import { currentMonthBounds } from "$lib/monthPager";
  import type { Expense, ExpenseInput } from "$lib/types";
  import MonthPager from "$lib/components/MonthPager.svelte";

  const CATEGORIES = ["Listrik", "Air", "Gaji", "Uang Makan", "Sewa", "Perlengkapan", "Transport", "Lainnya"];

  let expenses = $state<Expense[]>([]);
  let from = $state("");
  let to = $state("");

  function blank(): ExpenseInput {
    return { id: null, date: new Date().toISOString().slice(0, 10), category: CATEGORIES[0], amount: 0, note: "" };
  }
  let form = $state<ExpenseInput>(blank());
  let busy = $state(false);

  async function load() {
    try {
      expenses = await api.listExpenses(from || null, to || null);
    } catch (e) {
      toastError(e);
    }
  }
  onMount(() => {
    [from, to] = currentMonthBounds();
    load();
  });

  async function save() {
    if (!form.category.trim()) return showToast("Kategori wajib diisi.", "error");
    if (!form.amount || form.amount <= 0) return showToast("Nominal harus lebih dari 0.", "error");
    busy = true;
    try {
      await api.saveExpense({ ...form, user_id: $currentUser?.username ?? null });
      showToast("Pengeluaran tersimpan.", "success");
      form = blank();
      await load();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  async function remove(e: Expense) {
    if (!confirm(`Hapus pengeluaran "${e.category}" (${formatIDR(e.amount)})?`)) return;
    try {
      await api.deleteExpense(e.id);
      await load();
    } catch (err) {
      toastError(err);
    }
  }

  const total = $derived(expenses.reduce((s, e) => s + e.amount, 0));
</script>

<div class="page-head"><h1>Pengeluaran (Kas Keluar)</h1></div>

<div class="grid-2" style="align-items:start;">
  <div class="card">
    <h2>Catat Pengeluaran</h2>
    <div class="grid-2">
      <div>
        <label>Tanggal</label>
        <input type="date" bind:value={form.date} />
      </div>
      <div>
        <label>Kategori</label>
        <select bind:value={form.category}>
          {#each CATEGORIES as c}<option value={c}>{c}</option>{/each}
        </select>
      </div>
      <div style="grid-column:1/3;">
        <label>Nominal (Rp)</label>
        <input type="number" min="0" bind:value={form.amount} />
      </div>
      <div style="grid-column:1/3;">
        <label>Catatan</label>
        <input bind:value={form.note} placeholder="opsional" />
      </div>
    </div>
    <div class="row" style="justify-content:flex-end; margin-top:1rem;">
      <button class="btn-primary" disabled={busy} onclick={save}>💾 Simpan Pengeluaran</button>
    </div>
  </div>

  <div class="card" style="padding:0; overflow:hidden;">
    <div style="padding:0.7rem 0.9rem; display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:0.5rem;">
      <MonthPager bind:from bind:to onchange={load} />
      <b class="mono">Total: {formatIDR(total)}</b>
    </div>
    <table>
      <thead><tr><th>Tanggal</th><th>Kategori</th><th class="text-right">Nominal</th><th>Catatan</th><th></th></tr></thead>
      <tbody>
        {#each expenses as e (e.id)}
          <tr>
            <td class="mono">{e.date}</td>
            <td>{e.category}</td>
            <td class="text-right mono">{formatIDR(e.amount)}</td>
            <td class="text-dim">{e.note ?? "-"}</td>
            <td><button class="btn-ghost" style="color:var(--danger);" onclick={() => remove(e)}>Hapus</button></td>
          </tr>
        {:else}
          <tr><td colspan="5" class="text-dim">Belum ada pengeluaran.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
