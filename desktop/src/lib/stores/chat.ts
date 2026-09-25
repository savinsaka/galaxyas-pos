// Status & pesan Chat antar toko, dipakai bersama oleh view Chat dan popup
// alert global. Event dari Rust (chat://*) didengarkan SEKALI di sini
// (initChat dipanggil oleh ChatAlert yang terpasang di +page.svelte).
//
// Status dibaca (read_at) disimpan di server, bukan di PC ini, supaya angka
// "belum dibaca" sama di semua PC toko dan pengirim bisa melihat ✓✓ biru.
import { writable, get } from "svelte/store";
import { listen } from "@tauri-apps/api/event";
import { api } from "$lib/api";
import { showToast } from "$lib/toast";
import { playAlertSound } from "$lib/chatSound";
import type { ChatContact, ChatMessage, ChatOutgoing, ChatStatus } from "$lib/types";

export const chatStatus = writable<ChatStatus | null>(null);
export const chatMessages = writable<ChatMessage[]>([]);
export const chatContacts = writable<ChatContact[]>([]);
/** Alert (pesan push) yang belum ditutup. */
export const chatAlerts = writable<ChatMessage[]>([]);
/** Pesan keluar yang belum dikonfirmasi server (🕓) atau gagal (✕). Tidak ada
 *  kirim ulang otomatis — yang gagal hanya terkirim lewat tombol "Kirim ulang". */
export const chatOutbox = writable<ChatOutgoing[]>([]);
/** Nama terdaftar di web untuk kode toko yang belum disimpan sebagai kontak. */
export const chatDirectory = writable<Record<string, string>>({});

/** Kode toko lawan bicara untuk sebuah pesan. */
export function peerOf(m: ChatMessage, me: string): string {
  return m.from === me ? m.to : m.from;
}

export function isUnread(m: ChatMessage, me: string): boolean {
  return m.from !== me && !m.read_at;
}

/** Tandai semua pesan dari `peer` sudah dibaca (hanya dipanggil saat
 *  percakapan itu benar-benar terlihat di layar). */
export function markRead(peer: string) {
  const me = get(chatStatus)?.code ?? "";
  const unread = get(chatMessages).filter((m) => m.from === peer && isUnread(m, me));
  if (unread.length === 0) return;
  const upTo = Math.max(...unread.map((m) => m.id));
  const now = new Date().toISOString();
  // Langsung hilangkan angka belum dibaca; server mengonfirmasi lewat receipt.
  chatMessages.update((list) =>
    list.map((m) => (m.from === peer && m.to === me && m.id <= upTo && !m.read_at ? { ...m, read_at: now } : m)),
  );
  api.chatMarkRead(peer, upTo).catch(() => {});
}

/** Nama tampilan: nama kontak tersimpan → nama terdaftar di web → kode. */
export function contactName(code: string): string {
  return get(chatContacts).find((c) => c.code === code)?.name ?? get(chatDirectory)[code] ?? code;
}

const lookingUp = new Set<string>();

/** Ambil nama toko dari web untuk kode yang belum disimpan (sekali per kode). */
export function resolveName(code: string) {
  if (!code || lookingUp.has(code) || get(chatDirectory)[code]) return;
  if (get(chatContacts).some((c) => c.code === code)) return;
  lookingUp.add(code);
  api
    .chatLookup(code)
    .then((r) => chatDirectory.update((d) => ({ ...d, [code]: r.name })))
    .catch(() => {});
}

// ---------- Kirim (dengan tanda 🕓 / ✕) ----------

const SEND_TIMEOUT_MS = 15_000;
const timers = new Map<string, ReturnType<typeof setTimeout>>();

function newClientId(): string {
  return crypto.randomUUID().replace(/-/g, "");
}

function failOut(clientId: string, error: string) {
  const t = timers.get(clientId);
  if (t) clearTimeout(t);
  timers.delete(clientId);
  chatOutbox.update((o) =>
    o.map((x) => (x.client_id === clientId && x.status === "sending" ? { ...x, status: "failed", error } : x)),
  );
}

function confirmOut(clientId: string) {
  const t = timers.get(clientId);
  if (t) clearTimeout(t);
  timers.delete(clientId);
  chatOutbox.update((o) => o.filter((x) => x.client_id !== clientId));
}

async function dispatch(item: ChatOutgoing) {
  chatOutbox.update((o) => [...o.filter((x) => x.client_id !== item.client_id), item]);
  if (!get(chatStatus)?.connected) {
    failOut(item.client_id, "PC ini sedang offline.");
    return;
  }
  timers.set(
    item.client_id,
    setTimeout(() => failOut(item.client_id, "Server tidak menjawab."), SEND_TIMEOUT_MS),
  );
  try {
    if (item.voice) {
      await api.chatSendFile(item.to, "voice.webm", item.voice.bytes, item.push, item.client_id, item.voice.duration);
    } else if (item.file) {
      await api.chatSendFile(item.to, item.file.name, item.file.bytes, item.push, item.client_id);
    } else {
      await api.chatSend(item.to, item.body, item.push, item.client_id);
    }
  } catch (e) {
    failOut(item.client_id, String(e));
  }
}

