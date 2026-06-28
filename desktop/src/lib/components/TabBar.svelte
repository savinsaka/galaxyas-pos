<script lang="ts">
  import { tabs, activeTabId, setActive, closeTab } from "$lib/stores/tabs";
</script>

{#if $tabs.length}
  <div class="tabbar">
    {#each $tabs as tab (tab.id)}
      <div class="tab" class:active={tab.id === $activeTabId} onclick={() => setActive(tab.id)} role="tab" tabindex="0">
        <span>{tab.icon ?? "📄"}</span>
        <span class="tab-title">{tab.title}</span>
        <button class="tab-close" onclick={(e) => { e.stopPropagation(); closeTab(tab.id); }} title="Tutup">✕</button>
      </div>
    {/each}
  </div>
{/if}
