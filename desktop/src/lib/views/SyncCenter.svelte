<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatDateTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import type { Brand, SyncResult } from "$lib/types";
  import BrandMultiSelect from "$lib/components/BrandMultiSelect.svelte";

  let settings = $state<Record<string, string>>({});
  let syncing = $state(false);
  let lastResult = $state<SyncResult | null>(null);

  // Merek yang dikecualikan dari Hard Push / Hard Pull (setting JSON array).
  let allBrands = $state<Brand[]>([]);
  let excludeBrands = $state<Set<string>>(new Set());
  let excludeLoaded = false;
  let hardConfirm = $state<"hard_push" | "hard_pull" | null>(null);

  async function load() {
    try {
      settings = await api.getSettings();
      if (!excludeLoaded) {
        try {
          excludeBrands = new Set(JSON.parse(settings.hard_sync_exclude_brands || "[]"));
        } catch {
          excludeBrands = new Set();
        }
        excludeLoaded = true;
      }
    } catch (e) {
      toastError(e);
    }
  }
  onMount(async () => {
    await load();
    try {
      allBrands = await api.listBrands();
    } catch (e) {
      toastError(e);
    }
  });

  // Simpan otomatis tiap kali daftar pengecualian berubah.
  let lastSavedExclude = "";
  $effect(() => {
    const json = JSON.stringify([...excludeBrands]);
    if (!excludeLoaded) return;
    if (!lastSavedExclude) {
      lastSavedExclude = json;
      return;
    }
    if (json === lastSavedExclude) return;
    lastSavedExclude = json;
    api.updateSetting("hard_sync_exclude_brands", json).catch(toastError);
  });

  async function saveField(key: string) {
    try {
      await api.updateSetting(key, settings[key] ?? "");
      showToast("Tersimpan.", "success");
    } catch (e) {
      toastError(e);
    }
  }

  type Kind = "push" | "pull" | "all" | "hard_push" | "hard_pull";
  const runners: Record<Kind, () => Promise<SyncResult>> = {
    push: api.syncPush,
    pull: api.syncPull,
    all: api.syncAll,
    hard_push: api.syncHardPush,
    hard_pull: api.syncHardPull,
  };

  async function run(kind: Kind) {
    hardConfirm = null;
    syncing = true;
    lastResult = null;
    try {
      const fn = runners[kind];
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

    <div class="hard-box">
      <div class="hard-head">⚠ Hard Sync</div>
      <p class="text-dim" style="margin:0 0 0.6rem; font-size:0.8rem;">
        Mengabaikan aturan "siapa terakhir diubah". Data yang sama <strong>ditimpa</strong>;
        data yang hanya ada di satu sisi <strong>tidak dihapus</strong>.
      </p>
      <div class="row" style="gap:0.5rem;">
        <button class="btn-danger" style="flex:1; padding:0.7rem;" disabled={syncing} onclick={() => (hardConfirm = "hard_push")}>⏫ Hard Push</button>
        <button class="btn-danger" style="flex:1; padding:0.7rem;" disabled={syncing} onclick={() => (hardConfirm = "hard_pull")}>⏬ Hard Pull</button>
      </div>
      <label style="margin-top:0.8rem;">Pengecualian merek (tidak ikut Hard Push/Pull)</label>
      <BrandMultiSelect {allBrands} bind:selected={excludeBrands} placeholder="Tambah merek dikecualikan…" />
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
              <div class="log-row log-{l.action === 'Diterapkan' ? 'applied' : l.action === 'Dikirim' ? 'sent' : l.action === 'Dikecualikan' ? 'excluded' : 'skipped'}">
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

{#if hardConfirm}
  <div class="modal-backdrop" onclick={() => (hardConfirm = null)} role="presentation">
    <div class="modal hard-confirm" onclick={(e) => e.stopPropagation()} role="presentation">
      <div class="hard-icon">⚠️</div>
      {#if hardConfirm === "hard_push"}
        <h2>Hard Push ke SSoT?</h2>
        <p>
          <strong>SEMUA</strong> data barang di PC ini akan dikirim dan <strong>MENIMPA</strong> data yang sama
          di SSoT (server), walaupun data di server lebih baru. Toko lain akan ikut menerima perubahan ini
          saat Sync In.
        </p>
      {:else}
        <h2>Hard Pull dari SSoT?</h2>
        <p>
          <strong>SEMUA</strong> data barang di PC ini akan <strong>DITIMPA</strong> dengan data dari SSoT
          (server), termasuk perubahan lokal yang belum dikirim.
        </p>
      {/if}
      <p class="text-dim" style="font-size:0.82rem;">
        Barang yang hanya ada di satu sisi tidak dihapus. Stok &amp; transaksi tidak terpengaruh.
        {#if excludeBrands.size > 0}
          <br />Dikecualikan: <strong>{[...excludeBrands].join(", ")}</strong>
        {:else}
          <br />Tidak ada merek yang dikecualikan.
        {/if}
      </p>
      <p class="hard-warn">Tindakan ini tidak bisa dibatalkan.</p>
      <div class="row" style="gap:0.5rem;">
        <button class="btn-ghost" style="flex:1;" onclick={() => (hardConfirm = null)}>Batal</button>
        <button class="btn-danger" style="flex:1;" onclick={() => hardConfirm && run(hardConfirm)}>
          Ya, {hardConfirm === "hard_push" ? "Hard Push" : "Hard Pull"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .hard-box { border: 1px dashed var(--danger); border-radius: 8px; padding: 0.8rem; margin-bottom: 1rem; }
  .hard-head { font-weight: 700; color: var(--danger); margin-bottom: 0.3rem; }
  .hard-confirm { max-width: 460px; text-align: center; }
  .hard-confirm p { margin: 0.4rem 0 0.8rem; }
  .hard-icon { font-size: 2.2rem; }
  .hard-warn { color: var(--danger); font-weight: 700; }
  .log-excluded .log-action { background: var(--warning, #e0a800); color: #fff; }
  .sync-log-head { font-weight: 650; font-size: 0.85rem; margin-top: 0.8rem; margin-bottom: 0.3rem; }
  .sync-log { max-height: 240px; overflow-y: auto; border: 1px solid var(--border); border-radius: 8px; background: var(--white); }
  .log-row { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; padding: 0.3rem 0.6rem; font-size: 0.8rem; border-bottom: 1px solid var(--border); }
  .log-row:last-child { border-bottom: none; }
  .log-action { font-size: 0.7rem; font-weight: 700; padding: 0.1rem 0.4rem; border-radius: 999px; white-space: nowrap; }
  .log-applied .log-action { background: var(--success); color: #fff; }
  .log-sent .log-action { background: var(--primary); color: #fff; }
  .log-skipped .log-action { background: var(--border-strong); color: var(--text-dim); }
</style>
