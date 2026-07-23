import { writable } from "svelte/store";

export type ToastKind = "success" | "error" | "info";
export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

export const toasts = writable<Toast[]>([]);

let counter = 0;

export function showToast(message: string, kind: ToastKind = "info", timeout = 3000) {
  const id = ++counter;
  toasts.update((list) => [...list, { id, kind, message }]);
  setTimeout(() => {
    toasts.update((list) => list.filter((t) => t.id !== id));
  }, timeout);
}

export const toastError = (e: unknown) =>
  showToast(typeof e === "string" ? e : (e as Error)?.message ?? String(e), "error", 5000);
