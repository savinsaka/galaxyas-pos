import { writable } from "svelte/store";

/** Baris keranjang yang ditahan (struktural sama dengan CartLine di KasirPOS). */
export interface PendingCartLine {
  product_id: string;
  name: string;
  barcode: string | null;
  price: number;
  qty: number;
  discount: number;
  brand: string | null;
  default_discount: number;
  periodic: boolean;
  manualOverride: boolean;
  manualPercent: number | null;
  stock_qty: number;
}

export interface PendingSale {
  id: string;
  label: string;
  cart: PendingCartLine[];
  customerId: string | null;
  paymentMethod: string;
  paid: number;
  paidCash?: number;
  paidQris?: number;
  createdAt: string;
}

/** Sengaja hanya di memori (bukan tabel DB) — hilang saat aplikasi ditutup, sesuai keputusan produk. */
export const pendingSales = writable<PendingSale[]>([]);

export function addPending(sale: Omit<PendingSale, "id" | "createdAt">): string {
  const id = crypto.randomUUID();
  pendingSales.update((list) => [...list, { ...sale, id, createdAt: new Date().toISOString() }]);
  return id;
}

export function removePending(id: string) {
  pendingSales.update((list) => list.filter((s) => s.id !== id));
}
