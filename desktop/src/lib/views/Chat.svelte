<script lang="ts">
  // Chat antar toko. Pesan biasa tidak memunculkan notifikasi apa pun —
  // penerima baru tahu saat membuka menu ini. Pesan yang dicentang "alert"
  // memunculkan popup di GPOS toko tujuan (ChatAlert.svelte).
  import { onMount, onDestroy, tick } from "svelte";
  import { confirmDialog } from "$lib/dialog";
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import {
    chatStatus,
    chatMessages,
    chatContacts,
    chatDirectory,
    chatOutbox,
    discardOut,
    fmtDuration,
    initChat,
    messagePreview,
    sendVoice,
    retryOut,
    sendFile,
    sendText,
    isUnread,
    markRead,
    peerOf,
    refreshHistory,
    resolveName,
    callLabel,
  } from "$lib/stores/chat";
  import { call, startCall } from "$lib/stores/call";
  import { activeTabId } from "$lib/stores/tabs";
  import type { ChatMessage, ChatOutgoing } from "$lib/types";
  import VoicePlayer from "$lib/components/VoicePlayer.svelte";
  import FilePreview from "$lib/components/FilePreview.svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import {
    CHAT_ACCEPT,
    fileKind,
    fmtSize,
    isImagePreview,
    kindLabel,
    mimeOf,
    rejectReason,
    screenshotName,
  } from "$lib/chatFiles";

  let { peer: initialPeer = "", tabId = "" }: { peer?: string; tabId?: string } = $props();

  // Tab yang tidak aktif tetap hidup di belakang (Workspace), jadi "dibaca"
  // hanya dihitung saat tab ini aktif DAN jendela GPOS sedang terlihat.
  let pageVisible = $state(typeof document === "undefined" || document.visibilityState === "visible");
  const onVisibility = () => (pageVisible = document.visibilityState === "visible");
  const onScreen = $derived(pageVisible && (!tabId || $activeTabId === tabId));

  let suggestedCode = $state("");
  let setupCode = $state("");
  let setupKey = $state("");
  let showSetup = $state(false);
  let busy = $state(false);

  let selected = $state("");
  let draft = $state("");
  let asAlert = $state(false);
  let fileInput = $state<HTMLInputElement | null>(null);
  let scroller = $state<HTMLDivElement | null>(null);

  let addOpen = $state(false);
  let addCode = $state("");
  let addName = $state("");
  let addFound = $state(false);

  let renaming = $state(false);
  let renameValue = $state("");

  const me = $derived($chatStatus?.code ?? "");
  const configured = $derived($chatStatus?.configured ?? false);

  // Kontak = yang disimpan + kode asing yang pernah mengirim/dikirimi pesan.
  const peers = $derived.by(() => {
    type Peer = { code: string; name: string; saved: boolean; last: number; unread: number; lastAt: string; lastText: string; lastMine: boolean };
    const map = new Map<string, Peer>();
    const blank = { last: 0, unread: 0, lastAt: "", lastText: "", lastMine: false };
    for (const c of $chatContacts) map.set(c.code, { code: c.code, name: c.name, saved: true, ...blank });
    const dir = $chatDirectory;
    for (const m of $chatMessages) {
      const p = peerOf(m, me);
      if (!p) continue;
      let e = map.get(p);
      if (!e) {
        e = { code: p, name: dir[p] ?? p, saved: false, ...blank };
        map.set(p, e);
      }
      if (m.id >= e.last) {
        e.last = m.id;
        e.lastAt = m.created_at;
        e.lastText = messagePreview(m);
        e.lastMine = m.from === me;
      }
      if (isUnread(m, me)) e.unread++;
    }
    return [...map.values()].sort((a, b) => b.last - a.last || a.name.localeCompare(b.name));
  });

  const current = $derived(peers.find((p) => p.code === selected) ?? null);
  const thread = $derived($chatMessages.filter((m) => peerOf(m, me) === selected));
  const outgoing = $derived($chatOutbox.filter((x) => x.to === selected));
  const online = $derived($chatStatus?.connected ?? false);
  // Kunci salah / diganti dari admin web: menyambung ulang sendiri tidak akan
  // berhasil, jadi tampilkan tombol untuk memasukkan kunci baru.
  const keyProblem = $derived(!online && /kunci|toko dihapus/i.test($chatStatus?.error ?? ""));

  // Satu linimasa: pesan dari server + pesan keluar yang belum/tidak terkirim,
  // lengkap dengan pemisah tanggal ala aplikasi chat.
  type Item = { key: string; mine: boolean; at: string; msg?: ChatMessage; out?: ChatOutgoing; day?: string };
  const items = $derived.by(() => {
    const list: Item[] = [
      ...thread.map((m) => ({ key: "m" + m.id, mine: m.from === me, at: m.created_at, msg: m })),
      ...outgoing.map((x) => ({ key: "o" + x.client_id, mine: true, at: x.created_at, out: x })),
    ];
    let prev = "";
    for (const it of list) {
      const d = new Date(it.at).toDateString();
      if (d !== prev) it.day = dayLabel(it.at);
      prev = d;
    }
    return list;
  });

  let unlistenDrop: (() => void) | null = null;

  onMount(async () => {
    document.addEventListener("visibilitychange", onVisibility);
    // Seret file dari Explorer: event drop HTML5 ditelan Tauri, jadi pakai
    // event drag-drop webview (berisi path) lalu baca isinya lewat Rust.
    getCurrentWebview()
      .onDragDropEvent((e) => {
        const p = e.payload;
        if (p.type === "leave") return void (dragOver = false);
        if (!canAttach) return;
        if (p.type === "enter" || p.type === "over") dragOver = true;
        else if (p.type === "drop") {
          dragOver = false;
          addDropped(p.paths);
        }
      })
      .then((u) => (unlistenDrop = u))
      .catch(() => {});
    await initChat();
    try {
      const s = await api.chatStatus();
      suggestedCode = s.suggested_code;
      setupCode = s.status.code || s.suggested_code;
    } catch (e) {
      toastError(e);
    }
    if (configured) refreshHistory();
  });

  onDestroy(() => {
    document.removeEventListener("visibilitychange", onVisibility);
    unlistenDrop?.();
    clearStaged();
    for (const u of thumbCache.values()) URL.revokeObjectURL(u);
    // Tab ditutup saat merekam: lepaskan mikrofon, jangan kirim apa pun.
    stopRecording(false);
  });

  // ---------- Voice note ----------
  const MAX_VOICE_S = 120;
  let recording = $state(false);
  let recSeconds = $state(0);
  let recorder: MediaRecorder | null = null;
  let recStream: MediaStream | null = null;
  let recChunks: Blob[] = [];
  let recStartedAt = 0;
  let recTimer: ReturnType<typeof setInterval> | null = null;
  let recTarget = "";
  let recPush = $state(false);

  async function startRecording() {
    if (!selected || recording) return;
    try {
      recStream = await navigator.mediaDevices.getUserMedia({ audio: true });
    } catch {
      showToast("Mikrofon tidak bisa dipakai. Cek mikrofon PC & izinnya.", "error", 5000);
      return;
    }
    const mime = MediaRecorder.isTypeSupported("audio/webm;codecs=opus") ? "audio/webm;codecs=opus" : "audio/webm";
    recorder = new MediaRecorder(recStream, { mimeType: mime, audioBitsPerSecond: 32000 });
    recChunks = [];
    recorder.ondataavailable = (e) => {
      if (e.data.size > 0) recChunks.push(e.data);
    };
    recTarget = selected;
    recPush = asAlert;
    recorder.start();
    recording = true;
    recStartedAt = Date.now();
    recSeconds = 0;
    recTimer = setInterval(() => {
      recSeconds = (Date.now() - recStartedAt) / 1000;
      // Batas 2 menit: berhenti & langsung dikirim.
      if (recSeconds >= MAX_VOICE_S) stopRecording(true);
    }, 200);
  }

  function stopRecording(send: boolean) {
    if (recTimer) clearInterval(recTimer);
    recTimer = null;
    const rec = recorder;
    const stream = recStream;
    recorder = null;
    recStream = null;
    if (!rec) {
      recording = false;
      return;
    }
    const duration = Math.min(MAX_VOICE_S, (Date.now() - recStartedAt) / 1000);
    const target = recTarget;
    const push = recPush;
    rec.onstop = async () => {
      stream?.getTracks().forEach((t) => t.stop());
      recording = false;
      if (!send || duration < 0.8) return; // terlalu pendek = anggap batal
      const blob = new Blob(recChunks, { type: "audio/webm" });
      recChunks = [];
      asAlert = false;
      await sendVoice(target, new Uint8Array(await blob.arrayBuffer()), Math.round(duration), push);
    };
    if (rec.state !== "inactive") rec.stop();
    else rec.onstop?.(new Event("stop"));
  }

  // Pindah kontak saat merekam = batal.
  $effect(() => {
    void selected;
    if (recording && selected !== recTarget) stopRecording(false);
  });

  // Kode yang belum disimpan: tampilkan nama terdaftarnya di web.
  $effect(() => {
    for (const p of peers) if (!p.saved) resolveName(p.code);
  });

  // Dibuka dari popup alert ("Lihat") → langsung ke percakapan itu.
  $effect(() => {
    if (initialPeer) selected = initialPeer;
  });

  // Percakapan yang sedang terlihat → pesan masuknya dibaca (✓✓ biru di pengirim).
  $effect(() => {
    if (onScreen && selected && thread.some((m) => isUnread(m, me))) markRead(selected);
  });

  $effect(() => {
    void thread.length;
    void outgoing.length;
    tick().then(() => scroller?.scrollTo({ top: scroller.scrollHeight }));
  });

  async function saveSetup() {
    busy = true;
    try {
      await api.chatSetup(setupCode, setupKey);
      setupKey = "";
      showSetup = false;
      showToast("Chat diatur. Menyambung ke server…", "success");
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  async function logout() {
    if (!(await confirmDialog("Keluar dari chat di PC ini? Kunci Chat perlu dimasukkan lagi untuk masuk.", { title: "Keluar Chat?", okText: "Keluar", danger: true }))) return;
    await api.chatLogout().catch(toastError);
    chatMessages.set([]);
  }

  async function send() {
    const body = draft.trim();
    if (!body || !selected) return;
    // Offline pun tetap "dikirim": langsung tampil ✕ dengan tombol Kirim ulang.
    const push = asAlert;
    draft = "";
    asAlert = false;
    await sendText(selected, body, push);
  }

  function onComposerKey(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  // ---------- Lampiran: dipilih (📎), Ctrl+V, atau diseret → antre dulu ----------
  // Tidak ada yang langsung terkirim: file masuk panel lampiran, bisa
  // dipratinjau & diberi keterangan (caption) per file, baru dikirim.
  type Staged = { id: string; name: string; bytes: Uint8Array; caption: string; thumb: string };
  let staged = $state<Staged[]>([]);
  let stagedIdx = $state(0);
  let dragOver = $state(false);
  let captionInput = $state<HTMLInputElement | null>(null);
  const stagedCur = $derived(staged[stagedIdx] ?? null);
  const canAttach = $derived(configured && onScreen && !!selected && !recording);

  function addFiles(files: { name: string; bytes: Uint8Array }[]) {
    if (!selected) return showToast("Pilih chat dulu sebelum melampirkan file.", "error");
    const errors: string[] = [];
    const before = staged.length;
    for (const f of files) {
      const bad = rejectReason(f.name, f.bytes.length);
      if (bad) {
        errors.push(bad);
        continue;
      }
      const thumb = isImagePreview(f.name) ? URL.createObjectURL(new Blob([f.bytes as BlobPart], { type: mimeOf(f.name) })) : "";
      staged.push({ id: crypto.randomUUID(), name: f.name, bytes: f.bytes, caption: "", thumb });
    }
    if (errors.length) showToast(errors.join("\n"), "error", 6000);
    if (staged.length > before) {
      stagedIdx = before;
      tick().then(() => captionInput?.focus());
    }
  }

  async function fromFileList(list: FileList | File[]) {
    const out: { name: string; bytes: Uint8Array }[] = [];
    for (const f of Array.from(list)) {
      // Gambar dari clipboard (screenshot) selalu bernama "image.png".
      const name = /^image\.(png|jpe?g|gif|bmp|webp)$/i.test(f.name) ? screenshotName(f.type || "image/png") : f.name;
      out.push({ name, bytes: new Uint8Array(await f.arrayBuffer()) });
    }
    addFiles(out);
  }

  async function addDropped(paths: string[]) {
    const out: { name: string; bytes: Uint8Array }[] = [];
    const errors: string[] = [];
    for (const path of paths) {
      const name = path.split(/[\\/]/).pop() || "file";
      try {
        out.push({ name, bytes: new Uint8Array(await api.chatReadDropped(path)) });
      } catch (e) {
        errors.push(`${name}: ${e instanceof Error ? e.message : String(e)}`);
      }
    }
    if (errors.length) showToast(errors.join("\n"), "error", 6000);
    if (out.length) addFiles(out);
  }

  function onFilePicked(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const files = input.files ? Array.from(input.files) : [];
    input.value = "";
    if (files.length) fromFileList(files);
  }

  // Ctrl+V: screenshot / file yang di-copy di Explorer → panel lampiran.
  // Copy sel Excel menaruh teks DAN gambar di clipboard — kalau ada teks dan
  // yang ada cuma gambar "image.*", tempel sebagai teks biasa.
  function onPaste(e: ClipboardEvent) {
    if (!canAttach) return;
    const files = Array.from(e.clipboardData?.files ?? []);
    if (!files.length) return;
    const text = e.clipboardData?.getData("text/plain") ?? "";
    if (text.trim() && files.every((f) => /^image\./i.test(f.name))) return;
    e.preventDefault();
    fromFileList(files);
  }

  function removeStaged(i: number) {
    const [gone] = staged.splice(i, 1);
    if (gone?.thumb) URL.revokeObjectURL(gone.thumb);
    stagedIdx = Math.min(stagedIdx, Math.max(0, staged.length - 1));
  }

  function clearStaged() {
    for (const f of staged) if (f.thumb) URL.revokeObjectURL(f.thumb);
    staged = [];
    stagedIdx = 0;
  }

  async function sendStaged() {
    if (!selected || !staged.length) return;
    const to = selected;
    const push = asAlert;
    const items = staged.map((f) => ({ name: f.name, bytes: f.bytes, caption: f.caption }));
    clearStaged();
    asAlert = false;
    for (const f of items) await sendFile(to, f.name, f.bytes, push, f.caption);
  }

  function onCaptionKey(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendStaged();
    } else if (e.key === "Escape") {
      e.preventDefault();
      clearStaged();
    }
  }

  // Pindah kontak = lampiran yang belum dikirim dibuang.
  let stagedFor = "";
  $effect(() => {
    if (selected !== stagedFor) {
      stagedFor = selected;
      if (staged.length) clearStaged();
    }
  });

  // ---------- Pratinjau & download file di gelembung ----------
  type PreviewFile = { name: string; size: number | null; path: string | null; bytes: Uint8Array | null };
  let preview = $state<PreviewFile | null>(null);

  async function download(path: string) {
    try {
      const dest = await api.chatDownloadFile(path);
      showToast(`Tersimpan di Downloads: ${dest.split(/[\\/]/).pop()}`, "success", 4000);
    } catch (e) {
      toastError(e);
    }
  }

  // Thumbnail gambar di gelembung, dimuat saat gelembungnya terlihat.
  const thumbCache = new Map<string, string>();
  let thumbs = $state<Record<string, string>>({});

  function lazyThumb(node: HTMLElement, path: string) {
    if (thumbCache.has(path)) {
      thumbs[path] = thumbCache.get(path)!;
      return;
    }
    const io = new IntersectionObserver((entries) => {
      if (!entries.some((en) => en.isIntersecting)) return;
      io.disconnect();
      api
        .chatReadFile(path)
        .then((buf) => {
          const u = URL.createObjectURL(new Blob([buf], { type: mimeOf(path) }));
          thumbCache.set(path, u);
          thumbs[path] = u;
        })
        .catch(() => {});
    });
    io.observe(node);
    return { destroy: () => io.disconnect() };
  }

  async function lookup() {
    addFound = false;
    try {
      const r = await api.chatLookup(addCode);
      addCode = r.code;
      addName = addName || r.name;
      addFound = true;
    } catch (e) {
      toastError(e);
    }
  }

  async function saveContact(code: string, name: string) {
    try {
      chatContacts.set(await api.chatSaveContact(code, name));
      return true;
    } catch (e) {
      toastError(e);
      return false;
    }
  }

  async function addContact() {
    if (await saveContact(addCode, addName)) {
      selected = addCode;
      addOpen = false;
      addCode = addName = "";
      addFound = false;
    }
  }

  async function renameCurrent() {
    if (current && (await saveContact(current.code, renameValue))) renaming = false;
  }

  async function deleteCurrent() {
    if (!current || !(await confirmDialog(`Hapus kontak ${current.name}? Riwayat pesan tetap ada.`, { title: "Hapus Kontak?", okText: "Hapus", danger: true }))) return;
    chatContacts.set(await api.chatDeleteContact(current.code).catch((e) => (toastError(e), $chatContacts)));
  }

  function fmtTime(iso: string): string {
    return new Date(iso).toLocaleTimeString("id-ID", { hour: "2-digit", minute: "2-digit" });
  }

  function dayLabel(iso: string): string {
    const d = new Date(iso);
    const today = new Date();
    const yesterday = new Date(today.getTime() - 86_400_000);
    if (d.toDateString() === today.toDateString()) return "Hari ini";
    if (d.toDateString() === yesterday.toDateString()) return "Kemarin";
    return d.toLocaleDateString("id-ID", { weekday: "long", day: "numeric", month: "long" });
  }

  /** Jam di daftar chat: jam kalau hari ini, selain itu tanggal. */
  function listTime(iso: string): string {
    if (!iso) return "";
    const d = new Date(iso);
    return d.toDateString() === new Date().toDateString()
      ? fmtTime(iso)
      : d.toLocaleDateString("id-ID", { day: "2-digit", month: "2-digit" });
  }

  function initials(name: string): string {
    const parts = name.replace(/[^\p{L}\p{N} ]/gu, " ").trim().split(/\s+/);
    return ((parts[0]?.[0] ?? "?") + (parts[1]?.[0] ?? "")).toUpperCase();
  }

  /** Warna avatar tetap per kode toko. */
  function avatarHue(code: string): number {
    let h = 0;
    for (const c of code) h = (h * 31 + c.charCodeAt(0)) % 360;
    return h;
  }

</script>

<svelte:window onpaste={onPaste} />

{#snippet fileCard(name: string, size: number | null, path: string | null, bytes: Uint8Array | null, mine: boolean)}
  {@const available = !!(path || bytes)}
  {@const isImg = isImagePreview(name) && !!path}
  <div class="file-box" class:unavailable={!available}>
    {#if isImg && path}
      <button class="img-thumb" use:lazyThumb={path} onclick={() => (preview = { name, size, path, bytes: null })}
        title="Lihat {name}">
        {#if thumbs[path]}<img src={thumbs[path]} alt={name} />{:else}<span class="thumb-wait">🖼</span>{/if}
      </button>
    {/if}
    <div class="file-row">
      <button class="file-card" disabled={!available} onclick={() => (preview = { name, size, path, bytes })}
        title={available ? `Lihat ${name}` : ""}>
        <span class="kind-icon k-{fileKind(name)}">{kindLabel(name)}</span>
        <span class="file-meta">
          <span class="file-name">{name}</span>
          <span class="file-size">
            {#if available}{fmtSize(size)}{:else if mine}File hanya ada di PC pengirim{:else}File tidak tersedia di PC ini{/if}
          </span>
        </span>
      </button>
      {#if path}
        <button class="dl-btn" title="Download ke folder Downloads" aria-label="Download {name}"
          onclick={() => download(path)}>
          <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
            <path d="M12 4v11m0 0-4.5-4.5M12 15l4.5-4.5M5 19.5h14" fill="none" stroke="currentColor"
              stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
      {/if}
    </div>
    {#if !available}
      <div class="note">PC mati saat file dikirim, atau file sudah lewat 14 hari.</div>
    {/if}
  </div>
{/snippet}

<div class="chat">
  {#if !configured || showSetup}
    <div class="card setup">
      <h2 style="margin-top:0;">Chat Toko</h2>
      <p class="text-dim" style="font-size:0.83rem;">
        <b>Kode toko</b> ibarat nomor HP (boleh diberitahukan ke toko lain). <b>Kunci Chat</b> ibarat
        kartu SIM — didapat dari <span class="mono">jjapps.net/admin → Toko Chat</span>, cukup ditempel sekali
        di setiap PC toko ini.
      </p>
      <div class="field">
        <label for="c-code">Kode toko ini</label>
        <input id="c-code" class="mono" bind:value={setupCode} placeholder={suggestedCode || "toko-001"} />
      </div>
      <div class="field">
        <label for="c-key">Kunci Chat</label>
        <input id="c-key" class="mono" bind:value={setupKey} placeholder="XXXX-XXXX-XXXX-XXXX" autocomplete="off" />
      </div>
      <div class="row" style="gap:0.5rem;">
        <button class="btn-primary" disabled={busy || !setupCode.trim() || !setupKey.trim()} onclick={saveSetup}>
          Simpan & Sambungkan
        </button>
        {#if configured}<button class="btn-ghost" onclick={() => (showSetup = false)}>Batal</button>{/if}
      </div>
    </div>
  {:else}
    {#if !online}
      <div class="offline-bar">
        {#if keyProblem}
          🔑 <b>Chat terputus:</b> {$chatStatus?.error}
          <button class="btn-primary" onclick={() => (showSetup = true)}>Masukkan kunci baru</button>
        {:else}
          ⚠️ <b>PC ini offline</b>{$chatStatus?.error ? ` — ${$chatStatus.error}` : " — menyambung…"}.
          Pesan yang dikirim langsung ditandai ✕ dan harus dikirim ulang manual.
        {/if}
      </div>
    {/if}
    <div class="app-shell">
      <!-- ===== Kolom kiri: daftar chat ===== -->
      <aside class="side">
        <div class="side-head">
          <div class="avatar" style="--h:{avatarHue(me)}">{initials($chatStatus?.name || me)}</div>
          <div class="me-info">
            <div class="me-name">{$chatStatus?.name || me}</div>
            <div class="me-sub">
              <span class="dot" class:on={online}></span>{online ? "Online" : "Offline"} · <span class="mono">{me}</span>
            </div>
          </div>
          <button class="icon-btn" title="Chat baru / tambah kontak" onclick={() => (addOpen = !addOpen)}>＋</button>
          <button class="icon-btn" title="Ganti Kunci Chat" onclick={() => (showSetup = true)}>🔑</button>
          <button class="icon-btn" title="Keluar dari chat" onclick={logout}>⎋</button>
        </div>

        {#if addOpen}
          <div class="add">
            <input class="mono" placeholder="Kode toko, mis. toko-002" bind:value={addCode}
              onkeydown={(e) => e.key === "Enter" && lookup()} />
            {#if addFound}
              <input placeholder="Simpan sebagai…" bind:value={addName}
                onkeydown={(e) => e.key === "Enter" && addContact()} />
              <button class="btn-primary" disabled={!addName.trim()} onclick={addContact}>Simpan</button>
            {:else}
              <button onclick={lookup} disabled={!addCode.trim()}>Cari</button>
            {/if}
          </div>
        {/if}

        <div class="chat-list">
          {#each peers as p (p.code)}
            <button class="chat-row" class:active={p.code === selected} onclick={() => { selected = p.code; renaming = false; }}>
              <div class="avatar" style="--h:{avatarHue(p.code)}">{initials(p.name)}</div>
              <div class="row-main">
                <div class="row-top">
                  <span class="row-name">{p.name}</span>
                  <span class="row-time" class:hot={p.unread > 0}>{listTime(p.lastAt)}</span>
                </div>
                <div class="row-bottom">
                  <span class="row-preview">
                    {#if !p.saved}<span class="new-tag">belum disimpan</span>{/if}
                    {#if p.lastText}{p.lastMine ? "Anda: " : ""}{p.lastText}{:else}<span class="mono">{p.code}</span>{/if}
                  </span>
                  {#if p.unread > 0}<span class="badge-unread">{p.unread}</span>{/if}
                </div>
              </div>
            </button>
          {:else}
            <p class="empty">Belum ada chat. Klik ＋ lalu masukkan kode toko lain.</p>
          {/each}
        </div>
      </aside>

      <!-- ===== Kolom kanan: percakapan ===== -->
      <section class="conv">
        {#if !current}
          <div class="conv-empty">
            <div class="big">💬</div>
            <div>Pilih chat di sebelah kiri untuk mulai mengobrol.</div>
          </div>
        {:else}
          <header class="conv-head">
            <div class="avatar" style="--h:{avatarHue(current.code)}">{initials(current.name)}</div>
            {#if renaming}
              <input class="rename" bind:value={renameValue} onkeydown={(e) => e.key === "Enter" && renameCurrent()} />
              <button class="btn-primary" onclick={renameCurrent}>Simpan</button>
              <button class="btn-ghost" onclick={() => (renaming = false)}>Batal</button>
            {:else}
              <div class="conv-title">
                <div class="conv-name">{current.name}</div>
                <div class="conv-sub mono">{current.code}{current.saved ? "" : " · belum disimpan"}</div>
              </div>
              <button class="icon-btn" title={current.saved ? "Ubah nama kontak" : "Simpan kontak"}
                onclick={() => { renameValue = current?.name ?? ""; renaming = true; }}>{current.saved ? "✏️" : "💾"}</button>
              {#if current.saved}<button class="icon-btn" title="Hapus kontak" onclick={deleteCurrent}>🗑️</button>{/if}
              <button class="icon-btn call-icon" title="Voice call {current.name}"
                disabled={!$chatStatus?.connected || ($call.phase !== "idle" && $call.phase !== "ended")}
                onclick={() => current && startCall(current.code)}>📞</button>
            {/if}
          </header>

          {#if staged.length && stagedCur}
            <!-- ===== Panel lampiran: pratinjau + caption sebelum dikirim ===== -->
            <div class="stage">
              <div class="stage-top">
                <button class="icon-btn" title="Batal (Esc)" onclick={clearStaged}>✕</button>
                <span class="stage-name">{stagedCur.name}</span>
                <span class="stage-size">{fmtSize(stagedCur.bytes.length)}</span>
              </div>
              <div class="stage-preview">
                {#key stagedCur.id}
                  <FilePreview inline name={stagedCur.name} bytes={stagedCur.bytes} />
                {/key}
              </div>
              <div class="stage-caption">
                <div class="input-wrap" class:alerting={asAlert}>
                  <input bind:this={captionInput} bind:value={stagedCur.caption} onkeydown={onCaptionKey}
                    placeholder="Tambahkan keterangan…" maxlength="2000" />
                  <button class="alert-btn" class:on={asAlert} onclick={() => (asAlert = !asAlert)}
                    title="Kirim sebagai alert: muncul sebagai popup di GPOS toko tujuan">🔔</button>
                </div>
              </div>
              <div class="stage-bottom">
                <div class="stage-thumbs">
                  {#each staged as f, i (f.id)}
                    <div class="st-thumb" class:on={i === stagedIdx}>
                      <button class="st-pick" onclick={() => (stagedIdx = i)} title={f.name}>
                        {#if f.thumb}<img src={f.thumb} alt={f.name} />{:else}<span class="kind-icon k-{fileKind(f.name)}">{kindLabel(f.name)}</span>{/if}
                      </button>
                      <button class="st-x" title="Buang {f.name}" onclick={() => removeStaged(i)}>✕</button>
                    </div>
                  {/each}
                  <button class="st-add" title="Tambah file" onclick={() => fileInput?.click()}>＋</button>
                </div>
                <button class="send-btn" onclick={sendStaged}
                  title="Kirim {staged.length} file (Enter)">➤{#if staged.length > 1}<span class="send-count">{staged.length}</span>{/if}</button>
              </div>
            </div>
          {:else}
          <div class="messages" bind:this={scroller}>
            {#each items as it (it.key)}
              {#if it.day}<div class="day-sep"><span>{it.day}</span></div>{/if}
              <div class="msg-row" class:mine={it.mine}>
                {#if it.msg}
                  {@const m = it.msg}
                  <div class="bubble" class:mine={it.mine} class:alert={m.push}>
                    {#if m.push}<div class="alert-tag">🔔 Alert</div>{/if}
                    {#if m.kind === "call"}
                      <span class="body call-log" class:missed={m.body === "missed" && !it.mine}>{callLabel(m.body)}</span>
                    {:else if m.kind === "voice"}
                      {#if m.local_path}
                        <VoicePlayer path={m.local_path} duration={Number(m.body) || 0} mine={it.mine} />
                      {:else}
                        <div class="note">🎤 Voice note {fmtDuration(Number(m.body) || 0)} — tidak tersimpan di PC ini.</div>
                      {/if}
                    {:else if m.kind === "file"}
                      {@render fileCard(m.file_name ?? "file", m.file_size, m.local_path ?? null, null, it.mine)}
                      {#if m.body}<div class="body caption">{m.body}</div>{/if}
                    {:else}
                      <span class="body">{m.body}</span>
                    {/if}
                    <span class="meta">
                      {fmtTime(m.created_at)}
                      {#if it.mine}
                        {#if m.read_at}
                          <span class="ticks read" title="Dibaca">✓✓</span>
                        {:else if m.delivered_at}
                          <span class="ticks" title="Diterima">✓✓</span>
                        {:else}
                          <span class="ticks" title="Terkirim, belum diterima (toko tujuan offline)">✓</span>
                        {/if}
                      {/if}
                    </span>
                  </div>
                {:else if it.out}
                  {@const x = it.out}
                  <div class="bubble mine" class:failed={x.status === "failed"} class:alert={x.push}>
                    {#if x.push}<div class="alert-tag">🔔 Alert</div>{/if}
                    {#if x.voice}
                      <VoicePlayer bytes={x.voice.bytes} duration={x.voice.duration} mine />
                    {:else if x.file}
                      {@render fileCard(x.file.name, x.file.size, null, x.file.bytes, true)}
                      {#if x.body}<div class="body caption">{x.body}</div>{/if}
                    {:else}
                      <span class="body">{x.body}</span>
                    {/if}
                    <span class="meta">
                      {fmtTime(x.created_at)}
                      {#if x.status === "failed"}<span class="ticks fail" title={x.error ?? "Tidak terkirim"}>✕</span>{/if}
                    </span>
                    {#if x.status === "failed"}
                      <div class="fail-row">
                        <span>Tidak terkirim{x.error ? ` — ${x.error}` : ""}</span>
                        <button onclick={() => retryOut(x.client_id)} disabled={!online}
                          title={online ? "Kirim ulang" : "Tunggu PC tersambung lagi"}>↻ Kirim ulang</button>
                        <button class="btn-ghost" onclick={() => discardOut(x.client_id)}>Hapus</button>
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>
            {:else}
              <div class="conv-empty small">Belum ada pesan 14 hari terakhir. Sapa duluan! 👋</div>
            {/each}
          </div>

          <footer class="composer">
            {#if recording}
              <button class="icon-btn attach" onclick={() => stopRecording(false)} title="Batalkan rekaman">🗑️</button>
              <div class="rec-bar" class:alerting={recPush}>
                <span class="rec-dot"></span>
                <span class="rec-time">{fmtDuration(recSeconds)}</span>
                <span class="rec-label">Merekam{recPush ? " alert" : ""}… maks 2:00</span>
              </div>
              <button class="send-btn" onclick={() => stopRecording(true)} title="Kirim voice note">➤</button>
            {:else}
              <button class="icon-btn attach" onclick={() => fileInput?.click()}
                title="Lampirkan gambar, Word, Excel, PDF — maks 10 MB. Bisa juga Ctrl+V atau seret file ke sini.">📎</button>
              <div class="input-wrap" class:alerting={asAlert}>
                <textarea
                  rows="1"
                  placeholder={asAlert ? "Tulis alert… (muncul sebagai popup di toko tujuan)" : "Ketik pesan"}
                  bind:value={draft}
                  onkeydown={onComposerKey}
                ></textarea>
                <button class="alert-btn" class:on={asAlert} onclick={() => (asAlert = !asAlert)}
                  title="Kirim sebagai alert: muncul sebagai popup di GPOS toko tujuan">🔔</button>
              </div>
              {#if draft.trim()}
                <button class="send-btn" onclick={send} title="Kirim (Enter)">➤</button>
              {:else}
                <button class="send-btn" onclick={startRecording} title="Rekam voice note">🎤</button>
              {/if}
            {/if}
          </footer>
          {/if}
          <input type="file" accept={CHAT_ACCEPT} multiple bind:this={fileInput} onchange={onFilePicked} hidden />
          {#if dragOver}
            <div class="drop-overlay">
              <div class="drop-box">📎 Lepaskan file untuk dilampirkan ke <b>{current.name}</b></div>
            </div>
          {/if}
        {/if}
      </section>
    </div>
  {/if}
  {#if preview}
    {@const pv = preview}
    <FilePreview name={pv.name} size={pv.size} path={pv.path} bytes={pv.bytes} onclose={() => (preview = null)}
      ondownload={pv.path ? () => download(pv.path!) : null} />
  {/if}
</div>

<style>
  .chat {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    /* Semua warna chat diturunkan dari variabel tema di app.css, jadi ikut
       Baby Blue / Dark / Forest / Sunset / Grape / Kontras Tinggi. Tata
       letak tetap seperti WhatsApp: kita di kanan (diwarnai warna tema),
       lawan bicara di kiri (warna panel). */
    color: var(--text);
    --c-panel: var(--panel);
    --c-head: var(--baby-blue-bg);
    --c-hover: color-mix(in srgb, var(--text) 5%, var(--panel));
    --c-active: var(--baby-blue-soft);
    --c-soft: color-mix(in srgb, var(--text) 8%, transparent);
    /* Varian gelap warna tema supaya teks putih di atasnya tetap terbaca
       (mis. oranye Sunset); Dark Mode memakai --primary, lihat di bawah. */
    --c-accent: var(--primary-dark);
    /* --white = warna panel tema: putih di tema terang, gelap di Dark Mode —
       selalu kontras di atas warna aksen. */
    --c-on-accent: var(--white);
    --c-warn-soft: color-mix(in srgb, var(--warning) 22%, var(--panel));
    --c-danger-soft: color-mix(in srgb, var(--danger) 14%, var(--panel));
    --c-bubble-shadow: 0 1px 0.5px color-mix(in srgb, var(--text) 20%, transparent);
    --c-read: #34b7f1;
    --bubble-mine: color-mix(in srgb, var(--primary) 22%, var(--panel));
    --bubble-theirs: var(--panel);
    --conv-bg: color-mix(in srgb, var(--primary) 6%, var(--bg));
    /* --text-dim tema dipekatkan sedikit: jam & cuplikan berukuran kecil. */
    --muted: color-mix(in srgb, var(--text-dim) 55%, var(--text));
  }
  /* Dark Mode: teks di atas aksen memakai --white (gelap), jadi aksennya
     justru harus yang terang. */
  :global(:root[data-theme="dark"]) .chat {
    --c-accent: var(--primary);
  }
  .setup {
    max-width: 460px;
  }
  .field {
    margin-bottom: 0.7rem;
  }
  .field label {
    display: block;
    font-size: 0.8rem;
    margin-bottom: 0.2rem;
  }
  .app-shell {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 320px 1fr;
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
    background: var(--c-panel);
  }

  /* ----- Kiri ----- */
  .side {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--border);
    background: var(--c-panel);
  }
  .side-head,
  .conv-head {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.55rem 0.8rem;
    background: var(--c-head);
    min-height: 56px;
  }
  .me-info,
  .conv-title {
    flex: 1;
    min-width: 0;
  }
  .me-name,
  .conv-name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .me-sub,
  .conv-sub {
    font-size: 0.72rem;
    color: var(--muted);
  }
  .dot {
    display: inline-block;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-dim);
    margin-right: 4px;
    vertical-align: middle;
  }
  .dot.on {
    background: var(--success);
  }
  .icon-btn {
    border: none;
    background: transparent;
    font-size: 1.05rem;
    padding: 0.3rem 0.45rem;
    border-radius: 50%;
    cursor: pointer;
    line-height: 1;
  }
  .icon-btn:hover:not(:disabled) {
    background: var(--c-soft);
  }
  .icon-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .call-icon {
    margin-left: auto;
  }
  .call-log {
    font-weight: 600;
  }
  .call-log.missed {
    color: var(--danger);
  }
  .add {
    display: flex;
    gap: 0.4rem;
    padding: 0.6rem 0.8rem;
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
  }
  .add input {
    flex: 1;
    min-width: 120px;
  }
  .chat-list {
    flex: 1;
    overflow-y: auto;
  }
  .chat-row {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    width: 100%;
    padding: 0.6rem 0.8rem;
    border: none;
    border-bottom: 1px solid var(--border);
    border-radius: 0;
    background: var(--c-panel);
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }
  .chat-row:hover {
    background: var(--c-hover);
  }
  .chat-row.active {
    background: var(--c-active);
  }
  .row-main {
    flex: 1;
    min-width: 0;
  }
  .row-top,
  .row-bottom {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .row-name {
    flex: 1;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-time {
    font-size: 0.7rem;
    color: var(--muted);
  }
  .row-time.hot {
    color: var(--success);
    font-weight: 600;
  }
  .row-preview {
    flex: 1;
    font-size: 0.8rem;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .new-tag {
    font-size: 0.65rem;
    background: var(--c-warn-soft);
    color: var(--text);
    border-radius: 4px;
    padding: 0 0.3rem;
    margin-right: 0.3rem;
  }
  .badge-unread {
    background: var(--c-accent);
    color: var(--c-on-accent);
    border-radius: 999px;
    font-size: 0.7rem;
    font-weight: 700;
    min-width: 20px;
    text-align: center;
    padding: 0.08rem 0.4rem;
  }
  .empty {
    padding: 1rem;
    font-size: 0.82rem;
    color: var(--muted);
  }
  .avatar {
    width: 40px;
    height: 40px;
    flex: 0 0 40px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-weight: 700;
    font-size: 0.85rem;
    color: #fff;
    background: hsl(var(--h, 200) 45% 50%);
  }

  /* ----- Kanan ----- */
  .conv {
    position: relative;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--conv-bg);
  }
  .conv-empty {
    margin: auto;
    text-align: center;
    color: var(--muted);
    padding: 2rem;
  }
  .conv-empty .big {
    font-size: 3rem;
    margin-bottom: 0.5rem;
  }
  .conv-empty.small {
    background: var(--c-panel);
    border-radius: 8px;
    padding: 0.5rem 0.9rem;
    font-size: 0.82rem;
  }
  .rename {
    flex: 1;
  }
  .offline-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    margin-bottom: 0.5rem;
    border-radius: 8px;
    background: var(--c-danger-soft);
    color: color-mix(in srgb, var(--danger) 65%, var(--text));
    padding: 0.45rem 0.8rem;
    font-size: 0.82rem;
    border-bottom: 1px solid color-mix(in srgb, var(--danger) 35%, transparent);
  }
  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 0.8rem 6%;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .day-sep {
    display: flex;
    justify-content: center;
    margin: 0.7rem 0 0.4rem;
  }
  .day-sep span {
    background: var(--c-panel);
    color: var(--muted);
    font-size: 0.72rem;
    padding: 0.2rem 0.7rem;
    border-radius: 7px;
    box-shadow: var(--c-bubble-shadow);
  }
  /* Seperti WhatsApp: kita di KANAN, lawan bicara di KIRI. */
  .msg-row {
    display: flex;
    justify-content: flex-start;
    margin: 1px 0;
  }
  .msg-row.mine {
    justify-content: flex-end;
  }
  .bubble {
    position: relative;
    max-width: 65%;
    padding: 0.35rem 0.55rem 0.3rem;
    border-radius: 8px;
    background: var(--bubble-theirs);
    color: var(--text);
    box-shadow: var(--c-bubble-shadow);
    font-size: 0.9rem;
    line-height: 1.35;
  }
  .bubble:not(.mine) {
    border-top-left-radius: 0;
  }
  .bubble.mine {
    background: var(--bubble-mine);
    border-top-right-radius: 0;
  }
  .bubble.alert {
    box-shadow: 0 0 0 2px var(--warning) inset, var(--c-bubble-shadow);
  }
  .bubble.failed {
    box-shadow: 0 0 0 1.5px var(--danger) inset;
  }
  .body {
    white-space: pre-wrap;
    word-break: break-word;
  }
  /* Jam & centang menempel di pojok kanan bawah gelembung, seperti WA. */
  .meta {
    float: right;
    margin: 0.35rem 0 -0.1rem 0.6rem;
    font-size: 0.66rem;
    color: var(--muted);
    white-space: nowrap;
  }
  .ticks {
    margin-left: 0.2rem;
    letter-spacing: -0.25em;
    font-weight: 700;
  }
  .ticks.read {
    color: var(--c-read);
  }
  .ticks.fail {
    color: var(--danger);
    letter-spacing: 0;
  }
  .alert-tag {
    font-size: 0.7rem;
    font-weight: 700;
    color: var(--warning);
    margin-bottom: 0.1rem;
  }
  /* ----- Kartu file: klik = pratinjau, tombol ⬇ di samping = download ----- */
  .file-box {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    min-width: 240px;
    max-width: 340px;
  }
  .img-thumb {
    display: block;
    padding: 0;
    border: none;
    border-radius: 6px;
    overflow: hidden;
    background: var(--c-soft);
    cursor: zoom-in;
    min-height: 120px;
  }
  .img-thumb img {
    display: block;
    width: 100%;
    max-height: 260px;
    object-fit: cover;
  }
  .thumb-wait {
    display: grid;
    place-items: center;
    height: 120px;
    font-size: 1.8rem;
    opacity: 0.4;
  }
  .file-row {
    display: flex;
    align-items: stretch;
    gap: 0.35rem;
  }
  .file-card {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    background: var(--c-soft);
    color: var(--text);
    border: none;
    border-radius: 8px;
    padding: 0.5rem 0.6rem;
    text-align: left;
    cursor: pointer;
    font: inherit;
  }
  .file-card:hover:not(:disabled) {
    background: color-mix(in srgb, var(--text) 13%, transparent);
  }
  .file-card:disabled {
    cursor: default;
  }
  .unavailable .file-card {
    opacity: 0.6;
  }
  .kind-icon {
    flex: none;
    width: 34px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: 6px;
    font-size: 0.62rem;
    font-weight: 800;
    letter-spacing: 0.02em;
    color: #fff;
    background: var(--k, #64748b);
  }
  .k-pdf {
    --k: #d93025;
  }
  .k-word {
    --k: #2b579a;
  }
  .k-excel {
    --k: #217346;
  }
  .k-image {
    --k: #8e44ad;
  }
  .dl-btn {
    flex: none;
    width: 40px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: 8px;
    background: var(--c-soft);
    color: var(--text);
    cursor: pointer;
  }
  .dl-btn:hover {
    background: var(--c-accent);
    color: var(--c-on-accent);
  }
  .file-meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .file-name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file-size,
  .note {
    font-size: 0.72rem;
    color: var(--muted);
  }
  .caption {
    display: block;
    margin-top: 0.3rem;
  }

  /* ----- Panel lampiran sebelum kirim ----- */
  .stage {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--conv-bg);
  }
  .stage-top {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.4rem 0.8rem;
  }
  .stage-name {
    flex: 1;
    min-width: 0;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .stage-size {
    font-size: 0.75rem;
    color: var(--muted);
  }
  .stage-preview {
    flex: 1;
    min-height: 0;
    display: flex;
    margin: 0 0.8rem;
    border-radius: 8px;
    overflow: hidden;
    background: var(--c-panel);
    box-shadow: var(--c-bubble-shadow);
  }
  .stage-caption {
    padding: 0.6rem 15% 0.3rem;
  }
  .stage-caption input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    padding: 0.4rem 0;
    font: inherit;
  }
  .stage-bottom {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0.8rem;
    background: var(--c-head);
  }
  .stage-thumbs {
    flex: 1;
    display: flex;
    gap: 0.45rem;
    overflow-x: auto;
    justify-content: center;
    padding: 0.2rem;
  }
  .st-thumb {
    position: relative;
    flex: none;
  }
  .st-pick {
    width: 52px;
    height: 52px;
    padding: 0;
    display: grid;
    place-items: center;
    border-radius: 8px;
    overflow: hidden;
    border: 2px solid transparent;
    background: var(--c-panel);
    cursor: pointer;
  }
  .st-thumb.on .st-pick {
    border-color: var(--c-accent);
  }
  .st-pick img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .st-x {
    position: absolute;
    top: -6px;
    right: -6px;
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    border-radius: 50%;
    font-size: 0.6rem;
    line-height: 18px;
    background: var(--text);
    color: var(--panel);
    cursor: pointer;
    opacity: 0;
  }
  .st-thumb:hover .st-x,
  .st-thumb.on .st-x {
    opacity: 1;
  }
  .st-add {
    flex: none;
    width: 52px;
    height: 52px;
    border-radius: 8px;
    border: 2px dashed var(--border);
    background: transparent;
    color: var(--muted);
    font-size: 1.3rem;
    cursor: pointer;
  }
  .send-btn {
    position: relative;
  }
  .send-count {
    position: absolute;
    top: -4px;
    right: -4px;
    min-width: 18px;
    height: 18px;
    border-radius: 999px;
    font-size: 0.65rem;
    font-weight: 700;
    line-height: 18px;
    background: var(--danger);
    color: #fff;
  }

  /* ----- Seret file ke percakapan ----- */
  .drop-overlay {
    position: absolute;
    inset: 0;
    z-index: 5;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--c-panel) 82%, transparent);
    pointer-events: none;
  }
  .drop-box {
    padding: 2.2rem 3rem;
    border: 3px dashed var(--c-accent);
    border-radius: 14px;
    font-size: 1.05rem;
    background: var(--c-panel);
  }

  .fail-row button {
    font-size: 0.75rem;
    padding: 0.15rem 0.55rem;
  }
  .fail-row {
    clear: both;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
    padding-top: 0.3rem;
    font-size: 0.72rem;
    color: var(--danger);
  }

  /* ----- Kolom ketik ----- */
  .composer {
    display: flex;
    align-items: flex-end;
    gap: 0.5rem;
    padding: 0.5rem 0.8rem;
    background: var(--c-head);
  }
  .input-wrap {
    flex: 1;
    display: flex;
    align-items: flex-end;
    background: var(--c-panel);
    border-radius: 20px;
    padding: 0.3rem 0.4rem 0.3rem 0.9rem;
    border: 2px solid transparent;
  }
  .input-wrap.alerting {
    border-color: var(--warning);
  }
  .input-wrap textarea {
    flex: 1;
    border: none;
    outline: none;
    resize: none;
    background: transparent;
    color: var(--text);
    /* app.css memberi semua textarea min-height 60px; kolom chat mulai 1 baris. */
    min-height: 0;
    max-height: 120px;
    field-sizing: content;
    padding: 0.35rem 0;
    font: inherit;
  }
  .alert-btn {
    border: none;
    background: transparent;
    font-size: 1rem;
    padding: 0.25rem 0.4rem;
    border-radius: 50%;
    cursor: pointer;
    filter: grayscale(1);
    opacity: 0.55;
  }
  .alert-btn.on {
    filter: none;
    opacity: 1;
    background: var(--c-warn-soft);
  }
  .attach {
    align-self: center;
  }
  .send-btn {
    width: 42px;
    height: 42px;
    flex: 0 0 42px;
    border-radius: 50%;
    border: none;
    background: var(--c-accent);
    color: var(--c-on-accent);
    font-size: 1.05rem;
    cursor: pointer;
  }
  .rec-bar {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    background: var(--c-panel);
    border-radius: 20px;
    padding: 0.55rem 0.9rem;
    border: 2px solid transparent;
  }
  .rec-bar.alerting {
    border-color: var(--warning);
  }
  .rec-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--danger);
    animation: rec-blink 1s infinite;
  }
  @keyframes rec-blink {
    50% {
      opacity: 0.25;
    }
  }
  .rec-time {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  .rec-label {
    font-size: 0.8rem;
    color: var(--muted);
  }
  .send-btn:disabled {
    background: var(--text-dim);
    cursor: default;
  }
</style>
