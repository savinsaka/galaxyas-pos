// Status koneksi ke Server Pusat untuk banner "tidak terhubung" di shell.
// HP kasir jauh lebih sering lepas wifi daripada PC — cek berkala + saat
// app kembali fokus, dan sediakan retry manual.

import { get, writable } from "svelte/store";
import { api } from "$lib/api";
import { currentServer } from "$lib/stores/activeServer";

export const serverReachable = writable(true);
export const checking = writable(false);

export async function checkConnection(): Promise<boolean> {
  const info = get(currentServer);
  if (!info?.host || !info?.port || !info?.token) return true; // belum pairing — bukan urusan banner
  checking.set(true);
  try {
    await api.pingServer(info.host, info.port, info.token);
    serverReachable.set(true);
    return true;
  } catch {
    serverReachable.set(false);
    return false;
  } finally {
    checking.set(false);
  }
}

/** Mulai pemantauan: interval 30 detik + saat window kembali visible. */
export function startConnectionWatch(): () => void {
  const onVisible = () => {
    if (document.visibilityState === "visible") checkConnection();
  };
  const timer = setInterval(checkConnection, 30_000);
  document.addEventListener("visibilitychange", onVisible);
  checkConnection();
  return () => {
    clearInterval(timer);
    document.removeEventListener("visibilitychange", onVisible);
  };
}
