<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatDateTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import type { SyncResult } from "$lib/types";

  let settings = $state<Record<string, string>>({});
  let syncing = $state(false);
  let lastResult = $state<SyncResult | null>(null);

  async function load() {
    try {
      settings = await api.getSettings();
    } catch (e) {
      toastError(e);
    }
  }
  onMount(load);

  async function saveField(key: string) {
    try {
      await api.updateSetting(key, settings[key] ?? "");
      showToast("Tersimpan.", "success");
    } catch (e) {
      toastError(e);
    }
  }

  async function run(kind: "push" | "pull" | "all") {
    syncing = true;
    lastResult = null;
    try {
      const fn = kind === "push" ? api.syncPush : kind === "pull" ? api.syncPull : api.syncAll;
      lastResult = await fn();
      showToast(lastResult.message, "success", 5000);
      await load();
    } catch (e) {
      toastError(e);
    } finally {
      syncing = false;
    }
  }
</script>

<div class="page-head"><h1>Sinkronisasi Master Data</h1></div>

<div class="grid-2" style="align-items:start;">
  <div class="card">
    <h2>Koneksi Server</h2>
    <div style="margin-bottom:0.8rem;">
      <label>ID Toko</label>
      <div class="row"><input bind:value={settings.store_id} /><button onclick={() => saveField("store_id")}>Simpan</button></div>
    </div>
    <div>
      <label>URL Server Sync</label>
      <div class="row"><input bind:value={settings.server_url} placeholder="http://localhost:8000" /><button onclick={() => saveField("server_url")}>Simpan</button></div>
    </div>
    <div class="text-dim" style="margin-top:0.8rem; font-size:0.78rem;">
      Pull terakhir: {settings.last_pull_at ? formatDateTime(settings.last_pull_at) : "belum pernah"}
    </div>
  </div>

  <div class="card">
    <h2>Aksi Sinkronisasi</h2>
    <p class="text-dim">
      Manual · Delta Sync (<code>updated_at</code>) · Last Write Wins. Hanya master data;
      stok &amp; transaksi tetap lokal.
    </p>
    <div style="display:flex; flex-direction:column; gap:0.5rem; margin:1rem 0;">
      <button class="btn-primary" style="padding:0.8rem;" disabled={syncing} onclick={() => run("push")}>⬆ Kirim Data ke Server (Sync Out)</button>
      <button class="btn-primary" style="padding:0.8rem;" disabled={syncing} onclick={() => run("pull")}>⬇ Ambil Update dari Server (Sync In)</button>
      <button class="btn-success" style="padding:0.8rem;" disabled={syncing} onclick={() => run("all")}>⇅ Sync Semua</button>
    </div>
    {#if syncing}<p class="text-dim">Menyinkronkan…</p>{/if}
    {#if lastResult}
      <div class="card" style="background:var(--baby-blue-bg);">
        <div class="row" style="gap:1.5rem;">
          <div><div class="text-dim">Dikirim</div><strong class="mono">{lastResult.pushed}</strong></div>
          <div><div class="text-dim">Diterima</div><strong class="mono">{lastResult.pulled}</strong></div>
          <div><div class="text-dim">Dilewati</div><strong class="mono">{lastResult.skipped}</strong></div>
        </div>
        <div class="text-dim" style="margin-top:0.5rem;">{lastResult.message}</div>
        {#if lastResult.log.length > 0}
          <div class="sync-log-head">📋 Detail barang ({lastResult.log.length})</div>
          <div class="sync-log">
            {#each lastResult.log as l}
              <div class="log-row log-{l.action === 'Diterapkan' ? 'applied' : l.action === 'Dikirim' ? 'sent' : 'skipped'}">
                <span>{l.name}</span>
                <span class="log-action">{l.action}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .sync-log-head { font-weight: 650; font-size: 0.85rem; margin-top: 0.8rem; margin-bottom: 0.3rem; }
  .sync-log { max-height: 240px; overflow-y: auto; border: 1px solid var(--border); border-radius: 8px; background: var(--white); }
  .log-row { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; padding: 0.3rem 0.6rem; font-size: 0.8rem; border-bottom: 1px solid var(--border); }
  .log-row:last-child { border-bottom: none; }
  .log-action { font-size: 0.7rem; font-weight: 700; padding: 0.1rem 0.4rem; border-radius: 999px; white-space: nowrap; }
  .log-applied .log-action { background: var(--success); color: #fff; }
  .log-sent .log-action { background: var(--primary); color: #fff; }
  .log-skipped .log-action { background: var(--border-strong); color: var(--text-dim); }
</style>
