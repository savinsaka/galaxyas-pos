import { writable } from "svelte/store";
import type { StoreInfo } from "$lib/types";

export const currentStore = writable<StoreInfo | null>(null);
export const allStores = writable<StoreInfo[]>([]);
