<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatDateTime } from "$lib/format";
  import { updateState, updateNow } from "$lib/updater";
  import type { UpdatePolicy } from "$lib/types";

  // Gerbang "Wajib Update": server (jjapps.net) bisa menetapkan versi minimum.
  // Versi app di bawahnya → pita merah sampai batas waktu, lalu layar merah
  // penuh yang mengunci app. Server tidak terjangkau = tidak ada kewajiban.
  let policy = $state<UpdatePolicy | null>(null);
  let now = $state(Date.now());
  let busy = $state(false);
  let note = $state<string | null>(null);

  const deadlineMs = $derived(policy?.deadline ? Date.parse(policy.deadline) : null);
  const blocked = $derived(
    !!policy?.must_update && (policy.blocked || (deadlineMs !== null && now >= deadlineMs)),
  );
  const warning = $derived(!!policy?.must_update && !blocked);

  async function refresh() {
    try {
      policy = await api.appUpdatePolicy();
    } catch {
      /* offline / perintah gagal: jangan pernah mengunci app karena ini */
    }
  }

  let tickTimer: ReturnType<typeof setInterval> | undefined;
  let pollTimer: ReturnType<typeof setInterval> | undefined;
  onMount(() => {
    refresh();
    tickTimer = setInterval(() => (now = Date.now()), 30_000);
    pollTimer = setInterval(refresh, 60 * 60_000);
  });
  onDestroy(() => {
    clearInterval(tickTimer);
    clearInterval(pollTimer);
  });

  // Pita merah menggeser layar app ke bawah (lihat --gate-offset di app.css)
  // supaya tidak menutupi ribbon menu.
  $effect(() => {
    document.documentElement.style.setProperty("--gate-offset", warning ? "34px" : "0px");
  });

  async function doUpdate() {
    busy = true;
    note = null;
    try {
      const found = await updateNow();
      if (!found) note = "Update belum tersedia di server rilis. Coba lagi beberapa saat lagi atau hubungi admin.";
    } finally {
      busy = false;
    }
  }

  const downloading = $derived($updateState.status === "downloading" || $updateState.status === "ready");
  const progress = $derived($updateState.status === "downloading" ? $updateState.progress : 100);
</script>

{#if blocked && policy}
  <div class="gate-block" role="alertdialog" aria-modal="true">
    <div class="gate-card">
      <div class="gate-icon">⛔</div>
      <h1>Aplikasi Harus Diupdate</h1>
      <p>
        Versi ini (<b>{policy.current}</b>) sudah tidak boleh dipakai. Update ke versi
        <b>{policy.min_version}</b> atau lebih baru untuk melanjutkan.
      </p>
      {#if policy.message}<p class="gate-msg">{policy.message}</p>{/if}
      {#if downloading}
        <div class="gate-bar"><div class="gate-fill" style="width:{progress}%"></div></div>
        <p class="gate-small">{$updateState.status === "ready" ? "Memasang update, aplikasi akan restart…" : `Mengunduh ${progress}%`}</p>
      {:else}
        <button class="gate-btn" disabled={busy} onclick={doUpdate}>{busy ? "Memeriksa…" : "⬇ Update Sekarang"}</button>
      {/if}
      {#if $updateState.status === "error"}<p class="gate-small">Update gagal: {$updateState.message}</p>{/if}
      {#if note}<p class="gate-small">{note}</p>{/if}
    </div>
  </div>
{:else if warning && policy}
  <div class="gate-ribbon" role="alert">
    <span>
      ⚠ <b>Wajib update</b> ke versi {policy.min_version}
      {#if policy.deadline}sebelum <b>{formatDateTime(policy.deadline)}</b>{/if} — sesudah itu aplikasi tidak bisa dipakai.
      {#if policy.message}{policy.message}{/if}
    </span>
    {#if downloading}
      <span class="gate-ribbon-progress">Mengunduh {progress}%</span>
    {:else}
      <button disabled={busy} onclick={doUpdate}>{busy ? "Memeriksa…" : "Update Sekarang"}</button>
    {/if}
    {#if note}<span class="gate-ribbon-note">{note}</span>{/if}
  </div>
{/if}

<style>
  .gate-block {
    position: fixed; inset: 0; z-index: 100000;
    background: #b71c1c; color: #fff;
    display: flex; align-items: center; justify-content: center; padding: 1rem;
  }
  .gate-card { max-width: 520px; text-align: center; }
  .gate-card h1 { color: #fff; margin: 0.3rem 0 0.8rem; font-size: 1.8rem; }
  .gate-card p { margin: 0 0 0.8rem; line-height: 1.5; }
  .gate-icon { font-size: 3.5rem; }
  .gate-msg { background: rgba(0, 0, 0, 0.2); border-radius: 8px; padding: 0.6rem 0.8rem; }
  .gate-btn {
    background: #fff; color: #b71c1c; border: none; font-weight: 800;
    padding: 0.9rem 1.6rem; font-size: 1.05rem; border-radius: 10px; margin-top: 0.4rem;
  }
  .gate-small { font-size: 0.85rem; opacity: 0.9; margin-top: 0.6rem; }
  .gate-bar { height: 10px; border-radius: 999px; background: rgba(255, 255, 255, 0.3); overflow: hidden; margin: 0.6rem 0; }
  .gate-fill { height: 100%; background: #fff; transition: width 0.15s linear; }
  .gate-ribbon {
    position: fixed; top: 0; left: 0; right: 0; z-index: 9000;
    background: #c62828; color: #fff; font-size: 0.85rem;
    display: flex; align-items: center; justify-content: center; gap: 0.8rem;
    height: 34px; padding: 0 0.8rem; white-space: nowrap; overflow: hidden;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.25);
  }
  .gate-ribbon > span:first-child { overflow: hidden; text-overflow: ellipsis; }
  .gate-ribbon button {
    background: #fff; color: #c62828; border: none; font-weight: 700;
    padding: 0.25rem 0.8rem; border-radius: 6px; font-size: 0.8rem;
  }
  .gate-ribbon-progress, .gate-ribbon-note { font-weight: 600; }
</style>
