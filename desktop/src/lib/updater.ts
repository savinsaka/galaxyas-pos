import { writable } from "svelte/store";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateState =
  | { status: "idle" }
  | { status: "available"; version: string }
  | { status: "downloading"; version: string; progress: number }
  | { status: "ready" }
  | { status: "error"; message: string };

export const updateState = writable<UpdateState>({ status: "idle" });

let pendingUpdate: Update | null = null;

/** Cek update sekali saat app dibuka. Kegagalan (offline, endpoint belum siap,
 * dsb.) tidak boleh mengganggu penggunaan aplikasi — cukup dicatat ke console. */
export async function checkForUpdate() {
  try {
    const update = await check();
    if (!update) return;
    pendingUpdate = update;
    updateState.set({ status: "available", version: update.version });
  } catch (e) {
    console.warn("Cek update gagal:", e);
  }
}

/** Dipanggil dari klik user pada banner notifikasi. Download lalu install
 * (mengganti file aplikasi lama), lalu restart otomatis ke versi baru. */
export async function installUpdate() {
  if (!pendingUpdate) return;
  let downloaded = 0;
  let total = 0;
  try {
    updateState.set({ status: "downloading", version: pendingUpdate.version, progress: 0 });
    await pendingUpdate.downloadAndInstall((event) => {
      if (event.event === "Started") {
        total = event.data.contentLength ?? 0;
      } else if (event.event === "Progress") {
        downloaded += event.data.chunkLength;
        const progress = total ? Math.min(100, Math.round((downloaded / total) * 100)) : 0;
        updateState.update((s) => (s.status === "downloading" ? { ...s, progress } : s));
      } else if (event.event === "Finished") {
        updateState.set({ status: "ready" });
      }
    });
    await relaunch();
  } catch (e) {
    updateState.set({ status: "error", message: e instanceof Error ? e.message : String(e) });
  }
}

export function dismissUpdate() {
  updateState.set({ status: "idle" });
}
