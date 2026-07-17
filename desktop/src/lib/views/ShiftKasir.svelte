<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR, formatDateTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import { printElement } from "$lib/print";
  import type { Shift } from "$lib/types";

  let active = $state<Shift | null>(null);
  let history = $state<Shift[]>([]);
  let openingCash = $state(0);
  let closingCash = $state(0);
  let closeNote = $state("");
  let busy = $state(false);
  let closedResult = $state<Shift | null>(null);

  async function load() {
    try {
      [active, history] = await Promise.all([api.getActiveShift(), api.listShifts(50)]);
    } catch (e) {
      toastError(e);
    }
  }
  onMount(load);

  async function doOpen() {
    if (!$currentUser) return;
    busy = true;
    try {
      active = await api.openShift({
        user_id: $currentUser.username,
        user_name: $currentUser.name,
        opening_cash: openingCash,
      });
      showToast("Shift dibuka.", "success");
      openingCash = 0;
      await load();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  async function doClose() {
    if (!active) return;
    if (!confirm("Tutup shift sekarang? Pastikan uang di laci sudah dihitung.")) return;
    busy = true;
    try {
      closedResult = await api.closeShift({ id: active.id, closing_cash: closingCash, note: closeNote });
      showToast("Shift ditutup.", "success");
      closingCash = 0;
      closeNote = "";
      await load();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div id="printable-page">
<div class="page-head">
  <h1>Manajemen Shift &amp; Tutup Kasir</h1>
  {#if closedResult}<button class="no-print" onclick={() => printElement("printable-page", "Laporan Rekonsiliasi Shift")}>🖨️ Print Rekonsiliasi</button>{/if}
</div>

{#if !active}
  <div class="card" style="max-width:420px;">
    <h2>🟢 Buka Shift</h2>
    <p class="text-dim" style="margin-top:0; font-size:0.83rem;">
      Masukkan modal awal (uang tunai) di laci sebelum mulai melayani transaksi.
    </p>
    <label>Modal Awal (Rp)</label>
    <input type="number" min="0" bind:value={openingCash} />
    <div class="row" style="justify-content:flex-end; margin-top:1rem;">
      <button class="btn-primary" disabled={busy} onclick={doOpen}>Buka Shift</button>
    </div>
  </div>
{:else}
  <div class="card" style="max-width:480px;">
    <h2>🟡 Shift Berjalan</h2>
    <div class="trow"><span>Kasir</span><span>{active.user_name}</span></div>
    <div class="trow"><span>Dibuka</span><span class="mono">{formatDateTime(active.opened_at)}</span></div>
    <div class="trow"><span>Modal Awal</span><span class="mono">{formatIDR(active.opening_cash)}</span></div>

    <label style="margin-top:0.9rem;">Uang Fisik di Laci Sekarang (Rp)</label>
    <input type="number" min="0" bind:value={closingCash} />
    <label style="margin-top:0.6rem;">Catatan</label>
    <input bind:value={closeNote} placeholder="opsional" />
    <div class="row" style="justify-content:flex-end; margin-top:1rem;">
      <button class="btn-danger" disabled={busy} onclick={doClose}>🔒 Tutup Shift (End of Day)</button>
    </div>
  </div>
{/if}

{#if closedResult}
  <div class="card recon-card" style="max-width:480px; margin-top:1rem;">
    <h2>📋 Laporan Rekonsiliasi</h2>
    <div class="trow"><span>Kasir</span><span>{closedResult.user_name}</span></div>
    <div class="trow"><span>Dibuka</span><span class="mono">{formatDateTime(closedResult.opened_at)}</span></div>
    <div class="trow"><span>Ditutup</span><span class="mono">{formatDateTime(closedResult.closed_at ?? "")}</span></div>
    <div class="r-sep"></div>
    <div class="trow"><span>Modal Awal</span><span class="mono">{formatIDR(closedResult.opening_cash)}</span></div>
    <div class="trow"><span>Uang di Sistem (Modal + Tunai)</span><span class="mono">{formatIDR(closedResult.expected_cash ?? 0)}</span></div>
    <div class="trow"><span>Uang Fisik di Laci</span><span class="mono">{formatIDR(closedResult.closing_cash ?? 0)}</span></div>
    <div class="trow" style="font-weight:700;">
      <span>Selisih</span>
      <span class="mono" style="color:{(closedResult.difference ?? 0) < 0 ? 'var(--danger)' : (closedResult.difference ?? 0) > 0 ? 'var(--warning)' : 'inherit'}">
        {formatIDR(closedResult.difference ?? 0)}
      </span>
    </div>
    {#if closedResult.note}<div class="trow"><span>Catatan</span><span>{closedResult.note}</span></div>{/if}
  </div>
{/if}

<div class="card" style="padding:0; overflow:hidden; margin-top:1rem;">
  <div style="padding:0.7rem 0.9rem; font-weight:650; border-bottom:1px solid var(--border);">Riwayat Shift</div>
  <table>
    <thead>
      <tr><th>Kasir</th><th>Dibuka</th><th>Ditutup</th><th class="text-right">Modal</th><th class="text-right">Sistem</th><th class="text-right">Fisik</th><th class="text-right">Selisih</th></tr>
    </thead>
    <tbody>
      {#each history as s (s.id)}
        <tr>
          <td>{s.user_name}</td>
          <td class="mono">{formatDateTime(s.opened_at)}</td>
          <td class="mono">{s.closed_at ? formatDateTime(s.closed_at) : "—"}</td>
          <td class="text-right mono">{formatIDR(s.opening_cash)}</td>
          <td class="text-right mono">{s.expected_cash != null ? formatIDR(s.expected_cash) : "—"}</td>
          <td class="text-right mono">{s.closing_cash != null ? formatIDR(s.closing_cash) : "—"}</td>
          <td class="text-right mono">{s.difference != null ? formatIDR(s.difference) : "—"}</td>
        </tr>
      {:else}
        <tr><td colspan="7" class="text-dim">Belum ada riwayat shift.</td></tr>
      {/each}
    </tbody>
  </table>
</div>
</div>

<style>
  .trow { display: flex; justify-content: space-between; padding: 0.18rem 0; }
  .r-sep { border-top: 1px dashed var(--border); margin: 0.5rem 0; }
</style>
