<script lang="ts">
  // Port mobile dari desktop ServerPicker.svelte: tanpa entry "Server Lokal"
  // (HP selalu client), plus tombol hapus per server. Layar ini juga jadi
  // onboarding wajib saat belum ada server tersimpan.
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

  const noServers = $derived($allServers.length === 0);

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
    const t = token.trim().toUpperCase();
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

  async function remove(id: string) {
    if (!confirm("Hapus server ini dari daftar?")) return;
    busy = true;
    try {
      await api.removeServer(id);
      allServers.update((list) => list.filter((s) => s.id !== id));
      const active = await api.currentServer();
      currentServer.set(active);
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="login-screen">
  <div class="login-card">
    <div class="login-logo">GALAX<b>YAS</b> POS</div>
    <div class="login-sub">
      {noServers ? "Hubungkan ke Server Pusat" : "Pilih Server Pusat"}
    </div>

    {#if noServers && !adding}
      <p class="text-dim intro">
        Aplikasi ini adalah kasir tambahan. Semua data (barang, stok, transaksi)
        tersimpan di PC yang menjalankan GALAXYAS POS sebagai <b>Server Pusat</b>
        — pastikan HP dan PC berada di jaringan wifi yang sama.
      </p>
      <ol class="text-dim steps">
        <li>Di PC: buka <b>Pengaturan → Server Pusat</b>, centang "Jadikan PC ini Server Pusat".</li>
        <li>Catat <b>IP, Port,</b> dan <b>Kode Pairing</b> yang tampil.</li>
        <li>Masukkan di sini.</li>
      </ol>
    {/if}

    {#if !noServers}
      <div class="store-list">
        {#each $allServers as s (s.id)}
          <div class="store-row-wrap">
            <button class="store-row" disabled={busy} onclick={() => choose(s.id)}>
              <span>🖧</span>
              <span style="flex:1; text-align:left;">
                {s.name}
                <span class="text-dim" style="font-size:0.75rem;"> · {s.host}:{s.port}</span>
              </span>
              {#if $currentServer?.id === s.id}<span class="text-dim" style="font-size:0.75rem;">aktif</span>{/if}
            </button>
            <button class="btn-ghost del" disabled={busy} onclick={() => remove(s.id)} aria-label="Hapus server">✕</button>
          </div>
        {/each}
      </div>
    {/if}

    {#if adding || noServers}
      <div class="card add-form">
        <div class="field-gap">
          <label for="sv-name">Nama Server</label>
          <input id="sv-name" bind:value={name} placeholder="Kasir Pusat" disabled={busy} />
        </div>
        <div class="field-gap">
          <label for="sv-host">IP Server Pusat</label>
          <input id="sv-host" bind:value={host} placeholder="192.168.1.10" inputmode="decimal" disabled={busy} />
        </div>
        <div class="field-gap">
          <label for="sv-port">Port</label>
          <input id="sv-port" bind:value={port} placeholder="8899" inputmode="numeric" disabled={busy} />
        </div>
        <div class="field-gap">
          <label for="sv-token">Kode Pairing</label>
          <input
            id="sv-token"
            bind:value={token}
            placeholder="6 karakter, lihat di PC"
            autocapitalize="characters"
            autocomplete="off"
            disabled={busy}
          />
        </div>
        <div class="row">
          <button class="btn-primary" style="flex:1;" disabled={busy} onclick={addAndChoose}>
            {busy ? "Menghubungkan…" : "Hubungkan"}
          </button>
          {#if !noServers}
            <button class="btn-ghost" disabled={busy} onclick={() => (adding = false)}>Batal</button>
          {/if}
        </div>
      </div>
    {:else}
      <button style="margin-top:0.8rem; width:100%;" disabled={busy} onclick={() => (adding = true)}>
        + Tambah Server
      </button>
    {/if}
  </div>
</div>

<style>
  .intro {
    font-size: 0.82rem;
    text-align: left;
    margin: 0 0 0.6rem;
  }
  .steps {
    font-size: 0.8rem;
    text-align: left;
    margin: 0 0 1rem;
    padding-left: 1.1rem;
  }
  .steps li {
    margin-bottom: 0.25rem;
  }
  .store-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 1rem;
  }
  .store-row-wrap {
    display: flex;
    gap: 0.3rem;
    align-items: stretch;
  }
  .store-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex: 1;
    text-align: left;
    padding: 0.7rem 0.9rem;
  }
  .del {
    color: var(--danger);
    min-width: 44px;
  }
  .add-form {
    margin-top: 0.8rem;
    padding: 0.8rem;
    text-align: left;
  }
  .field-gap {
    margin-bottom: 0.6rem;
  }
</style>
