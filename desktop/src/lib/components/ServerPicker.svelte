<script lang="ts">
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import { currentServer, allServers } from "$lib/stores/activeServer";

  let { onChosen }: { onChosen: (info: import("$lib/types").ServerInfo) => void } = $props();

  let adding = $state(false);
  let busy = $state(false);
  let name = $state("");
  let host = $state("");
  let port = $state("8899");
  let token = $state("");

  async function choose(id: string) {
    busy = true;
    try {
      const info = await api.selectServer(id);
      currentServer.set(info);
      onChosen(info);
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  async function addAndChoose() {
    const n = name.trim();
    const h = host.trim();
    const p = parseInt(port, 10);
    const t = token.trim();
    if (!n || !h || !t || !p) {
      return showToast("Nama, IP, Port, dan Kode Pairing wajib diisi.", "error");
    }
    busy = true;
    try {
      const info = await api.addServer(n, h, p, t);
      allServers.update((list) => [...list, info]);
      name = "";
      host = "";
      port = "8899";
      token = "";
      adding = false;
      await choose(info.id);
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="login-screen">
  <div class="login-card" style="max-width:440px;">
    <div class="login-logo">GALAX<b>YAS</b> POS</div>
    <div class="login-sub">Pilih Server</div>

    <div class="store-list">
      {#each $allServers as s (s.id)}
        <button class="store-row" disabled={busy} onclick={() => choose(s.id)}>
          <span>{s.kind === "local" ? "💻" : "🖧"}</span>
          <span style="flex:1; text-align:left;">
            {s.name}
            {#if s.kind === "remote"}<span class="text-dim" style="font-size:0.75rem;"> · {s.host}:{s.port}</span>{/if}
          </span>
          {#if $currentServer?.id === s.id}<span class="text-dim" style="font-size:0.75rem;">aktif</span>{/if}
        </button>
      {/each}
    </div>

    {#if adding}
      <div class="card" style="margin-top:0.8rem; padding:0.8rem;">
        <div style="margin-bottom:0.6rem;">
          <label>Nama Server</label>
          <input bind:value={name} placeholder="Kasir Pusat" disabled={busy} />
        </div>
        <div style="margin-bottom:0.6rem;">
          <label>IP Server Pusat</label>
          <input bind:value={host} placeholder="192.168.1.10" disabled={busy} />
        </div>
        <div style="margin-bottom:0.6rem;">
          <label>Port</label>
          <input bind:value={port} placeholder="8899" disabled={busy} />
        </div>
        <div style="margin-bottom:0.6rem;">
          <label>Kode Pairing</label>
          <input bind:value={token} placeholder="Lihat di Pengaturan > Server Pusat PC tujuan" disabled={busy} />
        </div>
        <div class="row">
          <button class="btn-primary" disabled={busy} onclick={addAndChoose}>Hubungkan</button>
          <button class="btn-ghost" disabled={busy} onclick={() => (adding = false)}>Batal</button>
        </div>
      </div>
    {:else}
      <button style="margin-top:0.8rem; width:100%;" disabled={busy} onclick={() => (adding = true)}>
        + Tambah Server
      </button>
    {/if}

    <p class="text-dim" style="font-size:0.76rem; margin-top:0.9rem; text-align:center;">
      Pilih "Server Lokal" untuk pemakaian satu PC seperti biasa, atau tambah Server Pusat
      untuk berbagi data dengan kasir lain di jaringan wifi yang sama.
    </p>
  </div>
</div>

<style>
  .store-list { display: flex; flex-direction: column; gap: 0.5rem; margin-top: 1rem; }
  .store-row { display: flex; align-items: center; gap: 0.6rem; width: 100%; text-align: left; padding: 0.7rem 0.9rem; }
</style>
