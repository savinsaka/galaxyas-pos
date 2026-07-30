<script lang="ts">
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import { currentServer, currentPath, allServers } from "$lib/stores/activeServer";
  import { hasLanPath, hasOnlinePath } from "$lib/types";
  import type { ActiveServer, ServerInfo, ServerInput, ServerPath } from "$lib/types";

  let { onChosen }: { onChosen: (active: ActiveServer) => void } = $props();

  let adding = $state(false);
  let busy = $state(false);
  let setupCode = $state("");
  let name = $state("");
  let host = $state("");
  let port = $state("8899");
  let relayUrl = $state("");
  let storeId = $state("");
  let code = $state("");
  let path = $state<ServerPath>("lan");

  function alamatWifi(s: ServerInfo) {
    return hasLanPath(s) ? `${s.lan_host}:${s.lan_port ?? 8899}` : "belum diatur";
  }

  function alamatInternet(s: ServerInfo) {
    return hasOnlinePath(s) ? (s.relay_url ?? "") : "belum diatur";
  }

  function aktif(s: ServerInfo, p: ServerPath) {
    return $currentServer?.id === s.id && $currentPath === p;
  }

  async function choose(id: string, p?: ServerPath) {
    busy = true;
    try {
      const active = await api.selectServer(id, p);
      currentServer.set(active.server);
      currentPath.set(active.path);
      onChosen(active);
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  function buildInput(): ServerInput {
    return {
      name: name.trim(),
      lan_host: host.trim(),
      lan_port: parseInt(port, 10) || 8899,
      relay_url: relayUrl.trim(),
      store_id: storeId.trim(),
      code: code.trim(),
      path,
    };
  }

  /// Kode Setup memuat semua isian sekaligus — Store ID relay 32 karakter hex
  /// mustahil diketik benar, dan PC tidak punya kamera untuk scan QR.
  async function fillFromSetupCode() {
    busy = true;
    try {
      const p = await api.decodeSetupCode(setupCode);
      name = p.name;
      host = p.host;
      port = String(p.port || 8899);
      relayUrl = p.relay;
      storeId = p.store_id;
      code = p.code;
      // Arahkan ke jalur yang memang tersedia di kode tsb.
      path = p.relay && p.store_id ? "online" : "lan";
      showToast(
        p.relay
          ? "Isian terisi dari Kode Setup — jalur wifi & internet keduanya siap."
          : "Isian terisi dari Kode Setup — hanya jalur wifi (Akses Online belum aktif di PC pusat).",
        "success",
      );
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  async function testConnection() {
    busy = true;
    try {
      await api.pingServer(buildInput());
      showToast(
        path === "online" ? "PC pusat terjangkau lewat internet." : "PC pusat terjangkau lewat wifi.",
        "success",
      );
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  async function addAndChoose() {
    if (!code.trim()) return showToast("Kode Pairing wajib diisi.", "error");
    if (path === "lan" && !host.trim()) {
      return showToast("IP Server Pusat wajib diisi untuk jalur wifi.", "error");
    }
    if (path === "online" && (!relayUrl.trim() || !storeId.trim())) {
      return showToast("Alamat Relay dan Store ID wajib diisi untuk jalur internet.", "error");
    }
    busy = true;
    try {
      const info = await api.addServer(buildInput());
      allServers.update((list) => [...list, info]);
      setupCode = "";
      name = "";
      host = "";
      port = "8899";
      relayUrl = "";
      storeId = "";
      code = "";
      adding = false;
      await choose(info.id, path);
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="login-screen">
  <!-- `.login-card` di app.css lebarnya dipatok 360px (max-width tidak
       berpengaruh). Form ini punya isian panjang (Store ID 32 karakter, Kode
       Setup), jadi lebarnya ditimpa di sini. -->
  <div class="login-card" style="width:500px; max-width:92vw;">
    <div class="login-logo">GALAX<b>YAS</b> POS</div>
    <div class="login-sub">Pilih Server</div>

    <div class="store-list">
      {#each $allServers as s (s.id)}
        {#if s.kind === "local"}
          <button class="store-row" disabled={busy} onclick={() => choose(s.id)}>
            <span>💻</span>
            <span style="flex:1; text-align:left;">{s.name}</span>
            {#if $currentServer?.id === s.id}
              <span class="text-dim" style="font-size:0.75rem;">aktif</span>
            {/if}
          </button>
        {:else}
          <!-- Satu server = dua jalur. Kasir memilih sendiri tiap membuka app;
               app TIDAK menebak/berpindah sendiri. -->
          <div class="server-card">
            <div class="server-name">🖧 {s.name}</div>
            <div class="row" style="gap:0.5rem;">
              <button
                class="path-btn"
                class:path-active={aktif(s, "lan")}
                disabled={busy || !hasLanPath(s)}
                onclick={() => choose(s.id, "lan")}
                title={hasLanPath(s) ? `Lewat wifi toko · ${alamatWifi(s)}` : "Alamat wifi belum diatur"}
              >
                🖧 WIFI
              </button>
              <button
                class="path-btn"
                class:path-active={aktif(s, "online")}
                disabled={busy || !hasOnlinePath(s)}
                onclick={() => choose(s.id, "online")}
                title={hasOnlinePath(s) ? `Lewat internet · ${alamatInternet(s)}` : "Alamat internet belum diatur"}
              >
                🌐 INTERNET
              </button>
              {#if $currentServer?.id === s.id}
                <span class="text-dim" style="font-size:0.75rem; margin-left:auto;">aktif</span>
              {/if}
            </div>
            <div class="text-dim server-addr">
              wifi: {alamatWifi(s)} · internet: {alamatInternet(s)}
            </div>
          </div>
        {/if}
      {/each}
    </div>

    {#if adding}
      <div class="card" style="margin-top:0.8rem; padding:0.8rem;">
        <div style="margin-bottom:0.6rem;">
          <label>Kode Setup dari PC pusat</label>
          <div class="row">
            <input bind:value={setupCode} placeholder="GXP1-…" disabled={busy} />
            <button disabled={busy} onclick={fillFromSetupCode}>Isi Otomatis</button>
          </div>
          <div class="text-dim" style="font-size:0.74rem; margin-top:0.25rem;">
            Di PC pusat: Pengaturan → Server Pusat → tombol Salin di "Kode Setup PC Kasir".
            Bisa juga diisi manual di bawah.
          </div>
        </div>

        <div style="margin-bottom:0.6rem;">
          <label>Nama Server</label>
          <input bind:value={name} placeholder="Kasir Pusat" disabled={busy} />
        </div>
        <div class="row" style="margin-bottom:0.6rem; align-items:flex-end;">
          <div style="flex:1;">
            <label>IP Server Pusat (wifi)</label>
            <input bind:value={host} placeholder="192.168.1.10" disabled={busy} />
          </div>
          <div style="width:90px;">
            <label>Port</label>
            <input bind:value={port} placeholder="8899" disabled={busy} />
          </div>
        </div>
        <div style="margin-bottom:0.6rem;">
          <label>Alamat Relay (internet)</label>
          <input bind:value={relayUrl} placeholder="relay.jjapps.net" disabled={busy} />
        </div>
        <div style="margin-bottom:0.6rem;">
          <label>Store ID</label>
          <input bind:value={storeId} placeholder="32 karakter dari PC pusat" disabled={busy} />
        </div>
        <div style="margin-bottom:0.6rem;">
          <label>Kode Pairing</label>
          <input bind:value={code} placeholder="6 karakter, lihat Pengaturan PC pusat" disabled={busy} />
        </div>

        <div style="margin-bottom:0.7rem;">
          <label>Hubungkan lewat</label>
          <div class="row" style="gap:1rem;">
            <label class="row" style="gap:0.35rem; width:auto;">
              <input type="radio" style="width:auto;" value="lan" bind:group={path} disabled={busy} />
              🖧 Wifi toko
            </label>
            <label class="row" style="gap:0.35rem; width:auto;">
              <input type="radio" style="width:auto;" value="online" bind:group={path} disabled={busy} />
              🌐 Internet
            </label>
          </div>
        </div>

        <div class="row">
          <button class="btn-primary" disabled={busy} onclick={addAndChoose}>Hubungkan</button>
          <button disabled={busy} onclick={testConnection}>Uji Koneksi</button>
          <button class="btn-ghost" disabled={busy} onclick={() => (adding = false)}>Batal</button>
        </div>
      </div>
    {:else}
      <button style="margin-top:0.8rem; width:100%;" disabled={busy} onclick={() => (adding = true)}>
        + Tambah Server
      </button>
    {/if}

    <p class="text-dim" style="font-size:0.76rem; margin-top:0.9rem; text-align:center;">
      Pilih "Server Lokal" untuk pemakaian satu PC seperti biasa. Untuk berbagi data dengan
      kasir lain: <b>WIFI</b> bila PC ini satu jaringan dengan PC pusat, atau <b>INTERNET</b>
      bila dari tempat lain — PC pusat harus menyala dengan Akses Online aktif.
    </p>
  </div>
</div>

<style>
  .store-list { display: flex; flex-direction: column; gap: 0.5rem; margin-top: 1rem; }
  .store-row { display: flex; align-items: center; gap: 0.6rem; width: 100%; text-align: left; padding: 0.7rem 0.9rem; }
  .server-card {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.6rem 0.7rem;
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }
  .server-name { font-weight: 600; }
  .path-btn { width: auto; padding: 0.4rem 0.8rem; font-size: 0.82rem; }
  .path-active { outline: 2px solid var(--primary); font-weight: 600; }
  .server-addr { font-size: 0.72rem; }
</style>
