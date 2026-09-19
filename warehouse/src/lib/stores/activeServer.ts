import { derived, writable } from "svelte/store";
import type { ServerInfo, ServerPath } from "$lib/types";

export const currentServer = writable<ServerInfo | null>(null);
export const allServers = writable<ServerInfo[]>([]);
/** Jalur yang dipilih kasir untuk server aktif: wifi ("lan") atau internet. */
export const currentPath = writable<ServerPath>("lan");

/** true jika PC ini adalah client yang konek ke Server Pusat di PC lain. */
export const isRemoteClient = derived(currentServer, (s) => s?.kind === "remote");

/** true jika client tsb terhubung lewat internet (relay), bukan wifi toko. */
export const isOnlinePath = derived(
  [isRemoteClient, currentPath],
  ([remote, path]) => remote && path === "online",
);

/** Label jalur untuk ditampilkan ke kasir (Login, Pengaturan). */
export const pathLabel = (path: ServerPath) => (path === "online" ? "Internet" : "Wifi");
export const pathIcon = (path: ServerPath) => (path === "online" ? "🌐" : "🖧");
