<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { currentUser } from "$lib/stores/auth";
  import { currentStore, allStores } from "$lib/stores/activeStore";
  import { currentServer, allServers } from "$lib/stores/activeServer";
  import { logout } from "$lib/stores/auth";
  import { closeAllTabs } from "$lib/stores/tabs";
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
      currentServer.set(active);
      if (active.kind === "remote") {
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

  function onServerChosen(info: import("$lib/types").ServerInfo) {
    closeAllTabs();
    logout();
    showServerPicker = false;
    if (info.kind === "local") {
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
