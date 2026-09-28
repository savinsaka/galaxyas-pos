import { writable } from "svelte/store";

/**
 * Popup konfirmasi/pemberitahuan milik app sendiri — pengganti `confirm()` /
 * `alert()` bawaan webview yang tampil sebagai kotak "tauri.localhost says"
 * (tidak ikut tema, judulnya aneh buat kasir). Dirender oleh
 * `DialogHost.svelte` di +layout; kalau ada beberapa permintaan sekaligus,
 * ditampilkan bergiliran.
 */
export interface DialogOptions {
  title?: string;
  okText?: string;
  cancelText?: string;
  /** Tombol OK merah — untuk aksi hapus/permanen. */
  danger?: boolean;
  icon?: string;
}

export interface DialogRequest extends DialogOptions {
  id: number;
  kind: "confirm" | "alert";
  message: string;
  resolve: (ok: boolean) => void;
}

export const dialogQueue = writable<DialogRequest[]>([]);

let counter = 0;

function open(kind: DialogRequest["kind"], message: string, opts: DialogOptions): Promise<boolean> {
  return new Promise((resolve) => {
    const id = ++counter;
    dialogQueue.update((q) => [
      ...q,
      {
        ...opts,
        id,
        kind,
        message,
        resolve: (ok) => {
          dialogQueue.update((list) => list.filter((d) => d.id !== id));
          resolve(ok);
        },
      },
    ]);
  });
}

/** Tanya ya/tidak. Resolve `true` kalau OK, `false` kalau Batal/Esc/klik luar. */
export function confirmDialog(message: string, opts: DialogOptions = {}): Promise<boolean> {
  return open("confirm", message, opts);
}

/** Pemberitahuan dengan satu tombol. */
export async function alertDialog(message: string, opts: DialogOptions = {}): Promise<void> {
  await open("alert", message, opts);
}
