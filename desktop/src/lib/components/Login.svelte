<script lang="ts">
  import { api } from "$lib/api";
  import { setUser } from "$lib/stores/auth";
  import { showToast, toastError } from "$lib/toast";

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
    <div class="login-sub">Masuk untuk melanjutkan</div>

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
  </form>
</div>
