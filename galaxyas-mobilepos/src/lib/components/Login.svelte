<script lang="ts">
  // Port mobile dari desktop Login.svelte — tanpa "Ganti Toko" (HP tidak
  // punya toko lokal), login di-proxy ke Server Pusat lewat rpc.
  import { api } from "$lib/api";
  import { setUser } from "$lib/stores/auth";
  import { showToast, toastError } from "$lib/toast";
  import { currentServer } from "$lib/stores/activeServer";

  let { onChangeServer }: { onChangeServer: () => void } = $props();

  let username = $state("admin");
  let pin = $state("");
  let busy = $state(false);

  async function submit(e: Event) {
    e.preventDefault();
    busy = true;
    try {
      const user = await api.login(username.trim(), pin);
      if (!user) {
        showToast("Username atau PIN salah.", "error");
        return;
      }
      setUser(user);
      showToast(`Selamat datang, ${user.name}.`, "success");
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="login-screen">
  <form class="login-card" onsubmit={submit}>
    <div class="login-logo">GALAX<b>YAS</b> POS</div>
    <div class="login-sub">
      Masuk untuk melanjutkan
      {#if $currentServer}
        <span class="text-dim"> · 🖧 {$currentServer.name}</span>
      {/if}
    </div>

    <div class="field">
      <label for="login-username">Username</label>
      <input id="login-username" bind:value={username} autocomplete="off" autocapitalize="none" />
    </div>
    <div class="field">
      <label for="login-pin">PIN</label>
      <input id="login-pin" type="password" bind:value={pin} placeholder="••••" inputmode="numeric" />
    </div>

    <button class="btn-primary" style="width:100%; padding:0.7rem;" disabled={busy} type="submit">
      {busy ? "Memproses…" : "Masuk"}
    </button>
    <button type="button" class="btn-ghost" style="width:100%; margin-top:0.6rem;" onclick={onChangeServer}>
      🖧 Ganti Server
    </button>
  </form>
</div>
