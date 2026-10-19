<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatDateTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { confirmDialog } from "$lib/dialog";
  import type { Brand, SyncResult, SyncUndoInfo } from "$lib/types";
  import BrandMultiSelect from "$lib/components/BrandMultiSelect.svelte";
  import { syncAccount, refreshSyncAccount, openSyncAkun } from "$lib/stores/syncAccount";

  let settings = $state<Record<string, string>>({});
  let syncing = $state(false);
  const locked = $derived(!$syncAccount?.logged_in);
  let lastResult = $state<SyncResult | null>(null);
  let undoInfo = $state<SyncUndoInfo | null>(null);

  // Filter merek Hard Push / Hard Pull. Mode "exclude" = semua merek kecuali
  // daftar pengecualian; "include" = hanya merek yang dipilih. Dua daftar
  // disimpan terpisah (setting JSON array) supaya ganti mode tidak menghapus isinya.
  type BrandMode = "exclude" | "include";
  let allBrands = $state<Brand[]>([]);
  let brandMode = $state<BrandMode>("exclude");
  let excludeBrands = $state<Set<string>>(new Set());
  let includeBrands = $state<Set<string>>(new Set());
  let filterLoaded = $state(false);
  let hardConfirm = $state<"hard_push" | "hard_pull" | null>(null);

  function parseBrandList(raw: string | undefined): Set<string> {
    try {
      return new Set(JSON.parse(raw || "[]"));
    } catch {
      return new Set();
    }
  }

  async function load() {
    try {
      undoInfo = await api.syncUndoInfo();
      settings = await api.getSettings();
      if (!filterLoaded) {
        brandMode = settings.hard_sync_brand_mode === "include" ? "include" : "exclude";
        excludeBrands = parseBrandList(settings.hard_sync_exclude_brands);
        includeBrands = parseBrandList(settings.hard_sync_include_brands);
        filterLoaded = true;
      }
    } catch (e) {
      toastError(e);
    }
  }
  onMount(async () => {
    refreshSyncAccount();
    await load();
    try {
      allBrands = await api.listBrands();
    } catch (e) {
      toastError(e);
    }
  });

  // Simpan otomatis tiap kali mode / daftar merek berubah.
  function autoSave(key: string, value: () => string) {
    let last: string | null = null;
    $effect(() => {
      const v = value();
      if (!filterLoaded) return;
      if (last === null) {
        last = v;
        return;
      }
      if (v === last) return;
      last = v;
      api.updateSetting(key, v).catch(toastError);
    });
  }
  autoSave("hard_sync_brand_mode", () => brandMode);
  autoSave("hard_sync_exclude_brands", () => JSON.stringify([...excludeBrands]));
  autoSave("hard_sync_include_brands", () => JSON.stringify([...includeBrands]));

  const includeEmpty = $derived(brandMode === "include" && includeBrands.size === 0);

  type Kind = "push" | "pull" | "all" | "hard_push" | "hard_pull";
  const runners: Record<Kind, () => Promise<SyncResult>> = {
    push: api.syncPush,
    pull: api.syncPull,
    all: api.syncAll,
    hard_push: api.syncHardPush,
    hard_pull: api.syncHardPull,
  };

  const kindLabel: Record<Kind, string> = {
    push: "Sync Out",
    pull: "Sync In",
    all: "Sync Semua",
    hard_push: "Hard Push",
    hard_pull: "Hard Pull",
  };

  function undoScope(u: SyncUndoInfo): string {
    const parts: string[] = [];
    if (u.local_count > 0) parts.push(`${u.local_count} barang di PC ini`);
    if (u.server_count > 0) parts.push(`${u.server_count} barang di server (SSoT)`);
    return parts.join(" dan ");
  }

  async function undo() {
    const u = undoInfo;
    if (!u) return;
    const ok = await confirmDialog(
      `${kindLabel[u.kind]} tanggal ${formatDateTime(u.created_at)} akan dibatalkan: ` +
        `${undoScope(u)} dikembalikan ke data sebelum sync. ` +
        `Barang yang sudah diubah lagi sesudah sync tidak disentuh. ` +
        `Undo hanya bisa sekali.`,
      { title: "Undo sync terakhir?", okText: "Ya, Undo", danger: true, icon: "↶" },
    );
    if (!ok) return;
    syncing = true;
    lastResult = null;
    try {
      lastResult = await api.syncUndo();
      showToast(lastResult.message, "success", 6000);
      await load();
    } catch (e) {
      toastError(e);
      await load();
    } finally {
      syncing = false;
    }
  }

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
    <h2>Akun Server</h2>
    {#if $syncAccount?.logged_in}
      <div class="text-dim" style="font-size:0.78rem;">Masuk sebagai</div>
      <div class="mono" style="font-size:1.1rem; font-weight:700;">{$syncAccount.username}</div>
      {#if $syncAccount.name}<div class="text-dim">{$syncAccount.name}</div>{/if}
    {:else}
      <p class="text-dim" style="margin-top:0;">
        Belum login. Sync In / Out terkunci sampai PC ini login dengan akun anggota.
      </p>
      <button class="btn-primary" onclick={openSyncAkun}>🔑 Login di Akun Server</button>
    {/if}
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
      <button class="btn-primary" style="padding:0.8rem;" disabled={syncing || locked} onclick={() => run("push")}>⬆ Kirim Data ke Server (Sync Out)</button>
      <button class="btn-primary" style="padding:0.8rem;" disabled={syncing || locked} onclick={() => run("pull")}>⬇ Ambil Update dari Server (Sync In)</button>
      <button class="btn-success" style="padding:0.8rem;" disabled={syncing || locked} onclick={() => run("all")}>⇅ Sync Semua</button>
    </div>

    <div class="undo-box">
      <button class="btn-ghost undo-btn" disabled={syncing || locked || !undoInfo} onclick={undo}>↶ Undo Sync Terakhir</button>
      <div class="text-dim undo-info">
        {#if undoInfo}
          {kindLabel[undoInfo.kind]} · {formatDateTime(undoInfo.created_at)} · {undoScope(undoInfo)}
        {:else}
          Belum ada sync yang bisa di-undo.
        {/if}
      </div>
    </div>

    <div class="hard-box">
      <div class="hard-head">⚠ Hard Sync</div>
      <p class="text-dim" style="margin:0 0 0.6rem; font-size:0.8rem;">
        Mengabaikan aturan "siapa terakhir diubah". Data yang sama <strong>ditimpa</strong>;
        data yang hanya ada di satu sisi <strong>tidak dihapus</strong>.
      </p>
      <div class="seg" role="group" aria-label="Filter merek">
        <button type="button" class:active={brandMode === "exclude"} onclick={() => (brandMode = "exclude")}>
          Semua merek, kecuali…
        </button>
        <button type="button" class:active={brandMode === "include"} onclick={() => (brandMode = "include")}>
          Hanya merek terpilih
        </button>
      </div>
      {#if brandMode === "exclude"}
        <label style="margin-top:0.6rem;">Pengecualian merek (tidak ikut Hard Push/Pull)</label>
        <BrandMultiSelect {allBrands} bind:selected={excludeBrands} placeholder="Tambah merek dikecualikan…" />
      {:else}
        <label style="margin-top:0.6rem;">Merek yang ikut Hard Push/Pull (merek lain tidak disentuh)</label>
        <BrandMultiSelect {allBrands} bind:selected={includeBrands} placeholder="Tambah merek yang ikut…" />
        {#if includeEmpty}
          <div class="text-dim" style="font-size:0.78rem; margin-top:0.3rem;">Pilih minimal satu merek.</div>
        {/if}
      {/if}
      <div class="row" style="gap:0.5rem; margin-top:0.8rem;">
        <button class="btn-danger" style="flex:1; padding:0.7rem;" disabled={syncing || locked || includeEmpty} onclick={() => (hardConfirm = "hard_push")}>⏫ Hard Push</button>
        <button class="btn-danger" style="flex:1; padding:0.7rem;" disabled={syncing || locked || includeEmpty} onclick={() => (hardConfirm = "hard_pull")}>⏬ Hard Pull</button>
      </div>
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
              <div class="log-row log-{l.action === 'Diterapkan' || l.action.startsWith('Dikembalikan') ? 'applied' : l.action === 'Dikirim' ? 'sent' : l.action === 'Dikecualikan' || l.action === 'Tidak dipilih' || l.action.startsWith('Dihapus') ? 'excluded' : 'skipped'}">
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
          <strong>{brandMode === "include" ? "Semua barang bermerek terpilih" : "SEMUA"}</strong> data barang di PC ini akan dikirim dan <strong>MENIMPA</strong> data yang sama
          di SSoT (server), walaupun data di server lebih baru. Toko lain akan ikut menerima perubahan ini
          saat Sync In.
        </p>
      {:else}
        <h2>Hard Pull dari SSoT?</h2>
        <p>
          <strong>{brandMode === "include" ? "Semua barang bermerek terpilih" : "SEMUA"}</strong> data barang di PC ini akan <strong>DITIMPA</strong> dengan data dari SSoT
          (server), termasuk perubahan lokal yang belum dikirim.
        </p>
      {/if}
      <p class="text-dim" style="font-size:0.82rem;">
        Barang yang hanya ada di satu sisi tidak dihapus. Stok &amp; transaksi tidak terpengaruh.
        {#if brandMode === "include"}
          <br />Hanya merek: <strong>{[...includeBrands].join(", ")}</strong>
        {:else if excludeBrands.size > 0}
          <br />Dikecualikan: <strong>{[...excludeBrands].join(", ")}</strong>
        {:else}
          <br />Tidak ada merek yang dikecualikan.
        {/if}
      </p>
      <p class="hard-warn">Bisa dibatalkan sekali lewat tombol "Undo Sync Terakhir".</p>
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
  .undo-box { display: flex; align-items: center; gap: 0.7rem; margin: -0.3rem 0 1rem; }
  .undo-btn { padding: 0.55rem 0.9rem; white-space: nowrap; }
  .undo-info { font-size: 0.78rem; }
  .hard-box { border: 1px dashed var(--danger); border-radius: 8px; padding: 0.8rem; margin-bottom: 1rem; }
  .hard-head { font-weight: 700; color: var(--danger); margin-bottom: 0.3rem; }
  .seg { display: flex; border: 1px solid var(--border-strong); border-radius: 8px; overflow: hidden; }
  .seg button { flex: 1; border: none; border-radius: 0; background: var(--white); color: var(--text-dim); padding: 0.45rem 0.5rem; font-size: 0.8rem; }
  .seg button + button { border-left: 1px solid var(--border-strong); }
  .seg button.active { background: var(--danger); color: #fff; font-weight: 650; }
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
