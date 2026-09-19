import type { Transaction } from "$lib/types";

/** Bagian transaksi yang dibutuhkan untuk memecah nilai per metode bayar. */
type PaidFields = Pick<Transaction, "payment_method" | "total" | "paid"> & {
  paid_cash?: number | null;
  paid_qris?: number | null;
};

/**
 * Pecah satu transaksi jadi porsi uang per metode pembayaran NYATA.
 *
 * "Kombinasi" hanya status pembayaran di struk / daftar kasir; di laporan
 * nilainya dibagi ke Tunai dan QRIS sesuai rincian bayar (mis. total 10.000
 * dengan 5.000 QRIS + 5.000 tunai → 5.000 masuk QRIS, 5.000 masuk Tunai).
 *
 * Kembalian dianggap selalu diberikan tunai dari laci, jadi porsi QRIS =
 * `paid_qris` (dibatasi maksimum `total`) dan sisanya porsi tunai — sejalan
 * dengan hitungan `shift_summary` di sisi Rust. Transaksi kombinasi lama yang
 * tidak punya rincian (kedua kolom kosong) dibiarkan di bucket "Kombinasi"
 * supaya nilainya tidak hilang dari laporan.
 */
export function paymentParts(t: PaidFields): [string, number][] {
  if (t.payment_method !== "Kombinasi") return [[t.payment_method, t.total]];

  const clamp = (n: number) => Math.min(Math.max(n, 0), t.total);
  let qris: number;
  if (t.paid_qris != null) {
    qris = clamp(t.paid_qris);
  } else if (t.paid_cash != null) {
    // Rincian lama tanpa paid_qris: sisa setelah tunai bersih (di luar kembalian).
    qris = clamp(t.total - clamp(t.paid_cash - Math.max(t.paid - t.total, 0)));
  } else {
    return [["Kombinasi", t.total]];
  }

  const cash = t.total - qris;
  const parts: [string, number][] = [];
  if (cash > 0) parts.push(["Tunai", cash]);
  if (qris > 0) parts.push(["QRIS", qris]);
  return parts;
}

/**
 * Total uang per metode pembayaran untuk sekumpulan transaksi, dengan
 * "Kombinasi" sudah dipecah ke Tunai/QRIS. `count` = jumlah transaksi yang
 * menyumbang ke metode itu (transaksi kombinasi terhitung di dua metode).
 */
export function totalsByMethod(txs: PaidFields[]): Map<string, { count: number; total: number }> {
  const map = new Map<string, { count: number; total: number }>();
  for (const t of txs) {
    for (const [method, amount] of paymentParts(t)) {
      const b = map.get(method) ?? { count: 0, total: 0 };
      b.count += 1;
      b.total += amount;
      map.set(method, b);
    }
  }
  return map;
}
