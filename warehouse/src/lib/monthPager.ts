/** Helper murni untuk navigasi "month-pager": paging berbasis bulan kalender. */

export function isoDate(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

/** Batas awal/akhir bulan kalender (year, month0=0-11). */
export function monthBounds(year: number, month0: number): [string, string] {
  const first = new Date(year, month0, 1);
  const last = new Date(year, month0 + 1, 0);
  return [isoDate(first), isoDate(last)];
}

export function currentMonthBounds(): [string, string] {
  const d = new Date();
  return monthBounds(d.getFullYear(), d.getMonth());
}

/** Geser `delta` bulan dari bulan yang memuat `fromIso` (atau hari ini bila kosong/invalid). */
export function shiftMonthBounds(fromIso: string, delta: number): [string, string] {
  const anchor = fromIso ? new Date(fromIso) : new Date();
  if (isNaN(anchor.getTime())) return currentMonthBounds();
  return monthBounds(anchor.getFullYear(), anchor.getMonth() + delta);
}

/** Label "Juli 2026" dari sebuah tanggal ISO di dalam bulan tsb. */
export function monthLabel(fromIso: string): string {
  const d = fromIso ? new Date(fromIso) : new Date();
  if (isNaN(d.getTime())) return "";
  return d.toLocaleDateString("id-ID", { month: "long", year: "numeric" });
}
