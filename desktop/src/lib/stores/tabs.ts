import { writable, get } from "svelte/store";
import { confirmCloseTab, clearTabDirty } from "./tabGuard";

export interface TabDef {
  id: string;
  viewKey: string;
  title: string;
  icon?: string;
  props?: Record<string, unknown>;
  singleton?: boolean;
}

export const tabs = writable<TabDef[]>([]);
export const activeTabId = writable<string | null>(null);

let counter = 0;

export interface OpenTabOptions {
  viewKey: string;
  title: string;
  icon?: string;
  props?: Record<string, unknown>;
  /** Bila true, fokus ke tab yang sudah ada alih-alih membuka duplikat. */
  singleton?: boolean;
}

export function openTab(opts: OpenTabOptions): string {
  if (opts.singleton) {
    const existing = get(tabs).find((t) => t.viewKey === opts.viewKey);
    if (existing) {
      // Refresh judul & props (mis. transaksi yang diedit berbeda dari sebelumnya)
      // supaya tab singleton yang di-refokus tidak menampilkan data basi.
      tabs.update((list) =>
        list.map((t) => (t.id === existing.id ? { ...t, title: opts.title, props: opts.props } : t)),
      );
      activeTabId.set(existing.id);
      return existing.id;
    }
  }
  const id = `tab-${++counter}`;
  tabs.update((list) => [...list, { id, ...opts }]);
  activeTabId.set(id);
  return id;
}

export function closeTab(id: string) {
  if (!confirmCloseTab(id)) return;
  const list = get(tabs);
  const idx = list.findIndex((t) => t.id === id);
  const next = list.filter((t) => t.id !== id);
  tabs.set(next);
  clearTabDirty(id);
  if (get(activeTabId) === id) {
    const fallback = next[idx] ?? next[idx - 1] ?? next[next.length - 1] ?? null;
    activeTabId.set(fallback ? fallback.id : null);
  }
}

export function setActive(id: string) {
  activeTabId.set(id);
}

export function closeAllTabs() {
  tabs.set([]);
  activeTabId.set(null);
}

/** Ganti judul tab (mis. setelah simpan, untuk tab POS bernomor). */
export function renameTab(id: string, title: string) {
  tabs.update((list) => list.map((t) => (t.id === id ? { ...t, title } : t)));
}

/** Pindahkan tab `fromId` ke posisi tab `toId` (untuk seret-susun ala Chrome). */
export function moveTab(fromId: string, toId: string) {
  if (fromId === toId) return;
  tabs.update((list) => {
    const from = list.findIndex((t) => t.id === fromId);
    const to = list.findIndex((t) => t.id === toId);
    if (from < 0 || to < 0) return list;
    const next = [...list];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    return next;
  });
}
