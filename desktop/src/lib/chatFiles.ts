// Jenis file lampiran chat: menentukan ikon kartu file dan cara pratinjau.
// Pratinjau hanya untuk foto, PDF, Word (docx), dan Excel (xlsx/xls/csv);
// sisanya (doc lama, heic, tif) cukup tombol Download.

export const CHAT_ACCEPT = ".jpg,.jpeg,.png,.gif,.webp,.bmp,.heic,.heif,.tif,.tiff,.doc,.docx,.xls,.xlsx,.csv,.pdf";
export const CHAT_ALLOWED = new Set(CHAT_ACCEPT.split(",").map((e) => e.slice(1)));
export const CHAT_MAX_FILE = 10 * 1024 * 1024;

export type FileKind = "image" | "pdf" | "word" | "excel" | "other";

const IMAGE_PREVIEW = new Set(["jpg", "jpeg", "png", "gif", "webp", "bmp"]);
const IMAGE_ANY = new Set([...IMAGE_PREVIEW, "heic", "heif", "tif", "tiff"]);

export function extOf(name: string): string {
  return name.includes(".") ? name.split(".").pop()!.toLowerCase() : "";
}

export function fileKind(name: string): FileKind {
  const ext = extOf(name);
  if (IMAGE_ANY.has(ext)) return "image";
  if (ext === "pdf") return "pdf";
  if (ext === "doc" || ext === "docx") return "word";
  if (ext === "xls" || ext === "xlsx" || ext === "csv") return "excel";
  return "other";
}

/** Bisa dipratinjau di dalam GPOS (bukan cuma diunduh). */
export function canPreview(name: string): boolean {
  const ext = extOf(name);
  return IMAGE_PREVIEW.has(ext) || ["pdf", "docx", "xlsx", "xls", "csv"].includes(ext);
}

export function isImagePreview(name: string): boolean {
  return IMAGE_PREVIEW.has(extOf(name));
}

const MIME: Record<string, string> = {
  jpg: "image/jpeg", jpeg: "image/jpeg", png: "image/png", gif: "image/gif", webp: "image/webp", bmp: "image/bmp",
  pdf: "application/pdf",
  docx: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
};

export function mimeOf(name: string): string {
  return MIME[extOf(name)] ?? "application/octet-stream";
}

/** Label pendek di ikon kartu file. */
export function kindLabel(name: string): string {
  const k = fileKind(name);
  if (k === "pdf") return "PDF";
  if (k === "word") return "DOC";
  if (k === "excel") return extOf(name) === "csv" ? "CSV" : "XLS";
  if (k === "image") return "IMG";
  return extOf(name).toUpperCase().slice(0, 4) || "FILE";
}

export function fmtSize(n: number | null | undefined): string {
  if (!n) return "";
  return n >= 1024 * 1024 ? `${(n / 1024 / 1024).toFixed(1)} MB` : `${Math.max(1, Math.round(n / 1024))} KB`;
}

/** Nama untuk gambar hasil Ctrl+V (screenshot), mis. "Screenshot 2026-10-19 19.45.12.png". */
export function screenshotName(type: string, at = new Date()): string {
  const ext = type.split("/")[1]?.replace("jpeg", "jpg") || "png";
  const p = (n: number) => String(n).padStart(2, "0");
  const date = `${at.getFullYear()}-${p(at.getMonth() + 1)}-${p(at.getDate())}`;
  const time = `${p(at.getHours())}.${p(at.getMinutes())}.${p(at.getSeconds())}`;
  return `Screenshot ${date} ${time}.${ext}`;
}

/** Alasan file ditolak, atau null kalau boleh dikirim. */
export function rejectReason(name: string, size: number): string | null {
  if (!CHAT_ALLOWED.has(extOf(name))) return `${name}: hanya gambar, Word, Excel, dan PDF yang bisa dikirim.`;
  if (size > CHAT_MAX_FILE) return `${name}: terlalu besar (maks 10 MB).`;
  if (size === 0) return `${name}: file kosong.`;
  return null;
}
