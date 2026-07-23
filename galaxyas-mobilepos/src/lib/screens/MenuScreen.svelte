<script lang="ts">
  // Tab Menu: info sesi + server, pilihan tema, logout / ganti server.
  // Entri Persediaan, Shift, Hak Akses, Pengaturan lengkap menyusul di
  // Phase 3-4 (di-push ke stack lewat nav.push).
  import { currentUser, logout } from "$lib/stores/auth";
  import { currentServer } from "$lib/stores/activeServer";
  import { THEMES, saveTheme } from "$lib/theme";
  import { toastError } from "$lib/toast";
  import { resetNav } from "$lib/nav";

  let {
    themeKey,
    onThemeChange,
    onChangeServer,
  }: {
    themeKey: string;
    onThemeChange: (key: string) => void;
    onChangeServer: () => void;
  } = $props();

  async function pickTheme(key: string) {
    try {
      await saveTheme(key);
      onThemeChange(key);
    } catch (e) {
      toastError(e);
    }
  }

  function doLogout() {
    resetNav();
    logout();
  }
</script>

<div class="menu">
  <div class="card sect">
    <label for="menu-sesi">Sesi</label>
    <div class="kv" id="menu-sesi">
      <span>👤 {$currentUser?.name}</span>
      <span class="badge">{$currentUser?.role}</span>
    </div>
    <div class="kv text-dim">
      <span>🖧 {$currentServer?.name}</span>
      <span class="mono">{$currentServer?.host}:{$currentServer?.port}</span>
    </div>
    <div class="row" style="margin-top:0.7rem;">
      <button style="flex:1;" onclick={doLogout}>Keluar</button>
      <button style="flex:1;" onclick={onChangeServer}>Ganti Server</button>
    </div>
  </div>

  <div class="card sect">
    <label for="menu-tema">Tema</label>
    <div class="themes" id="menu-tema">
      {#each THEMES as t (t.key)}
        <button class="theme-btn" class:active={themeKey === t.key} onclick={() => pickTheme(t.key)}>
          <span class="swatch" style="background:{t.swatch};"></span>
          {t.label}
        </button>
      {/each}
    </div>
  </div>

  <p class="text-dim ver">GALAXYAS Mobile POS</p>
</div>

<style>
  .menu {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }
  .sect label {
    margin-bottom: 0.5rem;
  }
  .kv {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0;
    font-size: 0.92rem;
  }
  .themes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }
  .theme-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.82rem;
    justify-content: flex-start;
  }
  .theme-btn.active {
    border-color: var(--primary);
    box-shadow: 0 0 0 2px rgba(74, 144, 217, 0.25);
  }
  .swatch {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1px solid var(--border-strong);
    flex-shrink: 0;
  }
  .ver {
    text-align: center;
    font-size: 0.75rem;
    margin-top: 0.4rem;
  }
</style>
