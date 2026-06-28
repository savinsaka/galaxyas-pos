<script lang="ts">
  import { tabs, activeTabId } from "$lib/stores/tabs";
  import { VIEW_REGISTRY } from "$lib/views/registry";
</script>

<div class="workspace">
  {#if $tabs.length === 0}
    <div class="empty-ws">
      <div class="big">🗂️</div>
      <div>Belum ada tab terbuka.</div>
      <div class="text-dim">Klik salah satu menu di ribbon atas untuk membuka tab kerja.</div>
    </div>
  {:else}
    {#each $tabs as tab (tab.id)}
      {@const Comp = VIEW_REGISTRY[tab.viewKey]}
      <div style:display={tab.id === $activeTabId ? "block" : "none"}>
        {#if Comp}
          <Comp {...(tab.props ?? {})} />
        {:else}
          <p class="text-dim">View tidak ditemukan: {tab.viewKey}</p>
        {/if}
      </div>
    {/each}
  {/if}
</div>
