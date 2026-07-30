<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
  import { api } from "$lib/api";
  import { currentUser } from "$lib/stores/auth";
  import { currentStore, allStores } from "$lib/stores/activeStore";
  import { currentServer, currentPath, allServers } from "$lib/stores/activeServer";
  import { logout } from "$lib/stores/auth";
  import { closeAllTabs, closeTab, activeTabId } from "$lib/stores/tabs";
  import { loadAndApplyTheme } from "$lib/theme";
  import Login from "$lib/components/Login.svelte";
  import StorePicker from "$lib/components/StorePicker.svelte";
  import ServerPicker from "$lib/components/ServerPicker.svelte";
  import Ribbon from "$lib/components/Ribbon.svelte";
  import TabBar from "$lib/components/TabBar.svelte";
  import Workspace from "$lib/components/Workspace.svelte";

  let ready = $state(false);
  let showServerPicker = $state(false);
  let showPicker = $state(false);

  async function loadLocalStores() {
    try {
      const [stores, active] = await Promise.all([api.listStores(), api.currentStore()]);
      allStores.set(stores);
      currentStore.set(active);
      showPicker = true;
    } catch {
      // Command lama / gagal baca registry: anggap mode single-store, lanjut ke login.
    }
  }

  onMount(async () => {
    document.getElementById("boot-splash")?.remove();
    loadAndApplyTheme();
    try {
      const [servers, active] = await Promise.all([api.listServers(), api.currentServer()]);
      allServers.set(servers);
      currentServer.set(active.server);
      currentPath.set(active.path);
      if (active.server.kind === "remote") {
        // Server Pusat: lewati pemilihan toko lokal, langsung ke Login yang
        // login ke Server Pusat lewat proxy command.
        currentStore.set(null);
      } else {
        await loadLocalStores();
      }
    } catch {
      // Command lama: anggap mode Server Lokal + single-store, lanjut ke login.
      await loadLocalStores();
    } finally {
      ready = true;
    }
  });

  // Esc menutup popup yang sedang terbuka dulu (kalau ada) — semua popup di app
  // konsisten pakai <div class="modal-backdrop" onclick={...}>, jadi cukup klik
  // backdrop paling akhir di DOM (= popup paling atas). Kalau tidak ada popup,
  // baru tutup tab aktif (dengan konfirmasi otomatis kalau tab "dirty", lihat
  // closeTab()/tabGuard.ts).
  function onEscapeCloseTab(e: KeyboardEvent) {
    if (e.key !== "Escape") return;
    const backdrops = document.querySelectorAll<HTMLElement>(".modal-backdrop");
    if (backdrops.length > 0) {
      backdrops[backdrops.length - 1].click();
      return;
    }
    const id = get(activeTabId);
    if (id) closeTab(id);
  }
  onMount(() => window.addEventListener("keydown", onEscapeCloseTab));
  onDestroy(() => window.removeEventListener("keydown", onEscapeCloseTab));

  function onServerChosen(active: import("$lib/types").ActiveServer) {
    closeAllTabs();
    logout();
    showServerPicker = false;
    if (active.server.kind === "local") {
      showPicker = false;
      loadLocalStores();
    } else {
      currentStore.set(null);
      showPicker = false;
    }
  }
</script>

{#if !ready}
  <div class="boot-loading">Memuat…</div>
{:else if showServerPicker}
  <ServerPicker onChosen={onServerChosen} />
{:else if showPicker}
  <StorePicker onChosen={() => (showPicker = false)} />
{:else if !$currentUser}
  <Login
    onChangeStore={$currentServer?.kind === "local" && $allStores.length > 1 ? () => (showPicker = true) : undefined}
    onChangeServer={() => (showServerPicker = true)}
  />
{:else}
  <div class="app">
    <Ribbon />
    <TabBar />
    <Workspace />
  </div>
{/if}

<style>
  .boot-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    color: var(--text-dim);
  }
</style>
