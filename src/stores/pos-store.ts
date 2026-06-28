import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { Item } from "@/types";

export interface CartLine {
  item_id: string;
  nama_item: string;
  harga: number;
  qty: number;
  diskon: number;
}

export interface Cart {
  lines: CartLine[];
  /** Transaction-level manual discount (Rp). */
  diskon: number;
  pajak_persen: number;
  bayar: number;
  /** Index of the currently highlighted line for keyboard nav. */
  activeLine: number;
}

interface PosState {
  carts: Record<string, Cart>;
  ensureCart: (tabId: string, pajakPersen: number) => void;
  addItem: (tabId: string, item: Item, qty?: number) => void;
  setQty: (tabId: string, index: number, qty: number) => void;
  setLineDiscount: (tabId: string, index: number, diskon: number) => void;
  removeLine: (tabId: string, index: number) => void;
  setActiveLine: (tabId: string, index: number) => void;
  setDiskon: (tabId: string, diskon: number) => void;
  setBayar: (tabId: string, bayar: number) => void;
  clearCart: (tabId: string) => void;
  removeCart: (tabId: string) => void;
}

const emptyCart = (pajakPersen: number): Cart => ({
  lines: [],
  diskon: 0,
  pajak_persen: pajakPersen,
  bayar: 0,
  activeLine: -1,
});

export const usePosStore = create<PosState>()(
  immer((set) => ({
    carts: {},

    ensureCart: (tabId, pajakPersen) =>
      set((s) => {
        if (!s.carts[tabId]) s.carts[tabId] = emptyCart(pajakPersen);
      }),

    addItem: (tabId, item, qty = 1) =>
      set((s) => {
        const cart = s.carts[tabId];
        if (!cart) return;
        const existing = cart.lines.find((l) => l.item_id === item.id);
        const harga = Math.round(
          item.harga_jual * (1 - (item.diskon_persen || 0) / 100),
        );
        if (existing) {
          existing.qty += qty;
          cart.activeLine = cart.lines.indexOf(existing);
        } else {
          cart.lines.push({
            item_id: item.id,
            nama_item: item.nama_item,
            harga,
            qty,
            diskon: 0,
          });
          cart.activeLine = cart.lines.length - 1;
        }
      }),

    setQty: (tabId, index, qty) =>
      set((s) => {
        const line = s.carts[tabId]?.lines[index];
        if (line) line.qty = Math.max(0, qty);
      }),

    setLineDiscount: (tabId, index, diskon) =>
      set((s) => {
        const line = s.carts[tabId]?.lines[index];
        if (line) line.diskon = Math.max(0, diskon);
      }),

    removeLine: (tabId, index) =>
      set((s) => {
        const cart = s.carts[tabId];
        if (!cart) return;
        cart.lines.splice(index, 1);
        cart.activeLine = Math.min(cart.activeLine, cart.lines.length - 1);
      }),

    setActiveLine: (tabId, index) =>
      set((s) => {
        const cart = s.carts[tabId];
        if (cart) cart.activeLine = index;
      }),

    setDiskon: (tabId, diskon) =>
      set((s) => {
        const cart = s.carts[tabId];
        if (cart) cart.diskon = Math.max(0, diskon);
      }),

    setBayar: (tabId, bayar) =>
      set((s) => {
        const cart = s.carts[tabId];
        if (cart) cart.bayar = Math.max(0, bayar);
      }),

    clearCart: (tabId) =>
      set((s) => {
        const cart = s.carts[tabId];
        if (cart) {
          cart.lines = [];
          cart.diskon = 0;
          cart.bayar = 0;
          cart.activeLine = -1;
        }
      }),

    removeCart: (tabId) =>
      set((s) => {
        delete s.carts[tabId];
      }),
  })),
);

/** Pure helper to compute totals for a cart. */
export function computeTotals(cart: Cart) {
  const subtotal = cart.lines.reduce(
    (sum, l) => sum + (l.harga * l.qty - l.diskon),
    0,
  );
  const afterDiskon = Math.max(0, subtotal - cart.diskon);
  const pajak = Math.round((afterDiskon * cart.pajak_persen) / 100);
  const total = afterDiskon + pajak;
  const kembali = Math.max(0, cart.bayar - total);
  return { subtotal, pajak, total, kembali };
}
