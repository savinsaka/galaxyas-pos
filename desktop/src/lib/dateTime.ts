/** Tanggal hari ini dalam format YYYY-MM-DD (lokal). */
export const todayIso = () => new Date().toISOString().slice(0, 10);

/** Label periode untuk header laporan cetak, mis. "Periode: 01 Jul 2026 – 16 Jul 2026". */
export function formatPeriodLabel(from: string, to: string): string {
  if (!from && !to) return "Periode: Semua";
  const fmt = (s: string) =>
    new Date(s).toLocaleDateString("id-ID", { day: "2-digit", month: "short", year: "numeric" });
  if (from && to) return `Periode: ${fmt(from)} – ${fmt(to)}`;
  return `Periode: ${fmt(from || to)}`;
}

/** Jam sekarang dalam format HH:MM (untuk isian `input type="time"`). */
export const nowHHMM = () => {
  const d = new Date();
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
};

/**
 * Gabungkan tanggal (YYYY-MM-DD) + jam yang DIPILIH sendiri (HH:MM atau
 * HH:MM:SS) menjadi datetime ISO. Dipakai Time Opname, yang jamnya justru
 * bukan jam sekarang. Kembalikan "" kalau isiannya belum lengkap/valid.
 */
export function combineDateAndClock(dateStr: string, timeStr: string): string {
  const [y, m, d] = (dateStr || "").split("-").map(Number);
  const [hh, mm, ss] = (timeStr || "").split(":").map(Number);
  if (!y || !m || !d || Number.isNaN(hh) || Number.isNaN(mm)) return "";
  return new Date(y, m - 1, d, hh, mm, ss || 0, 0).toISOString();
}

/** Gabungkan tanggal terpilih (YYYY-MM-DD, bisa mundur) + jam saat ini menjadi datetime ISO. */
export function combineDateAndTime(dateStr: string, time: Date): string {
  const [y, m, d] = dateStr.split("-").map(Number);
  return new Date(
    y,
    (m || 1) - 1,
    d || 1,
    time.getHours(),
    time.getMinutes(),
    time.getSeconds(),
    time.getMilliseconds(),
  ).toISOString();
}
