import { create } from "zustand";
import type { SyncStatusInfo } from "@/types";
import { ipc } from "@/lib/ipc/commands";

interface SyncState {
  status: SyncStatusInfo | null;
  refreshing: boolean;
  refresh: () => Promise<void>;
  triggerSync: () => Promise<void>;
}

export const useSyncStore = create<SyncState>((set) => ({
  status: null,
  refreshing: false,

  refresh: async () => {
    try {
      const status = await ipc.syncStatus();
      set({ status });
    } catch {
      /* keep previous status on transient errors */
    }
  },

  triggerSync: async () => {
    set({ refreshing: true });
    try {
      const status = await ipc.triggerSync();
      set({ status });
    } finally {
      set({ refreshing: false });
    }
  },
}));
