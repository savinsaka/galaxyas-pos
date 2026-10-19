<script lang="ts">
  import { onMount, tick } from "svelte";
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import { confirmDialog } from "$lib/dialog";
  import { syncAccount, refreshSyncAccount } from "$lib/stores/syncAccount";

  // Login anggota ke server SSoT (jjapps.net). Tidak ada isian URL: akun dibuat
  // admin di jjapps.net/admin → Kelola Anggota, username = ID Toko.
  let username = $state("");
  let password = $state("");
  let busy = $state(false);
  let loaded = $state(false);
  let userEl = $state<HTMLInputElement | null>(null);
  let passEl = $state<HTMLInputElement | null>(null);

  // Server Pengiriman (bridge menu "Kirim ke Toko") — beda server & auth dari
  // login di atas; baru ditampilkan setelah login.
  const mobileFields = [
    { key: "mobile_server_url", label: "URL Server Bridge", hint: "https://app.jjapps.net", secret: false },
    { key: "ship_warehouse_key", label: "Warehouse Key", hint: "kunci mesin dari Bos", secret: true },
  ];
  let mobile = $state<Record<string, string>>({});
  let savingMobile = $state(false);

  async function loadMobile() {
    try {
      const s = await api.getSettings();
      mobile = Object.fromEntries(mobileFields.map((f) => [f.key, s[f.key] ?? ""]));
    } catch (e) {
      toastError(e);
    }
  }

  onMount(async () => {
    const acc = await refreshSyncAccount();
    username = acc?.username ?? "";
    loaded = true;
    if (acc?.logged_in) {
      await loadMobile();
    } else {
      await tick();
      (username ? passEl : userEl)?.focus();
    }
  });

  async function login(e: Event) {
    e.preventDefault();
    if (!username.trim() || !password) {
      showToast("ID Toko dan password wajib diisi.", "error");
      return;
    }
    busy = true;
    try {
      const acc = await api.syncLogin(username.trim(), password);
      syncAccount.set(acc);
      password = "";
      showToast(`Masuk sebagai ${acc.username}.`, "success");
      await loadMobile();
    } catch (err) {
      toastError(err);
      password = "";
      await tick();
      passEl?.focus();
    } finally {
      busy = false;
    }
  }

  async function logout() {
    const ok = await confirmDialog(
      "Sync In / Out dan SSoT di PC ini tidak bisa dipakai sampai login lagi.",
      { title: "Keluar dari akun server?", okText: "Ya, Keluar", danger: true, icon: "🔑" },
    );
    if (!ok) return;
    busy = true;
    try {
      await api.syncLogout();
      await refreshSyncAccount();
      showToast("Sudah keluar dari akun server.", "success");
      await tick();
      passEl?.focus();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  async function saveMobile() {
    savingMobile = true;
    try {
      for (const f of mobileFields) await api.updateSetting(f.key, String(mobile[f.key] ?? ""));
      showToast("Pengaturan pengiriman tersimpan.", "success");
    } catch (e) {
      toastError(e);
    } finally {
      savingMobile = false;
    }
  }
</script>

<div class="page-head"><h1>Akun Server</h1></div>

{#if !loaded}
  <p class="text-dim">Memuat…</p>
{:else if !$syncAccount?.logged_in}
  <div class="card login-card">
    <div class="login-icon">🔑</div>
    <h2>Login Server Sinkronisasi</h2>
    <p class="text-dim login-hint">
      Masuk dengan <b>ID Toko</b> dan password dari admin. Cukup sekali per PC — tetap masuk walau
      aplikasi diupdate. Sync In / Out &amp; SSoT baru bisa dipakai setelah login.
    </p>
    <form onsubmit={login}>
      <label for="sa-user">ID Toko</label>
      <input id="sa-user" class="mono" bind:this={userEl} bind:value={username} placeholder="gudang-001" autocomplete="username" disabled={busy} />
      <label for="sa-pass" style="margin-top:0.7rem;">Password</label>
      <input id="sa-pass" type="password" bind:this={passEl} bind:value={password} autocomplete="current-password" disabled={busy} />
      <button class="btn-primary login-btn" type="submit" disabled={busy}>{busy ? "Memeriksa…" : "Masuk"}</button>
    </form>
    <p class="text-dim login-foot">Belum punya akun? Minta admin membuatkan di panel Kelola Anggota.</p>
  </div>
{:else}
  <div class="card" style="max-width:520px;">
    <h2>Server Sinkronisasi</h2>
    <div class="acc-row">
      <div>
        <div class="text-dim acc-label">ID Toko</div>
        <div class="mono acc-id">{$syncAccount.username}</div>
        {#if $syncAccount.name}<div class="text-dim">{$syncAccount.name}</div>{/if}
      </div>
      <span class="acc-badge">● Terhubung</span>
    </div>
    <div class="row" style="justify-content:flex-end; margin-top:0.8rem;">
      <button class="btn-ghost" disabled={busy} onclick={logout}>Keluar</button>
    </div>
  </div>

  <div class="card" style="max-width:520px; margin-top:0.8rem;">
    <h2>Server Pengiriman</h2>
    <p class="text-dim" style="margin-top:0; font-size:0.83rem;">
      Untuk menu <b>Kirim ke Toko</b>: isi URL server bridge + <b>Warehouse Key</b>
      (kunci mesin dari Bos). Daftar toko tujuan lalu muncul otomatis.
    </p>
    {#each mobileFields as f}
      <div style="margin-bottom:0.9rem;">
        <label for="sa-{f.key}">{f.label}</label>
        <input id="sa-{f.key}" type={f.secret ? "password" : "text"} bind:value={mobile[f.key]} placeholder={f.hint} autocomplete="off" />
      </div>
    {/each}
    <div class="row" style="justify-content:flex-end;">
      <button class="btn-primary" disabled={savingMobile} onclick={saveMobile}>Simpan Pengaturan Pengiriman</button>
    </div>
  </div>
{/if}

<style>
  .login-card { max-width: 400px; text-align: center; }
  .login-card form { text-align: left; }
  .login-icon { font-size: 2.2rem; }
  .login-hint { font-size: 0.83rem; margin: 0.3rem 0 1rem; }
  .login-btn { width: 100%; margin-top: 1rem; padding: 0.7rem; }
  .login-foot { font-size: 0.75rem; margin: 0.8rem 0 0; }
  .acc-row { display: flex; align-items: center; justify-content: space-between; gap: 1rem; }
  .acc-label { font-size: 0.75rem; }
  .acc-id { font-size: 1.2rem; font-weight: 700; }
  .acc-badge { color: var(--success); font-weight: 650; font-size: 0.85rem; white-space: nowrap; }
</style>
