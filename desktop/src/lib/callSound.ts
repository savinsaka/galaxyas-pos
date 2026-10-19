// Nada Voice Call, dibuat langsung dengan Web Audio (tanpa file audio) seperti
// chatSound.ts: dering masuk (berulang sampai diangkat/ditolak), nada tunggu
// keluar ("tuut… tuut…" ala telepon Indonesia, 425 Hz), dan bunyi pendek saat
// panggilan selesai.

import { get } from "svelte/store";
import { applySink, audioPrefs } from "$lib/stores/audioPrefs";

let ctx: AudioContext | null = null;
let loop: ReturnType<typeof setInterval> | null = null;

function audio(): AudioContext | null {
  try {
    ctx ??= new AudioContext();
    ctx.resume().catch(() => {});
    // Dering & nada lewat speaker yang dipilih di Pengaturan Audio.
    applySink(ctx as unknown as { setSinkId?: (id: string) => Promise<void> }, get(audioPrefs).speakerId);
    return ctx;
  } catch {
    return null;
  }
}

// Dering bisa datang saat kasir tidak sedang mengklik apa-apa; WebView menahan
// AudioContext yang dibuat tanpa interaksi. Buat & buka kuncinya di interaksi
// pertama (login pasti melibatkan klik/ketik), sama seperti chatSound.ts.
if (typeof window !== "undefined") {
  const unlock = () => {
    audio();
    window.removeEventListener("pointerdown", unlock, true);
    window.removeEventListener("keydown", unlock, true);
  };
  window.addEventListener("pointerdown", unlock, true);
  window.addEventListener("keydown", unlock, true);
}

function tone(ac: AudioContext, freq: number, start: number, duration: number, volume: number) {
  const osc = ac.createOscillator();
  const gain = ac.createGain();
  osc.type = "sine";
  osc.frequency.setValueAtTime(freq, start);
  gain.gain.setValueAtTime(0.0001, start);
  gain.gain.exponentialRampToValueAtTime(volume, start + 0.02);
  gain.gain.setValueAtTime(volume, start + duration - 0.05);
  gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);
  osc.connect(gain).connect(ac.destination);
  osc.start(start);
  osc.stop(start + duration + 0.02);
}

function repeat(play: (ac: AudioContext, t: number) => void, everyMs: number) {
  stopCallTones();
  const ac = audio();
  if (!ac) return;
  play(ac, ac.currentTime + 0.02);
  loop = setInterval(() => play(ac, ac.currentTime + 0.02), everyMs);
}

/** Dering panggilan masuk: dua nada naik-turun dua kali, diulang tiap 3 detik. */
export function startRingtone() {
  repeat((ac, t) => {
    for (const off of [0, 0.45]) {
      tone(ac, 988, t + off, 0.18, 0.22);
      tone(ac, 1319, t + off + 0.2, 0.18, 0.2);
    }
  }, 3000);
}

/** Nada tunggu di sisi penelepon: 425 Hz 1 detik, jeda 3 detik. */
export function startRingback() {
  repeat((ac, t) => tone(ac, 425, t, 1.0, 0.12), 4000);
}

export function stopCallTones() {
  if (loop) clearInterval(loop);
  loop = null;
}

/** Bunyi pendek "tut-tut" saat panggilan berakhir. */
export function playEndTone() {
  stopCallTones();
  const ac = audio();
  if (!ac) return;
  const t = ac.currentTime + 0.02;
  tone(ac, 480, t, 0.18, 0.15);
  tone(ac, 480, t + 0.3, 0.18, 0.15);
}
