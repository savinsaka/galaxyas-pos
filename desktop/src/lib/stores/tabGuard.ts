import { writable, get } from "svelte/store";
import { confirmDialog } from "$lib/dialog";

/** Set tab id yang sedang punya data belum disimpan (kasir, tambah item
 * masuk/keluar, tambah barang, opname) — dipakai closeTab() untuk minta
 * konfirmasi sebelum benar-benar menutup, mencegah tab ke-close tidak sengaja. */
export const dirtyTabs = writable<Set<string>>(new Set());

export function setTabDirty(id: string, dirty: boolean) {
  dirtyTabs.update((set) => {
    const next = new Set(set);
    if (dirty) next.add(id);
    else next.delete(id);
    return next;
  });
}

export function clearTabDirty(id: string) {
  setTabDirty(id, false);
}

/** Resolve true kalau boleh lanjut menutup (tidak dirty, atau user konfirmasi). */
export async function confirmCloseTab(id: string): Promise<boolean> {
  if (!get(dirtyTabs).has(id)) return true;
  return confirmDialog("Ada transaksi yang belum disimpan di tab ini. Yakin mau menutup?", {
    title: "Tutup Tab?",
    okText: "Tutup Tab",
    danger: true,
  });
}
