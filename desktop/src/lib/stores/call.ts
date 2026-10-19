// Voice Call antar toko: WebRTC audio di webview, sinyal (dering, angkat,
// SDP/ICE) lewat socket chat (Rust chat.rs ↔ server chat.py). Server yang
// memutuskan siapa boleh bicara dengan siapa; di sini hanya satu panggilan
// sekaligus.

import { get, writable } from "svelte/store";
import { listen } from "@tauri-apps/api/event";
import { api } from "$lib/api";
import { showToast, toastError } from "$lib/toast";
import { playEndTone, startRingback, startRingtone, stopCallTones } from "$lib/callSound";
import { chatStatus, contactName } from "$lib/stores/chat";

export type CallPhase = "idle" | "outgoing" | "ringing" | "incoming" | "connecting" | "active" | "ended";

export interface CallView {
  phase: CallPhase;
  callId: string;
  peer: string;
  peerName: string;
  direction: "out" | "in";
  /** Waktu (ms) suara tersambung — dasar penghitung durasi. */
  startedAt: number | null;
  muted: boolean;
  /** Alasan selesai, sudah dalam kalimat untuk ditampilkan. */
  endText: string | null;
}

const IDLE: CallView = {
  phase: "idle", callId: "", peer: "", peerName: "", direction: "out",
  startedAt: null, muted: false, endText: null,
};

export const call = writable<CallView>(IDLE);

// Batas menunggu suara tersambung setelah diangkat (ICE lewat TURN biasanya < 3 dtk).
const CONNECT_TIMEOUT_MS = 25_000;

let pc: RTCPeerConnection | null = null;
let localStream: MediaStream | null = null;
let remoteAudio: HTMLAudioElement | null = null;
let iceServers: RTCIceServer[] = [];
let pendingCandidates: RTCIceCandidateInit[] = [];
let connectTimer: ReturnType<typeof setTimeout> | null = null;
let endTimer: ReturnType<typeof setTimeout> | null = null;

function endText(reason: string, direction: "out" | "in"): string {
  switch (reason) {
    case "offline": return "Toko sedang offline";
    case "busy": return "Sedang sibuk — ada panggilan lain";
    case "unknown": return "Kode toko tidak terdaftar untuk chat";
    case "rate_limited": return "Terlalu sering menelepon, tunggu sebentar";
    case "no_answer": return direction === "out" ? "Tidak dijawab" : "Panggilan tak terjawab";
    case "rejected": return direction === "out" ? "Panggilan ditolak" : "Panggilan ditolak";
    case "cancelled": return "Panggilan dibatalkan";
    case "hangup": return "Panggilan selesai";
    case "disconnected": return "Sambungan terputus";
    case "failed": return "Suara tidak tersambung (jaringan)";
    case "mic": return "Mikrofon tidak bisa dipakai";
    default: return "Panggilan selesai";
  }
}

function send(msg: Record<string, unknown>) {
  return api.chatCall(msg);
}

function cleanupMedia() {
  if (connectTimer) clearTimeout(connectTimer);
  connectTimer = null;
  stopCallTones();
  pc?.close();
  pc = null;
  localStream?.getTracks().forEach((t) => t.stop());
  localStream = null;
  if (remoteAudio) {
    remoteAudio.pause();
    remoteAudio.srcObject = null;
  }
  pendingCandidates = [];
}

/** Tutup panggilan di sisi ini dan tampilkan alasannya sebentar. */
function finish(reason: string, silent = false) {
  const cur = get(call);
  if (cur.phase === "idle") return;
  cleanupMedia();
  if (endTimer) clearTimeout(endTimer);
  if (silent) {
    call.set(IDLE);
    return;
  }
  playEndTone();
  call.set({ ...cur, phase: "ended", endText: endText(reason, cur.direction), muted: false });
  const id = cur.callId;
  endTimer = setTimeout(() => {
    if (get(call).callId === id && get(call).phase === "ended") call.set(IDLE);
  }, 3000);
}

async function getMic(): Promise<MediaStream> {
  return navigator.mediaDevices.getUserMedia({
    audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true },
    video: false,
  });
}

function createPeer(callId: string) {
  const peer = new RTCPeerConnection({ iceServers });
  pc = peer;
  for (const track of localStream?.getTracks() ?? []) peer.addTrack(track, localStream!);
  peer.onicecandidate = (e) => {
    if (e.candidate) send({ type: "call_signal", call_id: callId, data: { candidate: e.candidate.toJSON() } }).catch(() => {});
  };
  peer.ontrack = (e) => {
    remoteAudio ??= new Audio();
    remoteAudio.autoplay = true;
    remoteAudio.srcObject = e.streams[0] ?? new MediaStream([e.track]);
    remoteAudio.play().catch(() => {});
  };
  peer.onconnectionstatechange = () => {
    if (pc !== peer) return;
    if (peer.connectionState === "connected") {
      if (connectTimer) clearTimeout(connectTimer);
      connectTimer = null;
      call.update((c) => (c.callId === callId ? { ...c, phase: "active", startedAt: c.startedAt ?? Date.now() } : c));
    } else if (peer.connectionState === "failed") {
      send({ type: "call_hangup", call_id: callId }).catch(() => {});
      finish("failed");
    }
  };
  connectTimer = setTimeout(() => {
    if (pc === peer && peer.connectionState !== "connected") {
      send({ type: "call_hangup", call_id: callId }).catch(() => {});
      finish("failed");
    }
  }, CONNECT_TIMEOUT_MS);
  return peer;
}

