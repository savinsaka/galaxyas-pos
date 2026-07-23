// Navigasi shell mobile: 4 tab bottom-nav, masing-masing punya stack layar
// sendiri (pengganti tab-MDI desktop). Sub-layar di-push ke stack tab aktif;
// tombol back hardware Android memicu `popstate` yang kita mirror ke pop()
// — setiap push() menambah satu entry history browser, jadi back selalu
// menutup layar teratas dulu sebelum keluar aplikasi.

import { get, writable } from "svelte/store";

export type TabKey = "kasir" | "produk" | "laporan" | "menu";

export interface StackEntry {
  /** Key komponen layar di screens/registry.ts */
  key: string;
  title: string;
  props?: Record<string, unknown>;
}

export const activeTab = writable<TabKey>("kasir");
export const stacks = writable<Record<TabKey, StackEntry[]>>({
  kasir: [],
  produk: [],
  laporan: [],
  menu: [],
});

/** true saat pop() dipicu dari history.back() internal (hindari double-pop). */
let poppingProgrammatically = false;

export function push(entry: StackEntry) {
  const tab = get(activeTab);
  stacks.update((s) => ({ ...s, [tab]: [...s[tab], entry] }));
  history.pushState({ galaxyasNav: true }, "");
}

/** Tutup layar teratas tab aktif. Dipanggil tombol back UI — lewat history
 *  supaya state browser & stack tetap sinkron dengan back hardware. */
export function pop() {
  const tab = get(activeTab);
  if (get(stacks)[tab].length === 0) return;
  poppingProgrammatically = true;
  history.back();
}

export function switchTab(tab: TabKey) {
  activeTab.set(tab);
}

/** Kosongkan semua stack (dipakai saat logout / ganti server). */
export function resetNav() {
  stacks.set({ kasir: [], produk: [], laporan: [], menu: [] });
  activeTab.set("kasir");
}

function handlePopstate() {
  poppingProgrammatically = false;
  const tab = get(activeTab);
  stacks.update((s) => {
    if (s[tab].length === 0) return s; // back di layar dasar: biarkan Android minimize
    return { ...s, [tab]: s[tab].slice(0, -1) };
  });
}

/** Pasang listener back hardware — panggil sekali dari shell saat mount. */
export function initNavHistory(): () => void {
  window.addEventListener("popstate", handlePopstate);
  return () => window.removeEventListener("popstate", handlePopstate);
}
