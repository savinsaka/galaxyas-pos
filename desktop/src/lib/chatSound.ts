// Bunyi alert chat: "ding-dong" dua nada, ±0,6 detik. Dibuat langsung dengan
// Web Audio (tanpa file audio), jadi tidak ada aset tambahan maupun urusan
// hak cipta. Hanya dipakai untuk pesan yang dikirim sebagai ALERT — pesan chat
// biasa tetap tanpa bunyi.

let ctx: AudioContext | null = null;
let lastPlayed = 0;

function audio(): AudioContext | null {
  try {
    ctx ??= new AudioContext();
    return ctx;
  } catch {
    return null;
  }
}

// WebView bisa menahan audio sampai ada interaksi pertama. Kasir pasti
// mengklik/mengetik sesuatu (login), jadi buka kuncinya di interaksi pertama.
if (typeof window !== "undefined") {
  const unlock = () => {
    audio()?.resume().catch(() => {});
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
  // Serangan cepat lalu meluruh — terdengar seperti bel, bukan dengung.
  gain.gain.setValueAtTime(0.0001, start);
  gain.gain.exponentialRampToValueAtTime(volume, start + 0.015);
  gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);
  osc.connect(gain).connect(ac.destination);
  osc.start(start);
  osc.stop(start + duration + 0.02);
}

/** Bunyikan alert. Beberapa alert beruntun dalam 1,5 detik hanya berbunyi sekali. */
export function playAlertSound() {
  const now = Date.now();
  if (now - lastPlayed < 1500) return;
  lastPlayed = now;
  const ac = audio();
  if (!ac) return;
  ac.resume().catch(() => {});
  const t = ac.currentTime + 0.02;
  tone(ac, 1318.5, t, 0.35, 0.25); // E6 — "ding"
  tone(ac, 1046.5, t + 0.2, 0.45, 0.22); // C6 — "dong"
}