export function sendText(to: string, body: string, push: boolean) {
  return dispatch({
    client_id: newClientId(),
    to,
    body,
    push,
    created_at: new Date().toISOString(),
    status: "sending",
  });
}

export function sendFile(to: string, name: string, bytes: Uint8Array, push: boolean) {
  return dispatch({
    client_id: newClientId(),
    to,
    body: "",
    push,
    created_at: new Date().toISOString(),
    status: "sending",
    file: { name, size: bytes.length, bytes },
  });
}

export function sendVoice(to: string, bytes: Uint8Array, duration: number, push: boolean) {
  return dispatch({
    client_id: newClientId(),
    to,
    body: "",
    push,
    created_at: new Date().toISOString(),
    status: "sending",
    voice: { bytes, duration },
  });
}

/** "0:12" dari detik. */
export function fmtDuration(sec: number): string {
  const s = Math.max(0, Math.round(sec));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/** Cuplikan satu baris untuk daftar chat & popup alert. */
export function messagePreview(m: { kind: string; body: string; file_name: string | null }): string {
  if (m.kind === "voice") return `🎤 Voice note (${fmtDuration(Number(m.body) || 0)})`;
  if (m.kind === "file") return "📎 " + (m.file_name ?? "file") + (m.body ? " — " + m.body : "");
  return m.body;
}

/** Kirim ulang manual — id baru supaya tidak tertukar dengan percobaan lama. */
export function retryOut(clientId: string) {
  const item = get(chatOutbox).find((x) => x.client_id === clientId);
  if (!item) return;
  chatOutbox.update((o) => o.filter((x) => x.client_id !== clientId));
  return dispatch({ ...item, client_id: newClientId(), status: "sending", error: undefined });
}

export function discardOut(clientId: string) {
  confirmOut(clientId);
}

function upsert(m: ChatMessage) {
  chatMessages.update((list) => {
    const i = list.findIndex((x) => x.id === m.id);
    if (i >= 0) {
      const copy = list.slice();
      copy[i] = { ...copy[i], ...m };
      return copy;
    }
    return [...list, m].sort((a, b) => a.id - b.id);
  });
}

export async function refreshHistory() {
  try {
    const rows = await api.chatHistory();
    chatMessages.set(rows.sort((a, b) => a.id - b.id));
  } catch {
    /* belum tersambung — coba lagi saat status connected */
  }
}

let started = false;

export async function initChat() {
  if (started) return;
  started = true;
  try {
    const { status } = await api.chatStatus();
    chatStatus.set(status);
    chatContacts.set(await api.chatContacts());
    if (status.configured) await refreshHistory();
  } catch {
    /* build lama tanpa command chat */
  }

  await listen<ChatStatus>("chat://status", (e) => {
    const was = get(chatStatus)?.connected;
    chatStatus.set(e.payload);
    // Baru tersambung (lagi): tarik pesan yang masuk selama terputus.
    if (e.payload.connected && !was) refreshHistory();
    // Putus: yang masih 🕓 dianggap gagal (✕) — tidak dikirim ulang otomatis.
    if (!e.payload.connected) {
      for (const x of get(chatOutbox)) if (x.status === "sending") failOut(x.client_id, "Koneksi terputus.");
    }
  });

  await listen<ChatMessage>("chat://message", (e) => {
    const m = e.payload;
    if (m.client_id) confirmOut(m.client_id);
    upsert(m);
    const me = get(chatStatus)?.code ?? "";
    if (m.from !== me) {
      resolveName(m.from);
      if (m.push) {
        chatAlerts.update((a) => [...a, m]);
        playAlertSound();
      }
    }
    if (m.save_error) showToast(m.save_error, "error", 5000);
  });

  await listen<{ ids: number[]; delivered_at: string | null; read_at: string | null }>("chat://receipt", (e) => {
    const { ids, delivered_at, read_at } = e.payload;
    const set = new Set(ids);
    chatMessages.update((list) =>
      list.map((m) =>
        set.has(m.id)
          ? { ...m, delivered_at: m.delivered_at ?? delivered_at, read_at: m.read_at ?? read_at }
          : m,
      ),
    );
  });

  await listen<{ message: string; client_id?: string | null }>("chat://error", (e) => {
    const id = e.payload.client_id;
    if (id && get(chatOutbox).some((x) => x.client_id === id)) failOut(id, e.payload.message);
    else showToast(e.payload.message, "error", 5000);
  });
}
