<script lang="ts">
  // Popup alert chat: muncul di tab mana pun, HANYA untuk pesan yang dikirim
  // sebagai alert. Pesan biasa tidak memunculkan apa-apa. Juga tempat
  // listener chat global dipasang (initChat), karena komponen ini selalu ada.
  import { onMount } from "svelte";
  import { chatAlerts, contactName, initChat, messagePreview } from "$lib/stores/chat";
  import { openTab } from "$lib/stores/tabs";

  onMount(() => {
    initChat();
  });

  const first = $derived($chatAlerts[0] ?? null);

  function close() {
    chatAlerts.update((a) => a.slice(1));
  }

  function closeAll() {
    chatAlerts.set([]);
  }

  function view() {
    if (!first) return;
    const peer = first.from;
    // Semua alert dari toko yang sama dianggap sudah dilihat.
    chatAlerts.update((a) => a.filter((m) => m.from !== peer));
    openTab({ viewKey: "chat", title: "Chat Toko", icon: "💬", singleton: true, props: { peer } });
  }

  function preview(m: { kind: string; body: string; file_name: string | null }): string {
    const text = messagePreview(m);
    return text.length > 280 ? text.slice(0, 280) + "…" : text;
  }
</script>

{#if first}
  <!-- Sengaja BUKAN .modal-backdrop: Esc global menutup modal-backdrop
       terakhir, dan alert tidak boleh hilang tanpa sengaja. -->
  <div class="chat-alert-wrap">
    <div class="chat-alert" role="alertdialog" aria-labelledby="chat-alert-title">
      <div id="chat-alert-title" class="title">🔔 Ada pesan dari <b>{contactName(first.from)}</b></div>
      <div class="msg">{preview(first)}</div>
      {#if $chatAlerts.length > 1}
        <div class="more">+{$chatAlerts.length - 1} alert lain</div>
      {/if}
      <div class="actions">
        {#if $chatAlerts.length > 1}<button class="btn-ghost" onclick={closeAll}>Tutup semua</button>{/if}
        <button class="btn-ghost" onclick={close}>Tutup</button>
        <button class="btn-primary" onclick={view}>Lihat</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .chat-alert-wrap {
    position: fixed;
    inset: 0;
    z-index: 9998;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
    background: rgba(15, 23, 42, 0.25);
  }
  .chat-alert {
    width: 440px;
    max-width: 92vw;
    background: var(--white, #fff);
    border: 2px solid #f59e0b;
    border-radius: 12px;
    padding: 1rem 1.1rem;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3);
  }
  .title {
    font-size: 1rem;
    margin-bottom: 0.5rem;
  }
  .msg {
    white-space: pre-wrap;
    word-break: break-word;
    background: rgba(245, 158, 11, 0.1);
    border-radius: 8px;
    padding: 0.6rem 0.7rem;
  }
  .more {
    font-size: 0.8rem;
    opacity: 0.7;
    margin-top: 0.4rem;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.8rem;
  }
</style>
