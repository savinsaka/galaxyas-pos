// Pengaturan audio Voice Call per PC: mikrofon (input), speaker (output), dan
// pemrosesan suara. Disimpan di setting lokal SQLite seperti preferensi kasir.

import { get, writable } from "svelte/store";
import { api } from "$lib/api";

export interface AudioPrefs {
  /** deviceId mikrofon; "" = default Windows. */
  micId: string;
  /** deviceId speaker; "" = default Windows. */
  speakerId: string;
  echoCancellation: boolean;
  noiseSuppression: boolean;
  autoGainControl: boolean;
}

const DEFAULTS: AudioPrefs = {
  micId: "",
  speakerId: "",
  echoCancellation: true,
  noiseSuppression: true,
  autoGainControl: true,
};

const KEYS: Record<keyof AudioPrefs, string> = {
  micId: "call_mic_device",
  speakerId: "call_speaker_device",
  echoCancellation: "call_echo_cancel",
  noiseSuppression: "call_noise_suppress",
  autoGainControl: "call_auto_gain",
};

export const audioPrefs = writable<AudioPrefs>(DEFAULTS);

let loaded = false;

export async function loadAudioPrefs(): Promise<AudioPrefs> {
  if (loaded) return get(audioPrefs);
  try {
    const s = await api.getSettings();
    const flag = (k: string, d: boolean) => (s[k] === undefined || s[k] === "" ? d : s[k] === "1");
    audioPrefs.set({
      micId: s[KEYS.micId] ?? "",
      speakerId: s[KEYS.speakerId] ?? "",
      echoCancellation: flag(KEYS.echoCancellation, true),
      noiseSuppression: flag(KEYS.noiseSuppression, true),
      autoGainControl: flag(KEYS.autoGainControl, true),
    });
    loaded = true;
  } catch {
    /* pakai default */
  }
  return get(audioPrefs);
}

export async function saveAudioPrefs(patch: Partial<AudioPrefs>) {
  const next = { ...get(audioPrefs), ...patch };
  audioPrefs.set(next);
  for (const [k, v] of Object.entries(patch) as [keyof AudioPrefs, string | boolean][]) {
    await api.updateSetting(KEYS[k], typeof v === "boolean" ? (v ? "1" : "0") : v);
  }
}

/** Constraint getUserMedia sesuai pengaturan. `exactDevice=false` = mic default. */
export function micConstraints(p: AudioPrefs, exactDevice = true): MediaStreamConstraints {
  return {
    audio: {
      ...(exactDevice && p.micId ? { deviceId: { exact: p.micId } } : {}),
      echoCancellation: p.echoCancellation,
      noiseSuppression: p.noiseSuppression,
      autoGainControl: p.autoGainControl,
    },
    video: false,
  };
}

/** Arahkan elemen audio / AudioContext ke speaker terpilih (WebView2 mendukung setSinkId). */
export async function applySink(target: { setSinkId?: (id: string) => Promise<void> } | null, speakerId: string) {
  if (!target?.setSinkId) return;
  try {
    await target.setSinkId(speakerId || "");
  } catch {
    /* speaker sudah dicabut: tetap di default */
  }
}
