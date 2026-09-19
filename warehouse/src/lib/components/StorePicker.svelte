<script lang="ts">
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import { currentStore, allStores } from "$lib/stores/activeStore";
  import { logout } from "$lib/stores/auth";
  import { closeAllTabs } from "$lib/stores/tabs";

  let { onChosen }: { onChosen: () => void } = $props();

  let creating = $state(false);
  let newName = $state("");
  let busy = $state(false);

  async function choose(id: string) {
    busy = true;
    try {
      const info = await api.selectStore(id);
      currentStore.set(info);
      closeAllTabs();
      logout();
      onChosen();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  async function createAndChoose() {
    const name = newName.trim();
    if (!name) return showToast("Nama toko wajib diisi.", "error");
    busy = true;
    try {
      const info = await api.createStore(name);
      allStores.update((list) => [...list, info]);
      newName = "";
      creating = false;
      await choose(info.id);
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="login-screen">
  <div class="login-card" style="max-width:420px;">
    <div class="login-logo">GALAX<b>YAS</b> POS</div>
    <div class="login-sub">Pilih Toko</div>

    <div class="store-list">
      {#each $allStores as s (s.id)}
        <button class="store-row" disabled={busy} onclick={() => choose(s.id)}>
          <span>🏪</span>
          <span style="flex:1;">{s.name}</span>
          {#if $currentStore?.id === s.id}<span class="text-dim" style="font-size:0.75rem;">aktif</span>{/if}
        </button>
      {/each}
    </div>

    {#if $allStores.length < 3}
      {#if creating}
        <div class="row" style="margin-top:0.8rem;">
          <input bind:value={newName} placeholder="Nama toko baru" disabled={busy} />
          <button class="btn-primary" disabled={busy} onclick={createAndChoose}>Buat</button>
          <button class="btn-ghost" disabled={busy} onclick={() => (creating = false)}>Batal</button>
        </div>
      {:else}
        <button style="margin-top:0.8rem; width:100%;" disabled={busy} onclick={() => (creating = true)}>
          + Toko Baru
        </button>
      {/if}
    {:else}
      <p class="text-dim" style="font-size:0.76rem; margin-top:0.8rem; text-align:center;">
        Maksimum 3 toko sudah tercapai.
      </p>
    {/if}
  </div>
</div>

<style>
  .store-list { display: flex; flex-direction: column; gap: 0.5rem; margin-top: 1rem; }
  .store-row { display: flex; align-items: center; gap: 0.6rem; width: 100%; text-align: left; padding: 0.7rem 0.9rem; }
</style>