async function flushCandidates() {
  if (!pc?.remoteDescription) return;
  for (const c of pendingCandidates.splice(0)) await pc.addIceCandidate(c).catch(() => {});
}

// ---------- Aksi dari UI ----------

/** Telepon toko `code`. */
export async function startCall(code: string) {
  code = code.trim();
  const cur = get(call);
  if (cur.phase !== "idle" && cur.phase !== "ended") {
    showToast("Masih ada panggilan yang berjalan.", "error");
    return;
  }
  const st = get(chatStatus);
  if (!st?.connected) {
    showToast("Chat sedang offline — tidak bisa menelepon.", "error");
    return;
  }
  if (!code || code === st.code) {
    showToast("Pilih toko lain untuk ditelepon.", "error");
    return;
  }
  try {
    localStream = await getMic();
  } catch {
    showToast("Mikrofon tidak bisa dipakai. Pastikan mic terpasang & diizinkan.", "error", 6000);
    return;
  }
  const callId = crypto.randomUUID();
  if (endTimer) clearTimeout(endTimer);
  call.set({ ...IDLE, phase: "outgoing", callId, peer: code, peerName: contactName(code), direction: "out" });
  try {
    await send({ type: "call_invite", call_id: callId, to: code });
  } catch (e) {
    cleanupMedia();
    call.set(IDLE);
    toastError(e);
  }
}

export async function acceptCall() {
  const cur = get(call);
  if (cur.phase !== "incoming") return;
  stopCallTones();
  try {
    localStream = await getMic();
  } catch {
    send({ type: "call_reject", call_id: cur.callId }).catch(() => {});
    finish("mic");
    return;
  }
  call.set({ ...cur, phase: "connecting" });
  createPeer(cur.callId);
  try {
    await send({ type: "call_accept", call_id: cur.callId });
  } catch (e) {
    toastError(e);
    finish("disconnected");
  }
}

export function rejectCall() {
  const cur = get(call);
  if (cur.phase !== "incoming") return;
  send({ type: "call_reject", call_id: cur.callId }).catch(() => {});
  finish("rejected", true);
}

export function hangupCall() {
  const cur = get(call);
  if (cur.phase === "idle" || cur.phase === "ended") return;
  if (cur.phase === "incoming") return rejectCall();
  send({ type: "call_hangup", call_id: cur.callId }).catch(() => {});
  finish(cur.phase === "active" ? "hangup" : "cancelled");
}

export function toggleMute() {
  const cur = get(call);
  const muted = !cur.muted;
  localStream?.getAudioTracks().forEach((t) => (t.enabled = !muted));
  call.set({ ...cur, muted });
}

// ---------- Sinyal dari server ----------

interface CallEvent {
  type: string;
  call_id: string;
  from?: string;
  from_name?: string;
  reason?: string;
  ice?: RTCIceServer[];
  data?: { sdp?: RTCSessionDescriptionInit; candidate?: RTCIceCandidateInit };
}

async function onEvent(ev: CallEvent) {
  const cur = get(call);
  const mine = ev.call_id === cur.callId;

  switch (ev.type) {
    case "call_ring": {
      if (cur.phase !== "idle" && cur.phase !== "ended") return; // server sudah cek sibuk
      iceServers = ev.ice ?? [];
      if (endTimer) clearTimeout(endTimer);
      const peer = ev.from ?? "";
      const name = contactName(peer);
      call.set({
        ...IDLE, phase: "incoming", callId: ev.call_id, peer,
        peerName: name !== peer ? name : ev.from_name || peer, direction: "in",
      });
      startRingtone();
      return;
    }
    case "call_ringing":
      if (!mine || cur.phase !== "outgoing") return;
      iceServers = ev.ice ?? [];
      call.set({ ...cur, phase: "ringing" });
      startRingback();
      return;
    case "call_accepted": {
      if (!mine) return;
      stopCallTones();
      call.set({ ...cur, phase: "connecting" });
      const peer = createPeer(cur.callId);
      const offer = await peer.createOffer();
      await peer.setLocalDescription(offer);
      await send({ type: "call_signal", call_id: cur.callId, data: { sdp: offer } }).catch(() => {});
      return;
    }
    case "call_signal": {
      if (!mine || !pc) return;
      const { sdp, candidate } = ev.data ?? {};
      if (sdp?.type === "offer") {
        await pc.setRemoteDescription(sdp);
        const answer = await pc.createAnswer();
        await pc.setLocalDescription(answer);
        await send({ type: "call_signal", call_id: cur.callId, data: { sdp: answer } }).catch(() => {});
        await flushCandidates();
      } else if (sdp?.type === "answer") {
        await pc.setRemoteDescription(sdp);
        await flushCandidates();
      } else if (candidate) {
        if (pc.remoteDescription) await pc.addIceCandidate(candidate).catch(() => {});
        else pendingCandidates.push(candidate);
      }
      return;
    }
    case "call_end":
      if (!mine) return;
      // Diangkat di PC lain toko ini: cukup berhenti berdering tanpa pesan.
      finish(ev.reason ?? "hangup", ev.reason === "answered_elsewhere");
      return;
  }
}

let started = false;

/** Dipasang sekali dari initChat(). */
export async function initCall() {
  if (started) return;
  started = true;
  await listen<CallEvent>("chat://call", (e) => {
    onEvent(e.payload).catch((err) => console.warn("Sinyal panggilan gagal:", err));
  });
  // Socket chat putus = sinyal panggilan ikut hilang; server juga mengakhiri.
  chatStatus.subscribe((st) => {
    if (st && !st.connected && get(call).phase !== "idle" && get(call).phase !== "ended") finish("disconnected");
  });
}
