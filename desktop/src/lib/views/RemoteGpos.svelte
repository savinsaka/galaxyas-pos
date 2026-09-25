<script lang="ts">
  // Remote GPOS (eksperimental) — seperti TeamViewer, tapi hanya jendela GPOS.
  // Dua peran: "Terima" (PC ini diremote) dan "Kontrol" (PC ini meremote PC
  // lain lewat ID + OTP). Semua lewat relay; lihat src-tauri/src/remote_gpos.rs.
  import { onMount, onDestroy } from "svelte";
  import { Channel } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import type { RemoteHostStatus, RemoteViewerStatus, RemoteMeta } from "$lib/types";

  type Role = "terima" | "kontrol";
  const ROLE_KEY = "remote-gpos-role";

  let role = $state<Role>(readRole());
  let host = $state<RemoteHostStatus | null>(null);
  let viewer = $state<RemoteViewerStatus | null>(null);
  let busy = $state(false);

  let inputId = $state("");
  let inputOtp = $state("");
  let meta = $state<RemoteMeta | null>(null);
  let hasFrame = $state(false);

  let canvas = $state<HTMLCanvasElement | null>(null);
  let stage = $state<HTMLDivElement | null>(null);
  const unlisteners: UnlistenFn[] = [];

  function readRole(): Role {
    try {
      return localStorage.getItem(ROLE_KEY) === "kontrol" ? "kontrol" : "terima";
    } catch {
      return "terima";
    }
  }

  function setRole(r: Role) {
    role = r;
    try {
      localStorage.setItem(ROLE_KEY, r);
    } catch {
      /* abaikan */
    }
  }

  const fmtId = (id: string) => id.replace(/(\d{3})(?=\d)/g, "$1 ");
  const viewerLive = $derived(viewer?.phase === "connecting" || viewer?.phase === "connected");

  onMount(async () => {
    try {
      [host, viewer] = await api.remoteGposStatus();
    } catch (e) {
      toastError(e);
    }
    unlisteners.push(await listen<RemoteHostStatus>("remote-gpos://host", (e) => (host = e.payload)));
    unlisteners.push(
      await listen<RemoteViewerStatus>("remote-gpos://viewer", (e) => {
        viewer = e.payload;
        if (e.payload.phase === "closed") {
          meta = null;
          hasFrame = false;
          if (e.payload.error) showToast(e.payload.error, "error", 5000);
        }
      }),
    );
    unlisteners.push(await listen<RemoteMeta>("remote-gpos://meta", (e) => (meta = e.payload)));
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("keyup", onKey, true);
  });

  onDestroy(() => {
    unlisteners.forEach((u) => u());
    window.removeEventListener("keydown", onKey, true);
    window.removeEventListener("keyup", onKey, true);
    // Tab ditutup = sesi meremote selesai. Sesi "Terima" tetap jalan
    // (dikendalikan dari banner global).
    if (viewerLive) api.remoteGposDisconnect().catch(() => {});
  });

  // ---------- Terima ----------

  async function toggleHost(e: Event) {
    const enabled = (e.currentTarget as HTMLInputElement).checked;
    busy = true;
    try {
      host = await api.remoteGposSetHost(enabled);
    } catch (err) {
      toastError(err);
    } finally {
      busy = false;
    }
  }

  async function setControl(allow: boolean) {
    try {
      await api.remoteGposSetControl(allow);
      if (host) host = { ...host, allow_control: allow };
    } catch (err) {
      toastError(err);
    }
  }

  // ---------- Kontrol ----------

  async function connect() {
    const id = inputId.replace(/\D/g, "");
    const otp = inputOtp.replace(/\D/g, "");
    if (id.length !== 9) return showToast("ID remote harus 9 angka.", "error");
    if (otp.length !== 6) return showToast("OTP harus 6 angka.", "error");
    meta = null;
    hasFrame = false;
    const frames = new Channel<ArrayBuffer>();
    frames.onmessage = (buf) => void drawFrame(buf);
    try {
      await api.remoteGposConnect(id, otp, frames);
      inputOtp = "";
    } catch (err) {
      toastError(err);
    }
  }

  async function disconnect() {
    await api.remoteGposDisconnect().catch(() => {});
  }

  async function drawFrame(buf: ArrayBuffer | number[]) {
    const bytes = buf instanceof ArrayBuffer ? buf : new Uint8Array(buf);
    try {
      const bmp = await createImageBitmap(new Blob([bytes], { type: "image/jpeg" }));
      if (canvas) {
        if (canvas.width !== bmp.width || canvas.height !== bmp.height) {
          canvas.width = bmp.width;
          canvas.height = bmp.height;
        }
        canvas.getContext("2d")?.drawImage(bmp, 0, 0);
        hasFrame = true;
      }
      bmp.close();
    } catch {
      /* frame rusak: lewati */
    } finally {
      // Frame berikutnya baru dikirim host setelah ack ini — koneksi lambat
      // tidak menumpuk frame.
      send({ type: "ack" });
    }
  }

  function send(message: unknown) {
    api.remoteGposSend(message).catch(() => {});
  }

  const canControl = $derived(viewer?.phase === "connected" && (meta?.control ?? false));

  function modifiers(e: MouseEvent | KeyboardEvent): number {
    return (e.altKey ? 1 : 0) | (e.ctrlKey ? 2 : 0) | (e.metaKey ? 4 : 0) | (e.shiftKey ? 8 : 0);
  }

  /** Koordinat di kanvas → piksel CSS di GPOS tujuan. */
  function point(e: MouseEvent): { x: number; y: number } | null {
    if (!canvas || !meta) return null;
    const r = canvas.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) return null;
    return {
      x: Math.max(0, Math.min(meta.width, ((e.clientX - r.left) / r.width) * meta.width)),
      y: Math.max(0, Math.min(meta.height, ((e.clientY - r.top) / r.height) * meta.height)),
    };
  }

  const BUTTONS = ["left", "middle", "right"] as const;

  function mouse(type: string, e: MouseEvent, extra: Record<string, unknown> = {}) {
    if (!canControl) return;
    const p = point(e);
    if (!p) return;
    send({
      type: "mouse",
      params: { type, ...p, modifiers: modifiers(e), buttons: e.buttons, button: "none", ...extra },
    });
  }

  // Gerakan mouse diringkas per frame animasi supaya tidak membanjiri relay.
  let pendingMove: MouseEvent | null = null;
  function onMove(e: MouseEvent) {
    if (!canControl) return;
    if (pendingMove === null) {
      requestAnimationFrame(() => {
        if (pendingMove) mouse("mouseMoved", pendingMove);
        pendingMove = null;
      });
    }
    pendingMove = e;
  }

  function onDown(e: MouseEvent) {
    canvas?.focus();
    e.preventDefault();
    mouse("mousePressed", e, { button: BUTTONS[e.button] ?? "left", clickCount: e.detail || 1 });
  }

  function onUp(e: MouseEvent) {
    e.preventDefault();
    mouse("mouseReleased", e, { button: BUTTONS[e.button] ?? "left", clickCount: e.detail || 1 });
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    mouse("mouseWheel", e, { deltaX: e.deltaX, deltaY: e.deltaY });
  }

  // Keyboard ditangkap di fase capture pada window, supaya F1–F12 dan
  // shortcut lain tidak ikut memicu GPOS milik PC ini sendiri (KasirPOS dkk.
  // memasang keydown di window).
  function onKey(e: KeyboardEvent) {
    if (!canvas || document.activeElement !== canvas) return;
    e.preventDefault();
    e.stopImmediatePropagation();
    if (!canControl) return;
    const printable = e.key.length === 1 && !e.ctrlKey && !e.altKey && !e.metaKey;
    const text = printable ? e.key : e.key === "Enter" ? "\r" : undefined;
    const params: Record<string, unknown> = {
      type: e.type === "keyup" ? "keyUp" : text ? "keyDown" : "rawKeyDown",
      key: e.key,
      code: e.code,
      windowsVirtualKeyCode: e.keyCode,
      nativeVirtualKeyCode: e.keyCode,
      modifiers: modifiers(e),
      autoRepeat: e.repeat,
      location: e.location,
    };
    if (e.type === "keydown" && text) {
      params.text = text;
      params.unmodifiedText = text;
    }
    send({ type: "key", params });
  }

  // Svelte memasang onwheel sebagai listener pasif (preventDefault diabaikan),
  // padahal scroll di kanvas tidak boleh ikut menggulung halaman PC ini.
  $effect(() => {
    const el = canvas;
    if (!el) return;
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  });

  function fullscreen() {
    stage?.requestFullscreen().catch(() => {});
    canvas?.focus();
  }
