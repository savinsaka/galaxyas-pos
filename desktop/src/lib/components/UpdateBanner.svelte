<script lang="ts">
  import { onMount } from "svelte";
  import { updateState, checkForUpdate, installUpdate, dismissUpdateBanner } from "$lib/updater";

  onMount(() => {
    checkForUpdate();
  });
</script>

{#if $updateState.status === "available"}
  <div class="update-banner">
    <span>🔔 Versi <b>{$updateState.version}</b> tersedia.</span>
    <div class="row" style="gap:0.4rem;">
      <button class="btn-primary" onclick={installUpdate}>Update Sekarang</button>
      <button class="btn-ghost" onclick={dismissUpdateBanner}>Nanti</button>
    </div>
  </div>
{:else if $updateState.status === "downloading"}
  <div class="update-banner">
    <span>⬇️ Mengunduh update {$updateState.version}… {$updateState.progress}%</span>
    <div class="update-progress-bar">
      <div class="update-progress-fill" style="width:{$updateState.progress}%"></div>
    </div>
  </div>
{:else if $updateState.status === "ready"}
  <div class="update-banner">
    <span>✅ Update siap. Aplikasi akan restart…</span>
  </div>
{:else if $updateState.status === "error"}
  <div class="update-banner update-banner-error">
    <span>⚠️ Update gagal: {$updateState.message}</span>
    <button class="btn-ghost" onclick={dismissUpdateBanner}>Tutup</button>
  </div>
{/if}

<style>
  .update-banner {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 9999;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    padding: 0.5rem 1rem;
    background: var(--primary);
    color: #fff;
    font-size: 0.85rem;
  }
  .update-banner-error {
    background: var(--danger);
  }
  .update-banner :global(button) {
    color: #fff;
    border-color: rgba(255, 255, 255, 0.5);
  }
  .update-progress-bar {
    width: 160px;
    height: 6px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.35);
    overflow: hidden;
  }
  .update-progress-fill {
    height: 100%;
    background: #fff;
    transition: width 0.15s linear;
  }
</style>
