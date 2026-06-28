import { create } from "zustand";

export type TabKind =
  | "pos"
  | "items"
  | "inventory"
  | "reports"
  | "sync"
  | "shifts"
  | "audit"
  | "settings";

export interface AppTab {
  id: string;
  kind: TabKind;
  title: string;
  /** Singleton tabs (e.g. settings) cannot be opened more than once. */
  singleton: boolean;
  closable: boolean;
}

interface TabState {
  tabs: AppTab[];
  activeId: string | null;
  posCounter: number;
  openTab: (kind: TabKind, opts?: { title?: string }) => string;
  openPosTab: () => string;
  closeTab: (id: string) => void;
  setActive: (id: string) => void;
}

const SINGLETON_TITLES: Record<Exclude<TabKind, "pos">, string> = {
  items: "Master Barang",
  inventory: "Inventory",
  reports: "Laporan",
  sync: "Sinkronisasi",
  shifts: "Shift Kas",
  audit: "Audit Log",
  settings: "Pengaturan",
};

let seq = 0;
const newId = () => `tab-${Date.now()}-${seq++}`;

export const useTabStore = create<TabState>((set, get) => ({
  tabs: [],
  activeId: null,
  posCounter: 0,

  openTab: (kind, opts) => {
    if (kind === "pos") return get().openPosTab();
    const existing = get().tabs.find((t) => t.kind === kind);
    if (existing) {
      set({ activeId: existing.id });
      return existing.id;
    }
    const id = newId();
    const tab: AppTab = {
      id,
      kind,
      title: opts?.title ?? SINGLETON_TITLES[kind],
      singleton: true,
      closable: true,
    };
    set((s) => ({ tabs: [...s.tabs, tab], activeId: id }));
    return id;
  },

  openPosTab: () => {
    const id = newId();
    const n = get().posCounter + 1;
    const tab: AppTab = {
      id,
      kind: "pos",
      title: `Kasir #${n}`,
      singleton: false,
      closable: true,
    };
    set((s) => ({ tabs: [...s.tabs, tab], activeId: id, posCounter: n }));
    return id;
  },

  closeTab: (id) => {
    set((s) => {
      const idx = s.tabs.findIndex((t) => t.id === id);
      if (idx === -1) return s;
      const tabs = s.tabs.filter((t) => t.id !== id);
      let activeId = s.activeId;
      if (s.activeId === id) {
        const next = tabs[idx] ?? tabs[idx - 1] ?? null;
        activeId = next?.id ?? null;
      }
      return { tabs, activeId };
    });
  },

  setActive: (id) => set({ activeId: id }),
}));
