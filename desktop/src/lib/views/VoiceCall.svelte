<script lang="ts">
  // Menu Voice Call: telepon toko lain (kontak chat atau ketik kodenya) +
  // riwayat panggilan 7 hari (pesan chat ber-kind "call").
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatDateTime } from "$lib/format";
  import { openTab } from "$lib/stores/tabs";
  import { chatContacts, chatDirectory, chatMessages, chatStatus, contactName, fmtDuration, resolveName } from "$lib/stores/chat";
  import { call, startCall } from "$lib/stores/call";
  import type { ChatMessage } from "$lib/types";
  import AudioSettings from "$lib/components/AudioSettings.svelte";

  let dial = $state("");
  let online = $state<Record<string, boolean>>({});
  let checking = $state(false);

  const me = $derived($chatStatus?.code ?? "");
  const busy = $derived($call.phase !== "idle" && $call.phase !== "ended");

  // Kontak tersimpan + toko yang pernah berkirim pesan / menelepon.
  const people = $derived.by(() => {
    const codes = new Set<string>($chatContacts.map((c) => c.code));
    for (const m of $chatMessages) {
      const peer = m.from === me ? m.to : m.from;
      if (peer && peer !== me) codes.add(peer);
    }
    void $chatDirectory; // nama dari web bisa menyusul
    return [...codes]
      .map((code) => ({ code, name: contactName(code) }))
      .sort((a, b) => a.name.localeCompare(b.name, "id", { sensitivity: "base" }));
  });

  const history = $derived(
    $chatMessages
      .filter((m) => m.kind === "call")
      .sort((a, b) => b.id - a.id)
      .slice(0, 60),
  );

  async function refreshOnline() {
    checking = true;
    const next: Record<string, boolean> = {};
    // Lookup dibatasi server 30/menit — cukup untuk daftar kontak toko.
    for (const p of people.slice(0, 20)) {
      try {
        next[p.code] = (await api.chatLookup(p.code)).online;
      } catch {
        /* kode sudah tidak terdaftar */
      }
    }
    online = next;
    checking = false;
  }

  onMount(() => {
    for (const p of people) resolveName(p.code);
    if ($chatStatus?.connected) refreshOnline();
  });

  function callDial(e: Event) {
    e.preventDefault();
    if (dial.trim()) startCall(dial.trim());
  }

  function outgoing(m: ChatMessage) {
    return m.from === me;
  }

  function historyText(m: ChatMessage): string {
    if (m.body === "missed") return outgoing(m) ? "Tidak dijawab" : "Panggilan tak terjawab";
    if (m.body === "rejected") return "Ditolak";
    return fmtDuration(Number(m.body) || 0);
  }

  function initials(name: string): string {
    return name.split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0]?.toUpperCase()).join("") || "?";
  }

  function openChatSetup() {
    openTab({ viewKey: "chat", title: "Chat Toko", icon: "💬", singleton: true });
  }
</script>

<div class="page-head"><h1>Voice Call</h1></div>

