<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import type { Customer, CustomerInput } from "$lib/types";

  let customers = $state<Customer[]>([]);
  let search = $state("");

  function blank(): CustomerInput {
    return { id: null, name: "", phone: "", email: "", address: "", note: "", is_active: true };
  }
  let form = $state<CustomerInput>(blank());
  let busy = $state(false);

  async function load() {
    try {
      customers = await api.listCustomers(search, true);
    } catch (e) {
      toastError(e);
    }
  }
  onMount(load);

  function edit(c: Customer) {
    form = { id: c.id, name: c.name, phone: c.phone ?? "", email: c.email ?? "", address: c.address ?? "", note: c.note ?? "", is_active: c.is_active };
  }
  function resetForm() {
    form = blank();
  }

  async function save() {
    if (!form.phone?.trim()) return showToast("Nomor HP wajib diisi (dipakai untuk verifikasi di kasir).", "error");
    if (!form.name.trim()) return showToast("Nama pelanggan wajib diisi.", "error");
    busy = true;
    try {
      await api.saveCustomer(form);
      showToast("Pelanggan tersimpan.", "success");
      resetForm();
      await load();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  async function remove(c: Customer) {
    if (!confirm(`Nonaktifkan pelanggan "${c.name}"?`)) return;
    try {
      await api.deleteCustomer(c.id);
      showToast("Pelanggan dinonaktifkan.", "success");
      await load();
    } catch (e) {
      toastError(e);
    }
  }
</script>

<div class="page-head">
  <h1>Manajemen Pelanggan</h1>
  <input placeholder="Cari nama / telepon…" bind:value={search} oninput={load} style="width:260px;" />
</div>

<div class="grid-2" style="align-items:start;">
  <div class="card">
    <h2>{form.id ? "Edit Pelanggan" : "Pelanggan Baru"}</h2>
    <div class="grid-2">
      <div style="grid-column:1/3;">
        <label>Nomor HP *</label>
        <input bind:value={form.phone} placeholder="0812-xxxx-xxxx" />
        <p class="text-dim" style="font-size:0.76rem; margin:0.2rem 0 0;">
          Dipakai kasir untuk mencari pelanggan ini — wajib diisi & harus benar demi keamanan (mencegah pemakaian akun orang lain).
        </p>
      </div>
      <div style="grid-column:1/3;">
        <label>Nama *</label>
        <input bind:value={form.name} />
      </div>
      <div>
        <label>Email</label>
        <input bind:value={form.email} placeholder="opsional" />
      </div>
      <div>
        <label>Status</label>
        <label class="row" style="margin:0; gap:0.4rem; font-weight:400;">
          <input type="checkbox" bind:checked={form.is_active} style="width:auto;" /> Aktif
        </label>
      </div>
      <div style="grid-column:1/3;">
        <label>Alamat</label>
        <input bind:value={form.address} placeholder="opsional" />
      </div>
      <div style="grid-column:1/3;">
        <label>Catatan</label>
        <input bind:value={form.note} placeholder="opsional" />
      </div>
    </div>
    <div class="row" style="justify-content:flex-end; margin-top:1rem;">
      {#if form.id}<button onclick={resetForm}>Batal</button>{/if}
      <button class="btn-primary" disabled={busy} onclick={save}>💾 Simpan</button>
    </div>
  </div>

  <div class="card" style="padding:0; overflow:hidden;">
    <table>
      <thead>
        <tr><th>Nomor HP</th><th>Nama</th><th>Email</th><th>Status</th><th></th></tr>
      </thead>
      <tbody>
        {#each customers as c (c.id)}
          <tr>
            <td class="mono">{c.phone ?? "-"}</td>
            <td>{c.name}</td>
            <td>{c.email ?? "-"}</td>
            <td><span class="badge {c.is_active ? 'on' : 'off'}">{c.is_active ? "Aktif" : "Non-aktif"}</span></td>
            <td>
              <div class="row" style="gap:0.3rem;">
                <button class="btn-ghost" onclick={() => edit(c)}>Edit</button>
                <button class="btn-ghost" style="color:var(--danger);" onclick={() => remove(c)}>Nonaktifkan</button>
              </div>
            </td>
          </tr>
        {:else}
          <tr><td colspan="5" class="text-dim">Belum ada pelanggan.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
