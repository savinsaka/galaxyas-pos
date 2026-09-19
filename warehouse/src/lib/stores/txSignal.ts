import { writable } from "svelte/store";

/** Naik tiap kali sebuah transaksi diedit dari tab "Edit Kasir" — dipakai
 * Daftar Kasir untuk tahu kapan perlu reload tanpa coupling langsung antar view. */
export const transactionsDirty = writable(0);

export function markTransactionsDirty() {
  transactionsDirty.update((n) => n + 1);
}
