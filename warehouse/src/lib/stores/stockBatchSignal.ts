import { writable } from "svelte/store";

/** Naik tiap kali batch Item Masuk/Keluar diedit dari tab "Edit Item Masuk/Keluar" —
 * dipakai Daftar Item Masuk/Keluar untuk tahu kapan perlu reload. */
export const stockBatchesDirty = writable(0);

export function markStockBatchesDirty() {
  stockBatchesDirty.update((n) => n + 1);
}
