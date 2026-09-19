<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatDateTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import type { Brand } from "$lib/types";

  let brands = $state<Brand[]>([]);
  let name = $state("");
  let editId = $state<string | null>(null);

  async function load() {
    try {
      brands = await api.listBrands();
    } catch (e) {
      toastError(e);
    }
  }
  onMount(load);

  async function save() {
    if (!name.trim()) return showToast("Nama merek wajib.", "error");
    try {
      await api.saveBrand({ id: editId, name });
      showToast("Merek tersimpan.", "success");
      name = "";
      editId = null;
      await load();
    } catch (e) {
      toastError(e);
    }
  }
  function edit(b: Brand) {
    editId = b.id;
    name = b.name;
  }
  async function remove(b: Brand) {
    if (!confirm(`Hapus merek "${b.name}"?`)) return;
    try {
      await api.deleteBrand(b.id);
      await load();
    } catch (e) {
      toastError(e);
    }
  }
</script>

<div class="page-head"><h1>Daftar Merek</h1></div>

<div class="grid-2" style="align-items:start;">
  <div class="card">
    <h2>{editId ? "Edit Merek" : "Merek Baru"}</h2>
    <label>Nama Merek</label>
    <input bind:value={name} onkeydown={(e) => e.key === "Enter" && save()} />
    <div class="row" style="justify-content:flex-end; margin-top:1rem;">
      {#if editId}<button onclick={() => { editId = null; name = ""; }}>Batal</button>{/if}
      <button class="btn-primary" onclick={save}>Simpan</button>
    </div>
  </div>

  <div class="card" style="padding:0; overflow:hidden;">
    <table>
      <thead><tr><th>Nama</th><th>Diperbarui</th><th></th></tr></thead>
      <tbody>
        {#each brands as b (b.id)}
          <tr>
            <td>{b.name}</td>
            <td class="text-dim">{formatDateTime(b.updated_at)}</td>
            <td>
              <div class="row">
                <button class="btn-ghost" onclick={() => edit(b)}>Edit</button>
                <button class="btn-ghost" style="color:var(--danger);" onclick={() => remove(b)}>Hapus</button>
              </div>
            </td>
          </tr>
        {:else}
          <tr><td colspan="3" class="text-dim">Belum ada merek.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
