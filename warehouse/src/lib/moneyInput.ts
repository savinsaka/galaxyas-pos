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
 * kembalikan kursor ke posisi digit yang SAMA relatif terhadap sebelum edit
 * (dihitung dari jumlah digit di kiri kursor), bukan selalu lempar ke
 * sebelum koma. Ini penting supaya edit satu digit di tengah angka (mis.
 * "40.000,00" -> "41.000,00") tidak bikin kursor loncat ke belakang setelah
 * setiap keystroke — cukup ganti satu digit, lanjut ketik di posisi yang sama. */
export function onMoneyInput(e: Event, setValue: (n: number) => void) {
  const input = e.currentTarget as HTMLInputElement;
  const caret = input.selectionStart ?? input.value.length;
  const digitsBeforeCaret = (input.value.slice(0, caret).match(/\d/g) ?? []).length;
  setValue(parseMoneyInput(input.value));
  tick().then(() => {
    const val = input.value;
    if (digitsBeforeCaret === 0) {
      input.setSelectionRange(0, 0);
      return;
    }
    let seen = 0;
    let pos = val.length;
    for (let i = 0; i < val.length; i++) {
      if (/\d/.test(val[i])) {
        seen++;
        if (seen === digitsBeforeCaret) {
          pos = i + 1;
          break;
        }
      }
    }
    input.setSelectionRange(pos, pos);
  });
}
