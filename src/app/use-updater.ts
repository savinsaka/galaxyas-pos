import { useEffect } from "react";
import { toast } from "sonner";
import { isTauri } from "@/lib/ipc/client";

/**
 * Check for application updates on startup using the Tauri updater plugin.
 * If a new version is available it is downloaded and installed, then the app
 * relaunches. No-op in the browser dev fallback.
 */
export function useUpdater() {
  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;

    (async () => {
      try {
        const { check } = await import("@tauri-apps/plugin-updater");
        const update = await check();
        if (cancelled || !update) return;

        toast.info(`Pembaruan ${update.version} tersedia`, {
          description: "Mengunduh di latar belakang…",
        });
        await update.downloadAndInstall();

        const { relaunch } = await import("@tauri-apps/plugin-process");
        toast.success("Pembaruan selesai. Memuat ulang…");
        await relaunch();
      } catch (e) {
        // Updater failures must never block normal (offline) usage.
        console.warn("update check failed", e);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);
}
