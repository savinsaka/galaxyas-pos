import { writable } from "svelte/store";
import type { Shift } from "$lib/types";

/** Snapshot shift aktif, dipakai SINKRON oleh guard tutup aplikasi
 * (ShiftCloseGate.svelte) — handler `onCloseRequested` TIDAK BOLEH menunggu
 * `invoke()` di dalamnya (berisiko freeze event loop webview2, mirip kasus
 * `open_print_window` yang WAJIB async di commands.rs). Di-update tiap kali
 * KasirPOS/ShiftKasir memuat atau mengubah status shift. */
export const activeShiftStore = writable<Shift | null>(null);
