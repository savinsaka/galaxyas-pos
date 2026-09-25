<script lang="ts">
  // Pemutar voice note ala WA: ▶/⏸, bar progres, durasi. Isi audio baru
  // dimuat saat pertama kali diputar (dari file di folder GPOS Chat, atau dari
  // memori untuk voice note yang belum terkirim).
  import { onDestroy } from "svelte";
  import { api } from "$lib/api";
  import { toastError } from "$lib/toast";

  let {
    path = "",
    bytes = null,
    duration = 0,
    mine = false,
  }: { path?: string; bytes?: Uint8Array | null; duration?: number; mine?: boolean } = $props();

  let audio: HTMLAudioElement | null = null;
  let url = "";
  let playing = $state(false);
  let current = $state(0);
  let loading = $state(false);

  const total = $derived(duration > 0 ? duration : 0);
  const progress = $derived(total > 0 ? Math.min(1, current / total) : 0);

  function fmt(sec: number): string {
    const s = Math.max(0, Math.round(sec));
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  }

  async function load(): Promise<HTMLAudioElement | null> {
    if (audio) return audio;
    loading = true;
    try {
      const data = bytes ?? new Uint8Array(await api.chatReadFile(path));
      url = URL.createObjectURL(new Blob([data], { type: "audio/webm" }));
      const a = new Audio(url);
      a.addEventListener("timeupdate", () => (current = a.currentTime));
      a.addEventListener("ended", () => {
        playing = false;
        current = 0;
      });
      a.addEventListener("pause", () => (playing = false));
      a.addEventListener("play", () => (playing = true));
      audio = a;
      return a;
    } catch (e) {
      toastError(e);
      return null;
    } finally {
      loading = false;
    }
  }

  async function toggle() {
    const a = await load();
    if (!a) return;
    if (a.paused) a.play().catch(toastError);
    else a.pause();
  }

  // Klik di bar → lompat ke posisi itu.
  async function seek(e: MouseEvent) {
    const a = await load();
    if (!a || total <= 0) return;
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    a.currentTime = Math.max(0, Math.min(total, ((e.clientX - r.left) / r.width) * total));
    current = a.currentTime;
  }

  onDestroy(() => {
    audio?.pause();
    if (url) URL.revokeObjectURL(url);
  });
</script>

<div class="vp" class:mine>
  <button class="play" onclick={toggle} disabled={loading} title={playing ? "Jeda" : "Putar"}>
    {#if loading}…{:else if playing}⏸{:else}▶{/if}
  </button>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="track" onclick={seek}>
    <div class="fill" style="width:{progress * 100}%"></div>
    <div class="knob" style="left:{progress * 100}%"></div>
  </div>
  <span class="dur">{fmt(playing || current > 0 ? current : total)}</span>
  <span class="mic">🎤</span>
</div>

<style>
  .vp {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    min-width: 230px;
    padding: 0.15rem 0;
  }
  .play {
    width: 34px;
    height: 34px;
    flex: 0 0 34px;
    border-radius: 50%;
    border: none;
    /* Ikut token aksen layar Chat bila ada (kontras terjaga di semua tema). */
    background: var(--c-accent, var(--primary));
    color: var(--c-on-accent, var(--white));
    font-size: 0.85rem;
    cursor: pointer;
    padding: 0;
  }
  .track {
    position: relative;
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: color-mix(in srgb, var(--text) 22%, transparent);
    cursor: pointer;
  }
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 2px;
    background: var(--c-accent, var(--primary));
  }
  .knob {
    position: absolute;
    top: 50%;
    width: 11px;
    height: 11px;
    margin-left: -5.5px;
    border-radius: 50%;
    background: var(--c-accent, var(--primary));
    transform: translateY(-50%);
  }
  .dur {
    font-size: 0.72rem;
    color: var(--muted, var(--text-dim));
    min-width: 2.2rem;
    text-align: right;
  }
  .mic {
    font-size: 0.85rem;
    opacity: 0.6;
  }
</style>
