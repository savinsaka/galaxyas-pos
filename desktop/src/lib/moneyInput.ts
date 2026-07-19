import { tick } from "svelte";

/** Format angka jadi string id-ID untuk ditampilkan di dalam input teks
 * (mis. field Bayar/Harga) — "50000.43" -> "50.000,43". Kosong kalau 0/null,
 * supaya placeholder tetap kelihatan alih-alih "0,00". */
export function formatMoneyInput(n: number | null | undefined): string {
  if (!n) return "";
  return n.toLocaleString("id-ID", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

/** Kebalikan formatMoneyInput — baca string hasil ketikan user (titik ribuan,
 * koma desimal) jadi number. "50.000,43" -> 50000.43. */
export function parseMoneyInput(raw: string): number {
  const cleaned = raw.replace(/\./g, "").replace(",", ".").replace(/[^\d.]/g, "");
  const n = parseFloat(cleaned);
  return Number.isFinite(n) ? n : 0;
}

/** oninput handler generik untuk field uang berformat ribuan+desimal. Set
 * angka lewat `setValue`, lalu setelah value terformat ulang di-render,
 * taruh kursor tepat SEBELUM koma (bagian desimal) — supaya ketik lanjut
 * (mis. "1" jadi "1,00") membangun angka rupiahnya, bukan nyelip di
 * belakang ",00" (yang bikin cursor kelihatan "loncat ke belakang"). */
export function onMoneyInput(e: Event, setValue: (n: number) => void) {
  const input = e.currentTarget as HTMLInputElement;
  setValue(parseMoneyInput(input.value));
  tick().then(() => {
    const idx = input.value.indexOf(",");
    const pos = idx === -1 ? input.value.length : idx;
    input.setSelectionRange(pos, pos);
  });
}
