import { writable } from "svelte/store";
import { api } from "$lib/api";
import type { SyncAccount } from "$lib/types";
import { openTab } from "$lib/stores/tabs";

/** Akun server (login anggota) PC ini — dibagi layar Sync, SSoT, Akun Server. */
export const syncAccount = writable<SyncAccount | null>(null);

export async function refreshSyncAccount(): Promise<SyncAccount | null> {
  try {
    const acc = await api.syncAccount();
    syncAccount.set(acc);
    return acc;
  } catch {
    syncAccount.set(null);
    return null;
  }
}

/** Buka (atau fokus) tab Sync → Akun Server. */
export function openSyncAkun() {
  openTab({ viewKey: "sync-akun", title: "Akun Server", icon: "🔑", singleton: true });
}
