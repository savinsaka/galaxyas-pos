<script lang="ts">
  import { get } from "svelte/store";
  import { RIBBON, type RibbonAction } from "$lib/ribbon";
  import { currentUser, logout } from "$lib/stores/auth";
  import { tabs, openTab, closeAllTabs } from "$lib/stores/tabs";
  import { isRemoteClient } from "$lib/stores/activeServer";

  let activeCatKey = $state("master");

  // Sync In/Out cuma relevan untuk PC yang punya SQLite lokal sendiri (Server
  // Lokal atau Server Pusat) — di PC client (konek ke Server Pusat) data
  // memang sudah live-shared, jadi sync manual disembunyikan agar tidak chaos.
  const visibleCats = $derived(
    RIBBON.filter(
      (c) =>
        $currentUser &&
        ($currentUser.role === "admin" || $currentUser.permissions.includes(c.perm)),
    ).map((c) => ({
      ...c,
      groups: c.groups
        .map((g) => ({
          ...g,
          actions: g.actions.filter(
          (a) => !($isRemoteClient && ["sync", "pengaturan-server", "pengaturan-lanjutan"].includes(a.key)),
        ),
        }))
        .filter((g) => g.actions.length > 0),
    })),
  );

  $effect(() => {
    if (visibleCats.length && !visibleCats.some((c) => c.key === activeCatKey)) {
      activeCatKey = visibleCats[0].key;
    }
  });

  const activeCat = $derived(visibleCats.find((c) => c.key === activeCatKey) ?? null);

  function runAction(action: RibbonAction) {
    let title = action.title;
    if (action.singleton === false) {
      const n = get(tabs).filter((t) => t.viewKey === action.viewKey).length + 1;
      title = `${action.title} ${n}`;
    }
    openTab({
      viewKey: action.viewKey,
      title,
      icon: action.icon,
      singleton: action.singleton !== false,
      props: action.props,
    });
  }

  function doLogout() {
    closeAllTabs();
    logout();
  }
</script>

<div class="ribbon">
  <div class="ribbon-top">
    <div class="ribbon-brand">GALAX<b>YAS</b></div>
    {#each visibleCats as cat}
      <button
        class="ribbon-cat"
        class:active={cat.key === activeCatKey}
        onclick={() => (activeCatKey = cat.key)}
      >
        {cat.label}
      </button>
    {/each}
    <div class="ribbon-user">
      <span>👤 {$currentUser?.name} · {$currentUser?.role}</span>
      <button onclick={doLogout}>Keluar</button>
    </div>
  </div>

  {#if activeCat}
    <div class="ribbon-panel">
      {#each activeCat.groups as group}
        <div class="ribbon-group">
          <div class="ribbon-group-actions">
            {#each group.actions as action}
              <button class="ribbon-btn" onclick={() => runAction(action)}>
                <span class="ico">{action.icon}</span>
                <span class="lbl">{action.label}</span>
              </button>
            {/each}
          </div>
          <div class="ribbon-group-label">{group.label}</div>
        </div>
      {/each}
    </div>
  {/if}
</div>
