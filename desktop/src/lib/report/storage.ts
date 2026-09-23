/**
 * Simpan/baca template .Greport lewat command Rust. File dipisah PER-JENIS di
 * `Documents\GalaxyAS POS\Reports\<kategori>\<kind>\` supaya daftar tiap jenis
 * laporan tidak tercampur (mis. struk vs item-masuk di kategori "kasir").
 * Juga mengelola "template aktif" per kind (di settings) & seeding default.
 */
import { api } from "$lib/api";
import { findKind } from "./kinds";
import { defaultTemplate } from "./defaults";
import { normalizeTemplate, templateToJson, type ReportTemplate } from "./template";

/** Kategori folder untuk sebuah kind (dari registri). */
export function categoryOf(kind: string): string {
  return findKind(kind)?.category ?? "umum";
}

const activeKey = (kind: string) => `report_template_${kind}`;

export async function listTemplates(kind: string): Promise<string[]> {
  return api.listReportTemplates(categoryOf(kind), kind);
}

export async function readTemplate(kind: string, name: string): Promise<ReportTemplate> {
  const raw = await api.readReportTemplate(categoryOf(kind), kind, name);
  return normalizeTemplate(JSON.parse(raw), kind);
}

export async function saveTemplate(template: ReportTemplate): Promise<void> {
  await api.writeReportTemplate(categoryOf(template.kind), template.kind, template.name, templateToJson(template));
}

export async function deleteTemplate(kind: string, name: string): Promise<void> {
  await api.deleteReportTemplate(categoryOf(kind), kind, name);
}

export async function renameTemplate(kind: string, oldName: string, newName: string): Promise<void> {
  await api.renameReportTemplate(categoryOf(kind), kind, oldName, newName);
}

export async function duplicateTemplate(kind: string, name: string, newName: string): Promise<void> {
  await api.duplicateReportTemplate(categoryOf(kind), kind, name, newName);
}

/**
 * Pastikan template default sebuah kind ada di disk (tanpa menimpa editan user).
 * Mengembalikan nama template default, atau null bila kind tak punya default.
 */
export async function ensureDefault(kind: string): Promise<string | null> {
  const def = defaultTemplate(kind);
  if (!def) return null;
  const existing = await listTemplates(kind);
  if (!existing.includes(def.name)) {
    await api.writeReportTemplate(categoryOf(kind), kind, def.name, templateToJson(def));
  }
  return def.name;
}

/** Nama template aktif untuk kind (dari settings), fallback ke default. */
export async function getActiveTemplateName(kind: string): Promise<string | null> {
  const settings = await api.getSettings();
  const saved = settings[activeKey(kind)];
  if (saved) return saved;
  return ensureDefault(kind);
}

export async function setActiveTemplateName(kind: string, name: string): Promise<void> {
  await api.updateSetting(activeKey(kind), name);
}

/**
 * True bila user SUDAH memilih template aktif (setting terisi). Gerbang opt-in:
 * selama belum dipilih, cetak tetap memakai jalur bawaan lama.
 */
export async function hasActiveTemplate(kind: string): Promise<boolean> {
  const settings = await api.getSettings();
  return !!settings[activeKey(kind)];
}

/** Muat template aktif sebuah kind (seed default bila perlu), selalu bisa dipakai. */
export async function loadActiveTemplate(kind: string): Promise<ReportTemplate> {
  const name = await getActiveTemplateName(kind);
  if (name) {
    try {
      return await readTemplate(kind, name);
    } catch {
      // File tercatat aktif tapi hilang/rusak — jatuh ke default.
    }
  }
  const def = defaultTemplate(kind);
  if (def) return def;
  throw new Error(`Tidak ada template untuk jenis laporan: ${kind}`);
}