</script>

<div class="remote">
  <div class="card head">
    <div class="row" style="gap:0.6rem; align-items:center;">
      <h2 style="margin:0;">Remote GPOS</h2>
      <span class="badge off">Eksperimental</span>
    </div>
    <p class="text-dim" style="margin:0.4rem 0 0.8rem; font-size:0.83rem;">
      Hanya jendela GPOS yang terlihat dan bisa dikendalikan — bukan seluruh layar
      komputer. Setiap sesi harus dinyalakan manual dan OTP-nya berganti tiap sesi.
    </p>
    <div class="roles">
      <button class:active={role === "terima"} onclick={() => setRole("terima")}>
        📥 Diremote (Terima)
      </button>
      <button class:active={role === "kontrol"} onclick={() => setRole("kontrol")}>
        🖱️ Remote PC Lain (Kontrol)
      </button>
    </div>
  </div>

  {#if role === "terima"}
    <div class="card" style="max-width:560px;">
      <label class="row switch-row">
        <input type="checkbox" checked={host?.enabled ?? false} disabled={busy || !host} onchange={toggleHost} />
        <span><b>Aktifkan Remote</b> — PC ini bisa diremote dengan ID + OTP di bawah</span>
      </label>

      {#if host?.enabled}
        <div class="creds">
          <div>
            <div class="text-dim">ID</div>
            <div class="big mono">{fmtId(host.remote_id)}</div>
          </div>
          <div>
            <div class="text-dim">OTP</div>
            <div class="big mono">
              {#if host.phase === "waiting"}
                {host.otp}
              {:else if host.phase === "connected"}
                <span class="text-dim" style="font-size:0.9rem;">(terpakai)</span>
              {:else}
                <span class="text-dim" style="font-size:0.9rem;">…</span>
              {/if}
            </div>
          </div>
        </div>

        <div class="status">
          {#if host.phase === "connecting"}
            ⏳ Menyambung ke relay…
          {:else if host.phase === "waiting"}
            🟢 Siap — berikan ID + OTP ke orang yang akan meremote.
          {:else if host.phase === "connected"}
            🔴 <b>Sedang diremote.</b>
          {:else if host.phase === "error"}
            ⚠️ {host.error ?? "Gagal menyambung"} — mencoba lagi…
          {/if}
        </div>
      {/if}

      <div style="margin-top:1rem;">
        <div style="font-weight:600; margin-bottom:0.4rem;">Akses untuk yang meremote</div>
        <label class="row opt">
          <input type="radio" name="remote-access" checked={host?.allow_control ?? true} onchange={() => setControl(true)} />
          <span><b>Kontrol penuh</b> — bisa klik & ketik di GPOS ini</span>
        </label>
        <label class="row opt">
          <input type="radio" name="remote-access" checked={!(host?.allow_control ?? true)} onchange={() => setControl(false)} />
          <span><b>Hanya lihat</b> — cuma bisa melihat layar GPOS</span>
        </label>
        <p class="text-dim" style="font-size:0.78rem; margin:0.4rem 0 0;">
          Bisa diganti kapan saja, juga di tengah sesi. Remote otomatis mati setiap
          aplikasi ditutup.
        </p>
      </div>
    </div>
  {:else}
    {#if !viewerLive}
      <div class="card" style="max-width:420px;">
        <div class="field">
          <label for="rid">ID PC tujuan</label>
          <input id="rid" class="mono" inputmode="numeric" placeholder="123 456 789" bind:value={inputId} />
        </div>
        <div class="field">
          <label for="rotp">OTP</label>
          <input
            id="rotp"
            class="mono"
            inputmode="numeric"
            maxlength="7"
            placeholder="6 angka"
            bind:value={inputOtp}
            onkeydown={(e) => e.key === "Enter" && connect()}
          />
        </div>
        <button class="btn-primary" onclick={connect}>Sambungkan</button>
        {#if viewer?.phase === "closed" && viewer.error}
          <p class="err">{viewer.error}</p>
        {/if}
      </div>
    {:else}
      <div class="toolbar">
        <span>
          {#if viewer?.phase === "connecting"}
            ⏳ Menyambung ke {fmtId(viewer.remote_id)}…
          {:else}
            🟢 Terhubung ke <b class="mono">{fmtId(viewer?.remote_id ?? "")}</b>
            {#if meta && !meta.control}<span class="badge off">Hanya lihat</span>{/if}
          {/if}
        </span>
        <span class="spacer"></span>
        <button class="btn-ghost" onclick={() => send({ type: "refresh" })}>🔄 Segarkan</button>
        <button class="btn-ghost" onclick={fullscreen}>⛶ Layar penuh</button>
        <button class="btn-danger" onclick={disconnect}>Putuskan</button>
      </div>
      <div class="stage" bind:this={stage}>
        {#if !hasFrame}
          <div class="placeholder text-dim">Menunggu layar dari PC tujuan…</div>
        {/if}
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <canvas
          bind:this={canvas}
          class:hidden={!hasFrame}
          class:view-only={!canControl}
          tabindex="0"
          onmousemove={onMove}
          onmousedown={onDown}
          onmouseup={onUp}
          oncontextmenu={(e) => e.preventDefault()}
        ></canvas>
      </div>
      <p class="text-dim hint">
        Klik layar di atas dulu supaya keyboard (termasuk F1–F12) diteruskan ke PC tujuan.
      </p>
    {/if}
  {/if}
</div>

<style>
  .remote {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
    height: 100%;
    min-height: 0;
  }
  .roles {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .roles button {
    padding: 0.5rem 0.9rem;
  }
  .roles button.active {
    background: var(--primary, #2563eb);
    color: #fff;
    border-color: transparent;
  }
  .switch-row {
    gap: 0.6rem;
    align-items: center;
  }
  .switch-row input,
  .opt input {
    width: auto;
  }
  .opt {
    gap: 0.5rem;
    align-items: center;
    margin: 0.25rem 0;
  }
  .creds {
    display: flex;
    gap: 2.5rem;
    margin: 1rem 0 0.6rem;
    padding: 0.9rem 1rem;
    border-radius: 8px;
    background: var(--baby-blue-bg, rgba(37, 99, 235, 0.08));
  }
  .big {
    font-size: 1.9rem;
    font-weight: 700;
    letter-spacing: 0.06em;
  }
  .status {
    font-size: 0.9rem;
  }
  .field {
    margin-bottom: 0.7rem;
  }
  .field label {
    display: block;
    font-size: 0.8rem;
    margin-bottom: 0.2rem;
  }
  .field input {
    font-size: 1.2rem;
    letter-spacing: 0.08em;
  }
  .err {
    color: var(--danger, #dc2626);
    font-size: 0.85rem;
    margin: 0.6rem 0 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .spacer {
    flex: 1;
  }
  .stage {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #111;
    border-radius: 6px;
    overflow: hidden;
    position: relative;
  }
  .stage:fullscreen {
    border-radius: 0;
  }
  canvas {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    outline: none;
    cursor: default;
  }
  canvas:focus {
    box-shadow: 0 0 0 2px #22c55e;
  }
  canvas.view-only {
    cursor: not-allowed;
  }
  canvas.hidden {
    display: none;
  }
  .placeholder {
    color: #aaa;
  }
  .hint {
    font-size: 0.78rem;
    margin: 0;
  }
</style>