{#if !$chatStatus?.configured}
  <div class="card" style="max-width:460px;">
    <p style="margin-top:0;">Voice Call memakai akun Chat toko. Atur kode toko &amp; Kunci Chat di menu Chat dulu.</p>
    <button class="btn-primary" onclick={openChatSetup}>💬 Buka Chat</button>
  </div>
{:else}
  {#if !$chatStatus.connected}
    <div class="offline-bar">Chat sedang offline — belum bisa menelepon. GPOS menyambung ulang otomatis.</div>
  {/if}

  <div class="grid-2" style="align-items:start;">
    <div class="card">
      <h2>Telepon Toko</h2>
      <form class="dial" onsubmit={callDial}>
        <input class="mono" bind:value={dial} placeholder="Ketik kode toko, mis. toko-002" disabled={busy} />
        <button class="btn-success" type="submit" disabled={busy || !$chatStatus.connected || !dial.trim()}>📞 Telepon</button>
      </form>

      <div class="list-head">
        <span>Kontak</span>
        <button class="btn-ghost small" disabled={checking || !$chatStatus.connected} onclick={refreshOnline}>
          {checking ? "Mengecek…" : "↻ Cek online"}
        </button>
      </div>
      <div class="people">
        {#each people as p (p.code)}
          <div class="person">
            <div class="avatar">{initials(p.name)}</div>
            <div class="who">
              <div class="name">{p.name}</div>
              <div class="code mono">
                {p.code}
                {#if p.code in online}
                  · <span class:on={online[p.code]}>{online[p.code] ? "● online" : "○ offline"}</span>
                {/if}
              </div>
            </div>
            <button class="call-btn" title="Telepon {p.name}" disabled={busy || !$chatStatus.connected} onclick={() => startCall(p.code)}>📞</button>
          </div>
        {:else}
          <div class="text-dim empty">Belum ada kontak. Ketik kode toko di atas untuk menelepon.</div>
        {/each}
      </div>
    </div>

    <div class="side">
    <AudioSettings />
    <div class="card">
      <h2>Riwayat Panggilan</h2>
      <div class="history">
        {#each history as m (m.id)}
          {@const peer = outgoing(m) ? m.to : m.from}
          <div class="hist-row" class:missed={m.body === "missed" && !outgoing(m)}>
            <span class="dir" title={outgoing(m) ? "Panggilan keluar" : "Panggilan masuk"}>{outgoing(m) ? "↗" : "↙"}</span>
            <div class="who">
              <div class="name">{contactName(peer)}</div>
              <div class="code">{historyText(m)} · {formatDateTime(m.created_at)}</div>
            </div>
            <button class="call-btn" title="Telepon balik" disabled={busy || !$chatStatus.connected} onclick={() => startCall(peer)}>📞</button>
          </div>
        {:else}
          <div class="text-dim empty">Belum ada panggilan 7 hari terakhir.</div>
        {/each}
      </div>
    </div>
    </div>
  </div>
{/if}

<style>
  .offline-bar {
    background: color-mix(in srgb, var(--warning) 18%, var(--white));
    border: 1px solid var(--warning);
    border-radius: 8px;
    padding: 0.5rem 0.8rem;
    margin-bottom: 0.8rem;
    font-size: 0.85rem;
  }
  .side { display: flex; flex-direction: column; gap: 0.8rem; }
  .dial { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
  .dial input { flex: 1; }
  .list-head { display: flex; justify-content: space-between; align-items: center; font-weight: 650; margin-bottom: 0.4rem; }
  .small { padding: 0.25rem 0.6rem; font-size: 0.78rem; }
  .people { max-height: 60vh; overflow-y: auto; border: 1px solid var(--border); border-radius: 8px; }
  .history { max-height: 40vh; overflow-y: auto; border: 1px solid var(--border); border-radius: 8px; }
  .person, .hist-row { display: flex; align-items: center; gap: 0.6rem; padding: 0.5rem 0.7rem; border-bottom: 1px solid var(--border); }
  .person:last-child, .hist-row:last-child { border-bottom: none; }
  .avatar {
    width: 34px; height: 34px; border-radius: 50%; flex: none;
    display: flex; align-items: center; justify-content: center;
    background: var(--primary); color: #fff; font-weight: 700; font-size: 0.8rem;
  }
  .who { flex: 1; min-width: 0; }
  .name { font-weight: 650; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .code { font-size: 0.78rem; color: var(--text-dim); }
  .code .on { color: var(--success); font-weight: 650; }
  .dir { font-size: 1.1rem; color: var(--success); width: 1.2rem; text-align: center; }
  .hist-row.missed .dir, .hist-row.missed .name { color: var(--danger); }
  .call-btn {
    border: none; border-radius: 50%; width: 36px; height: 36px; flex: none;
    background: var(--success); color: #fff; font-size: 1rem; padding: 0;
  }
  .call-btn:disabled { opacity: 0.45; }
  .empty { padding: 0.8rem; font-size: 0.85rem; }
</style>
