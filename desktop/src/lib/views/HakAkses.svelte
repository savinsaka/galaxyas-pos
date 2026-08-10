<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import type { ModuleKey, User, UserInput } from "$lib/types";

  const MODULES: { k: ModuleKey; l: string }[] = [
    { k: "master", l: "Master Data" },
    { k: "penjualan", l: "Penjualan" },
    { k: "persediaan", l: "Persediaan" },
    { k: "laporan", l: "Laporan" },
    { k: "pengaturan", l: "Pengaturan" },
    { k: "cek-harga", l: "Cek Harga" },
  ];

  let users = $state<User[]>([]);
  let form = $state<UserInput | null>(null);

  async function load() {
    try {
      users = await api.listUsers();
    } catch (e) {
      toastError(e);
    }
  }
  onMount(load);

  function newUser() {
    form = { id: null, username: "", name: "", role: "kasir", permissions: ["penjualan"], pin: "" };
  }
  function edit(u: User) {
    form = { id: u.id, username: u.username, name: u.name, role: u.role, permissions: [...u.permissions], pin: "" };
  }
  function togglePerm(k: ModuleKey) {
    if (!form) return;
    form.permissions = form.permissions.includes(k)
      ? form.permissions.filter((p) => p !== k)
      : [...form.permissions, k];
  }
  async function save() {
    if (!form) return;
    if (!form.username.trim() || !form.name.trim()) return showToast("Username & nama wajib.", "error");
    if (!form.id && !form.pin) return showToast("PIN wajib untuk pengguna baru.", "error");
    try {
      await api.saveUser(form);
      showToast("Pengguna tersimpan.", "success");
      form = null;
      await load();
    } catch (e) {
      toastError(e);
    }
  }
  async function remove(u: User) {
    if (u.id === $currentUser?.id) return showToast("Tidak bisa menghapus diri sendiri.", "error");
    if (!confirm(`Hapus pengguna ${u.username}?`)) return;
    try {
      await api.deleteUser(u.id);
      await load();
    } catch (e) {
      toastError(e);
    }
  }
</script>

<div class="page-head">
  <h1>Hak Akses &amp; Pengguna</h1>
  <button class="btn-primary" onclick={newUser}>➕ Tambah Pengguna</button>
</div>

<div class="grid-2" style="align-items:start;">
  <div class="card" style="padding:0; overflow:hidden;">
    <table>
      <thead><tr><th>Username</th><th>Nama</th><th>Role</th><th>Akses</th><th></th></tr></thead>
      <tbody>
        {#each users as u (u.id)}
          <tr>
            <td class="mono">{u.username}</td>
            <td>{u.name}</td>
            <td><span class="badge">{u.role}</span></td>
            <td class="text-dim" style="font-size:0.78rem;">{u.role === "admin" ? "semua" : u.permissions.join(", ")}</td>
            <td>
              <div class="row">
                <button class="btn-ghost" onclick={() => edit(u)}>Edit</button>
                <button class="btn-ghost" style="color:var(--danger);" onclick={() => remove(u)}>Hapus</button>
              </div>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  {#if form}
    <div class="card">
      <h2>{form.id ? "Edit Pengguna" : "Pengguna Baru"}</h2>
      <div class="grid-2">
        <div><label>Username</label><input bind:value={form.username} /></div>
        <div><label>Nama</label><input bind:value={form.name} /></div>
        <div>
          <label>Role</label>
          <select bind:value={form.role}>
            <option value="kasir">kasir</option>
            <option value="supervisor">supervisor</option>
            <option value="admin">admin</option>
          </select>
        </div>
        <div><label>PIN {form.id ? "(kosongkan jika tetap)" : ""}</label><input type="password" bind:value={form.pin} /></div>
      </div>

      <label style="margin-top:0.8rem;">Hak Akses Modul</label>
      {#if form.role === "admin"}
        <p class="text-dim">Admin otomatis punya akses ke semua modul.</p>
      {:else}
        <div class="row" style="flex-wrap:wrap;">
          {#each MODULES as m}
            <label class="row" style="margin:0; font-weight:400; gap:0.3rem;">
              <input type="checkbox" checked={form.permissions.includes(m.k)} onchange={() => togglePerm(m.k)} style="width:auto;" /> {m.l}
            </label>
          {/each}
        </div>
      {/if}

      <div class="row" style="justify-content:flex-end; margin-top:1rem;">
        <button onclick={() => (form = null)}>Batal</button>
        <button class="btn-primary" onclick={save}>Simpan</button>
      </div>
    </div>
  {/if}
</div>
