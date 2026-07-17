<script lang="ts">
  import { api } from "$lib/api";
  import { setUser } from "$lib/stores/auth";
  import { showToast, toastError } from "$lib/toast";
  import { currentStore } from "$lib/stores/activeStore";
  import { currentServer } from "$lib/stores/activeServer";

  let { onChangeStore, onChangeServer }: { onChangeStore?: () => void; onChangeServer?: () => void } = $props();

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
      {#if $currentServer?.kind === "remote"}
        <span class="text-dim"> · 🖧 {$currentServer.name}</span>
      {:else if $currentStore}
        <span class="text-dim"> · {$currentStore.name}</span>
      {/if}
    </div>

    <div class="field">
      <label>Username</label>
      <input bind:value={username} autocomplete="off" />
    </div>
    <div class="field">
      <label>PIN</label>
      <input type="password" bind:value={pin} placeholder="••••" />
    </div>

    <button class="btn-primary" style="width:100%; padding:0.7rem;" disabled={busy} type="submit">
      {busy ? "Memproses…" : "Masuk"}
    </button>
    <div class="login-sub" style="margin:1rem 0 0;">Default: <code>admin</code> / PIN <code>1234</code></div>
    {#if onChangeStore}
      <button type="button" class="btn-ghost" style="width:100%; margin-top:0.6rem;" onclick={onChangeStore}>
        🏪 Ganti Toko
      </button>
    {/if}
    {#if onChangeServer}
      <button type="button" class="btn-ghost" style="width:100%; margin-top:0.4rem;" onclick={onChangeServer}>
        🖧 Ganti Server
      </button>
    {/if}
  </form>
</div>
