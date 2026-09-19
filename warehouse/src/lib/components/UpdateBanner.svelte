<script lang="ts">
  import { updateState, checkForUpdate, installUpdate, dismissUpdate } from "$lib/updater";
  import { currentUser } from "$lib/stores/auth";

  // Cek update baru dipicu SETELAH login (bukan di layar login) — sekali per
  // sesi login, supaya popup tidak mengganggu sebelum kasir sempat masuk.
  let checked = false;
  $effect(() => {
    if ($currentUser && !checked) {
      checked = true;
      checkForUpdate();
    } else if (!$currentUser) {
      checked = false;
    }
  });
</script>

{#if $currentUser && $updateState.status !== "idle"}
  <div class="modal-backdrop" onclick={() => $updateState.status === "available" && dismissUpdate()} role="presentation">
    <div class="modal update-modal" onclick={(e) => e.stopPropagation()} role="presentation">
      {#if $updateState.status === "available"}
        <div class="update-icon">🔔</div>
        <h2>Update Tersedia</h2>
        <p class="text-dim" style="margin:0.3rem 0 1rem;">Sudah ada update versi <b>{$updateState.version}</b>.</p>
        <div class="row" style="gap:0.5rem;">
          <button class="btn-ghost" style="flex:1;" onclick={dismissUpdate}>Nanti</button>
          <button class="btn-primary" style="flex:1;" onclick={installUpdate}>Update Sekarang</button>
        </div>
      {:else if $updateState.status === "downloading"}
        <div class="update-icon">⬇️</div>
        <h2>Mengunduh Update</h2>
        <p class="text-dim" style="margin:0.3rem 0 1rem;">Versi {$updateState.version} — {$updateState.progress}%</p>
        <div class="update-progress-bar">
          <div class="update-progress-fill" style="width:{$updateState.progress}%"></div>
        </div>
      {:else if $updateState.status === "ready"}
        <div class="update-icon">✅</div>
        <h2>Update Siap</h2>
        <p class="text-dim" style="margin:0.3rem 0 0;">Aplikasi akan restart otomatis…</p>
      {:else if $updateState.status === "error"}
        <div class="update-icon">⚠️</div>
        <h2>Update Gagal</h2>
        <p class="text-dim" style="margin:0.3rem 0 1rem;">{$updateState.message}</p>
        <button class="btn-primary" style="width:100%;" onclick={dismissUpdate}>Tutup</button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .update-modal { max-width: 360px; text-align: center; }
  .update-icon { font-size: 2.2rem; margin-bottom: 0.3rem; }
  .update-progress-bar {
    height: 8px;
    border-radius: 999px;
    background: var(--baby-blue-bg);
    overflow: hidden;
  }
  .update-progress-fill {
    height: 100%;
    background: var(--primary);
    transition: width 0.15s linear;
  }
</style>
