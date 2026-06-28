import { writable, get } from "svelte/store";

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
  const list = get(tabs);
  const idx = list.findIndex((t) => t.id === id);
  const next = list.filter((t) => t.id !== id);
  tabs.set(next);
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
