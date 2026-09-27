import { writable } from "svelte/store";
import { api } from "$lib/api";

/** Closing SOP: daftar "things to do" yang wajib diceklis sebelum aplikasi
 * boleh ditutup. Default OFF. Disimpan di settings lokal PC ini:
 * `closing_sop_enabled` ("1"/"0") + `closing_sop_items` (JSON array string).
 *
 * Seperti activeShiftStore, store ini dibaca SINKRON oleh guard tutup
 * aplikasi (ShiftCloseGate.svelte) — handler `onCloseRequested` tidak boleh
 * menunggu `invoke()`. Makanya dimuat sekali saat boot dan di-update
 * langsung tiap kali Pengaturan → Closing SOP disimpan. */
export type ClosingSop = { enabled: boolean; items: string[] };

export const closingSopStore = writable<ClosingSop>({ enabled: false, items: [] });

export function parseClosingSop(s: Record<string, string>): ClosingSop {
  let items: string[] = [];
  try {
    const raw = JSON.parse(s.closing_sop_items ?? "[]");
    if (Array.isArray(raw)) items = raw.map((x) => String(x).trim()).filter(Boolean);
  } catch {
    items = [];
  }
  return { enabled: s.closing_sop_enabled === "1", items };
}

export async function loadClosingSop() {
  closingSopStore.set(parseClosingSop(await api.getSettings()));
}

export async function saveClosingSop(sop: ClosingSop) {
  const items = sop.items.map((x) => x.trim()).filter(Boolean);
  await api.updateSetting("closing_sop_items", JSON.stringify(items));
  await api.updateSetting("closing_sop_enabled", sop.enabled ? "1" : "0");
  closingSopStore.set({ enabled: sop.enabled, items });
}

/** SOP hanya menahan penutupan kalau aktif DAN ada isinya. */
export function sopActive(sop: ClosingSop) {
  return sop.enabled && sop.items.length > 0;
}
