<script lang="ts">
  // Tampilan Voice Call di tab mana pun: popup panggilan masuk (Angkat/Tolak)
  // dan bar panggilan (memanggil / bicara / selesai). Juga tempat listener
  // sinyal panggilan dipasang, karena komponen ini selalu ada saat login.
  import { onDestroy, onMount } from "svelte";
  import { call, initCall, acceptCall, rejectCall, hangupCall, toggleMute } from "$lib/stores/call";
  import { fmtDuration } from "$lib/stores/chat";

  let now = $state(Date.now());
  let timer: ReturnType<typeof setInterval> | undefined;
  onMount(() => {
    initCall();
    timer = setInterval(() => (now = Date.now()), 1000);
  });
  onDestroy(() => clearInterval(timer));

  const status = $derived.by(() => {
    const c = $call;
    switch (c.phase) {
      case "outgoing": return "Memanggil…";
      case "ringing": return "Berdering…";
      case "connecting": return "Menyambungkan…";
      case "active": return c.startedAt ? fmtDuration((now - c.startedAt) / 1000) : "0:00";
      case "ended": return c.endText ?? "Panggilan selesai";
      default: return "";
    }
  });

  function initials(name: string): string {
    return name.split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0]?.toUpperCase()).join("") || "?";
  }
</script>

{#if $call.phase === "incoming"}
  <!-- Sengaja BUKAN .modal-backdrop: Esc global menutup modal-backdrop
       terakhir, dan panggilan masuk tidak boleh hilang tanpa sengaja. -->
  <div class="call-incoming-wrap">
    <div class="call-incoming" role="alertdialog" aria-labelledby="call-in-title">
      <div class="call-avatar ring">{initials($call.peerName)}</div>
      <div class="call-label">📞 Panggilan suara masuk</div>
      <div id="call-in-title" class="call-name">{$call.peerName}</div>
      <div class="call-code mono">{$call.peer}</div>
      <div class="call-actions">
        <button class="call-btn reject" onclick={rejectCall}>✕ Tolak</button>
        <button class="call-btn accept" onclick={acceptCall}>📞 Angkat</button>
      </div>
    </div>
  </div>
{:else if $call.phase !== "idle"}
  <div class="call-bar" class:ended={$call.phase === "ended"} role="status">
    <div class="call-avatar small">{initials($call.peerName)}</div>
    <div class="call-info">
      <div class="call-bar-name">{$call.peerName}</div>
      <div class="call-bar-status" class:live={$call.phase === "active"}>
        {#if $call.phase === "active"}● {/if}{status}{#if $call.muted} · 🔇 dibisukan{/if}
      </div>
    </div>
    {#if $call.phase !== "ended"}
      {#if $call.phase === "active" || $call.phase === "connecting"}
        <button class="call-btn mute" class:on={$call.muted} onclick={toggleMute} title={$call.muted ? "Nyalakan mic" : "Bisukan mic"}>
          {$call.muted ? "🔇" : "🎙️"}
        </button>
      {/if}
      <button class="call-btn reject" onclick={hangupCall} title="Tutup panggilan">📵 Tutup</button>
    {/if}
  </div>
{/if}

<style>
  .call-incoming-wrap {
    position: fixed;
    inset: 0;
    z-index: 9999;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
    background: rgba(15, 23, 42, 0.35);
  }
  .call-incoming {
    width: 340px;
    max-width: 92vw;
    background: var(--white);
    color: var(--text);
    border: 2px solid var(--success);
    border-radius: 16px;
    padding: 1.4rem 1.2rem 1.2rem;
    text-align: center;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3);
  }
  .call-avatar {
    width: 72px;
    height: 72px;
    margin: 0 auto 0.6rem;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.6rem;
    font-weight: 700;
    color: #fff;
    background: var(--success);
  }
  .call-avatar.ring { animation: call-pulse 1.4s ease-out infinite; }
  @keyframes call-pulse {
    0% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--success) 55%, transparent); }
    100% { box-shadow: 0 0 0 22px transparent; }
  }
  .call-avatar.small { width: 40px; height: 40px; font-size: 0.95rem; margin: 0; flex: none; }
  .call-label { font-size: 0.85rem; color: var(--text-dim); }
  .call-name { font-size: 1.25rem; font-weight: 700; margin-top: 0.2rem; word-break: break-word; }
  .call-code { font-size: 0.8rem; color: var(--text-dim); }
  .call-actions { display: flex; gap: 0.7rem; margin-top: 1.1rem; }
  .call-actions .call-btn { flex: 1; padding: 0.75rem; font-size: 0.95rem; }
  .call-btn {
    border: none;
    border-radius: 999px;
    font-weight: 700;
    color: #fff;
    padding: 0.45rem 0.9rem;
    cursor: pointer;
    white-space: nowrap;
  }
  .call-btn.accept { background: var(--success); }
  .call-btn.reject { background: var(--danger); }
  .call-btn.mute { background: var(--border-strong); color: var(--text); }
  .call-btn.mute.on { background: var(--warning); color: #fff; }
  .call-bar {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 9997;
    display: flex;
    align-items: center;
    gap: 0.7rem;
    min-width: 300px;
    max-width: 92vw;
    background: var(--white);
    color: var(--text);
    border: 2px solid var(--success);
    border-radius: 14px;
    padding: 0.6rem 0.7rem;
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.25);
  }
  .call-bar.ended { border-color: var(--border-strong); }
  .call-info { flex: 1; min-width: 0; }
  .call-bar-name { font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .call-bar-status { font-size: 0.8rem; color: var(--text-dim); }
  .call-bar-status.live { color: var(--success); font-weight: 650; }
</style>
