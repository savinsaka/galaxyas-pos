<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import type { Brand } from "$lib/types";

  let { value = $bindable("") }: { value?: string | null } = $props();

  let brands = $state<Brand[]>([]);
  let adding = $state(false);
  let newName = $state("");

  async function load() {
    try {
      brands = await api.listBrands();
    } catch (e) {
      toastError(e);
    }
  }
  onMount(load);

  async function addBrand() {
    const n = newName.trim();
    if (!n) return;
    try {
      const b = await api.saveBrand({ name: n });
      await load();
      value = b.name;
      newName = "";
      adding = false;
      showToast("Merek ditambahkan.", "success");
    } catch (e) {
      toastError(e);
    }
  }
</script>

<div class="row" style="gap:0.4rem;">
  <select bind:value>
    <option value="">— pilih merek —</option>
    {#if value && !brands.some((b) => b.name === value)}
      <option value={value}>{value}</option>
    {/if}
    {#each brands as b (b.id)}
      <option value={b.name}>{b.name}</option>
    {/each}
  </select>
  <button type="button" onclick={() => (adding = true)} title="Tambah merek baru" style="padding:0.5rem 0.7rem;">＋</button>
</div>

{#if adding}
  <div class="modal-backdrop" onclick={() => (adding = false)} role="presentation">
    <div class="modal" style="width:360px;" onclick={(e) => e.stopPropagation()} role="presentation">
      <h2>Tambah Merek</h2>
      <label>Nama Merek</label>
      <input bind:value={newName} onkeydown={(e) => e.key === "Enter" && addBrand()} />
      <div class="row" style="justify-content:flex-end; margin-top:1rem;">
        <button type="button" onclick={() => (adding = false)}>Batal</button>
        <button type="button" class="btn-primary" onclick={addBrand}>Simpan</button>
      </div>
    </div>
  </div>
{/if}
