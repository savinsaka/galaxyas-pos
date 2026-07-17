import { derived, writable } from "svelte/store";
import type { ServerInfo } from "$lib/types";

export const currentServer = writable<ServerInfo | null>(null);
export const allServers = writable<ServerInfo[]>([]);

/** true jika PC ini adalah client yang konek ke Server Pusat di PC lain. */
export const isRemoteClient = derived(currentServer, (s) => s?.kind === "remote");
