<script lang="ts">
  // Shell utama mobile — padanan desktop +page.svelte:
  // gate boot: (belum pairing → ServerPicker) → (belum login → Login) →
  // shell bottom-nav 4 tab dengan stack layar per tab (nav.ts).
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { currentUser, logout, can } from "$lib/stores/auth";
  import { currentServer, allServers } from "$lib/stores/activeServer";
  import { loadAndApplyTheme } from "$lib/theme";
  import {
    activeTab,
    stacks,
    switchTab,
    pop,
    resetNav,
    initNavHistory,
    type TabKey,
  } from "$lib/nav";
  import { serverReachable, checking, checkConnection, startConnectionWatch } from "$lib/connection";
  import { SCREENS } from "$lib/screens/registry";
  import ServerPicker from "$lib/components/ServerPicker.svelte";
  import Login from "$lib/components/Login.svelte";
  import KasirScreen from "$lib/screens/KasirScreen.svelte";
  import ProdukScreen from "$lib/screens/ProdukScreen.svelte";
  import LaporanScreen from "$lib/screens/LaporanScreen.svelte";
  import MenuScreen from "$lib/screens/MenuScreen.svelte";

  let ready = $state(false);
  let showServerPicker = $state(false);
  let themeKey = $state("baby-blue");

  const TABS: { key: TabKey; label: string; ico: string; perm: string | null }[] = [
    { key: "kasir", label: "Kasir", ico: "🧾", perm: "penjualan" },
    { key: "produk", label: "Produk", ico: "📦", perm: "master" },
    { key: "laporan", label: "Laporan", ico: "📊", perm: "laporan" },
    { key: "menu", label: "Menu", ico: "☰", perm: null },
  ];

  const visibleTabs = $derived(
    $currentUser ? TABS.filter((t) => !t.perm || can(t.perm as never)) : TABS,
  );
  const stack = $derived($stacks[$activeTab]);
  const topEntry = $derived(stack.length > 0 ? stack[stack.length - 1] : null);

  const TAB_TITLES: Record<TabKey, string> = {
    kasir: "Kasir",
    produk: "Data Barang",
    laporan: "Laporan",
    menu: "Menu",
  };

  onMount(() => {
    document.getElementById("boot-splash")?.remove();
    const cleanupNav = initNavHistory();
    const cleanupConn = startConnectionWatch();

    (async () => {
      themeKey = await loadAndApplyTheme();
      try {
        const [servers, active] = await Promise.all([api.listServers(), api.currentServer()]);
        allServers.set(servers);
        currentServer.set(active);
        if (!active) showServerPicker = true;
      } catch {
        showServerPicker = true;
      } finally {
        ready = true;
      }
    })();

    return () => {
      cleanupNav();
      cleanupConn();
    };
  });

  function onServerChosen() {
    resetNav();
    logout();
    showServerPicker = false;
    checkConnection();
  }
</script>

{#if !ready}
  <div class="boot-loading">Memuat…</div>
{:else if showServerPicker || !$currentServer}
  <ServerPicker onChosen={onServerChosen} />
{:else if !$currentUser}
  <Login onChangeServer={() => (showServerPicker = true)} />
{:else}
  <div class="app">
    {#if !$serverReachable}
      <div class="conn-banner">
        <span style="flex:1;">⚠ Tidak terhubung ke Server Pusat</span>
        <button disabled={$checking} onclick={checkConnection}>
          {$checking ? "Memeriksa…" : "Coba lagi"}
        </button>
      </div>
    {/if}

    <header class="screen-head">
      {#if topEntry}
        <button class="back-btn" onclick={pop} aria-label="Kembali">‹</button>
        <div class="head-title">{topEntry.title}</div>
      {:else}
        <div class="head-title">{TAB_TITLES[$activeTab]}</div>
        <div class="head-sub">{$currentUser.name}</div>
      {/if}
    </header>

    <main class="screen-body">
      {#if topEntry}
        {@const Sub = SCREENS[topEntry.key]}
        {#if Sub}
          <Sub {...topEntry.props ?? {}} />
        {:else}
          <p class="text-dim">Layar "{topEntry.key}" belum tersedia.</p>
        {/if}
      {:else if $activeTab === "kasir"}
        <KasirScreen />
      {:else if $activeTab === "produk"}
        <ProdukScreen />
      {:else if $activeTab === "laporan"}
        <LaporanScreen />
      {:else}
        <MenuScreen
          {themeKey}
          onThemeChange={(k) => (themeKey = k)}
          onChangeServer={() => {
            resetNav();
            showServerPicker = true;
          }}
        />
      {/if}
    </main>

    <nav class="bottom-nav">
      {#each visibleTabs as t (t.key)}
        <button class:active={$activeTab === t.key} onclick={() => switchTab(t.key)}>
          <span class="ico">{t.ico}</span>
          {t.label}
        </button>
      {/each}
    </nav>
  </div>
{/if}

<style>
  .boot-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100dvh;
    color: var(--text-dim);
  }
</style>
