<script lang="ts">
  // Penanda global Remote GPOS: selama remote menyala, kasir selalu tahu —
  // di tab mana pun, bahkan di layar login — dan bisa langsung memutusnya.
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { api } from "$lib/api";
  import { toastError } from "$lib/toast";
  import type { RemoteHostStatus } from "$lib/types";

  let host = $state<RemoteHostStatus | null>(null);
  let unlisten: UnlistenFn | null = null;

  onMount(async () => {
    try {
      [host] = await api.remoteGposStatus();
    } catch {
      /* command belum ada (build lama) — banner diam saja */
    }
    unlisten = await listen<RemoteHostStatus>("remote-gpos://host", (e) => (host = e.payload));
  });
  onDestroy(() => unlisten?.());

  async function toggleControl() {
    if (!host) return;
    const allow = !host.allow_control;
    try {
      await api.remoteGposSetControl(allow);
      host = { ...host, allow_control: allow };
    } catch (e) {
      toastError(e);
    }
  }

  async function turnOff() {
    try {
      host = await api.remoteGposSetHost(false);
    } catch (e) {
      toastError(e);
    }
  }
</script>

{#if host?.enabled}
  <div class="remote-pill" class:live={host.phase === "connected"}>
    {#if host.phase === "connected"}
      <span>🔴 <b>Sedang diremote</b> · {host.allow_control ? "Kontrol penuh" : "Hanya lihat"}</span>
      <button onclick={toggleControl}>{host.allow_control ? "Jadikan hanya lihat" : "Izinkan kontrol"}</button>
    {:else}
      <span>🟢 Remote aktif{host.phase === "waiting" ? ` · OTP ${host.otp}` : ""}</span>
    {/if}
    <button class="off" onclick={turnOff}>Matikan</button>
  </div>
{/if}

<style>
  .remote-pill {
    position: fixed;
    right: 12px;
    bottom: 12px;
    z-index: 9999;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.5rem 0.4rem 0.8rem;
    border-radius: 999px;
    font-size: 0.8rem;
    background: #14532d;
    color: #fff;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.25);
  }
  .remote-pill.live {
    background: #991b1b;
  }
  .remote-pill button {
    padding: 0.2rem 0.6rem;
    font-size: 0.75rem;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.5);
    background: transparent;
    color: #fff;
    cursor: pointer;
  }
  .remote-pill button.off {
    background: rgba(255, 255, 255, 0.15);
  }
</style>
