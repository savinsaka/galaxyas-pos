/**
 * Model template laporan (.Greport) v2 — berbasis ELEMEN BERPOSISI (canvas).
 * Satu template = daftar elemen dengan koordinat mm (x,y,w,h) relatif terhadap
 * area isi kertas (di dalam margin). Di-render ke:
 *  - DOM absolut (WYSIWYG) untuk pratinjau & cetak A4/PDF/custom, dan
 *  - byte ESC/POS untuk thermal (elemen diurutkan by-Y jadi baris teks).
 * Skema field yang boleh diikat (`bind`) ditentukan per `kind` di kinds.ts.
 */

export const GREPORT_VERSION = 2;

export type PaperPreset = "struk58" | "struk80" | "a4" | "a5" | "letter" | "custom";
export type Align = "left" | "center" | "right";
export type TextSize = "sm" | "md" | "lg";

export interface PaperConfig {
  preset: PaperPreset;
  widthMm: number;
  /** null = tinggi mengalir (struk/roll thermal). */
  heightMm: number | null;
  marginMm: number;
  fontPt: number;
  fontFamily?: string;
}

export interface PaperPresetDef {
  label: string;
  widthMm: number;
  heightMm: number | null;
  marginMm: number;
  fontPt: number;
}

export const PAPER_PRESETS: Record<Exclude<PaperPreset, "custom">, PaperPresetDef> & {
  custom: PaperPresetDef;
} = {
  struk58: { label: "Struk 58mm", widthMm: 58, heightMm: null, marginMm: 3, fontPt: 9 },
  struk80: { label: "Struk 80mm", widthMm: 80, heightMm: null, marginMm: 3, fontPt: 10.5 },
  a4: { label: "A4 (210×297)", widthMm: 210, heightMm: 297, marginMm: 15, fontPt: 10.5 },
  a5: { label: "A5 (148×210)", widthMm: 148, heightMm: 210, marginMm: 12, fontPt: 10 },
  letter: { label: "Letter (216×279)", widthMm: 216, heightMm: 279, marginMm: 15, fontPt: 10.5 },
  custom: { label: "Custom", widthMm: 100, heightMm: 150, marginMm: 5, fontPt: 10 },
};

export function paperFromPreset(preset: PaperPreset): PaperConfig {
  const d = PAPER_PRESETS[preset];
  return { preset, widthMm: d.widthMm, heightMm: d.heightMm, marginMm: d.marginMm, fontPt: d.fontPt };
}

/** Lebar area isi (di dalam margin kiri+kanan). */
export function contentWidthMm(p: PaperConfig): number {
  return Math.max(10, p.widthMm - p.marginMm * 2);
}

// ---- Elemen ----

interface ElementBase {
  id: string;
  /** Koordinat mm relatif ke area isi (di dalam margin). */
  x: number;
  y: number;
  w: number;
  /** Tinggi mm; opsional (teks auto). Dipakai untuk resize di canvas. */
  h?: number;
  align?: Align;
  bold?: boolean;
  size?: TextSize;
  hidden?: boolean;
}

/** Teks statis atau field terikat (mis. nama toko, header, footer, no. invoice). */
export interface TextElement extends ElementBase {
  type: "text";
  value?: string;
  bind?: string;
  prefix?: string;
}

/** Baris label : nilai (mis. Subtotal, TOTAL, Kembali). */
export interface PairElement extends ElementBase {
  type: "pair";
  label: string;
  labelBind?: string;
  bind: string;
  prefix?: string;
}

export interface ItemColumn {
  bind: string;
  label: string;
  align?: Align;
  widthPct?: number;
  show?: boolean;
  /** Jangan ikut dijumlah di baris Total (mis. kolom Harga satuan / Gross). */
  noTotal?: boolean;
}

/** Tabel berulang terikat satu dataset di konteks. */
export interface ItemsElement extends ElementBase {
  type: "items";
  source: string;
  columns: ItemColumn[];
  showHeader?: boolean;
  total?: boolean;
  heading?: string;
  layout?: "table" | "receipt";
}

export interface LineElement extends ElementBase {
  type: "line";
}

export interface SpacerElement extends ElementBase {
  type: "spacer";
  lines?: number;
}

export type Element = TextElement | PairElement | ItemsElement | LineElement | SpacerElement;

export interface ReportTemplate {
  greport: number;
  kind: string;
  name: string;
  paper: PaperConfig;
  elements: Element[];
}

let elSeq = 0;
/** Id elemen unik & stabil dalam satu sesi. */
export function newElId(prefix = "el"): string {
  elSeq += 1;
  return `${prefix}-${Date.now().toString(36)}-${elSeq}`;
}

// ---- Normalisasi & upgrade ----

function num(v: unknown, def: number): number {
  const n = typeof v === "number" ? v : Number(v);
  return Number.isFinite(n) ? n : def;
}
function clampNum(v: unknown, min: number, max: number, def: number): number {
  return Math.min(max, Math.max(min, num(v, def)));
}
function normalizeAlign(v: unknown): Align | undefined {
  return v === "left" || v === "center" || v === "right" ? v : undefined;
}
function normalizeSize(v: unknown): TextSize {
  return v === "sm" || v === "lg" ? v : "md";
}

function normalizePaper(raw: unknown): PaperConfig {
  const p = (raw && typeof raw === "object" ? raw : {}) as Record<string, unknown>;
  // Upgrade v1: paper.type "58"|"80"|"A4"|"auto" → preset.
  let preset = p.preset as PaperPreset | undefined;
  if (!preset) {
    const t = p.type;
    preset = t === "58" ? "struk58" : t === "80" ? "struk80" : t === "A4" || t === "auto" ? "a4" : "struk80";
  }
  if (!(preset in PAPER_PRESETS)) preset = "custom";
  const base = PAPER_PRESETS[preset];
  const heightRaw = "heightMm" in p ? p.heightMm : base.heightMm;
  return {
    preset,
    widthMm: clampNum(p.widthMm, 20, 2000, base.widthMm),
    heightMm: heightRaw === null || heightRaw === undefined ? base.heightMm : clampNum(heightRaw, 20, 5000, base.heightMm ?? 297),
    marginMm: clampNum(p.marginMm, 0, 60, base.marginMm),
    fontPt: clampNum(p.fontPt ?? p.fontSize, 6, 40, base.fontPt),
    fontFamily: typeof p.fontFamily === "string" ? p.fontFamily : undefined,
  };
}

function normalizeColumns(raw: unknown): ItemColumn[] {
  const cols = Array.isArray(raw) ? raw : [];
  return cols.map((c) => {
    const col = (c && typeof c === "object" ? c : {}) as Record<string, unknown>;
    return {
      bind: typeof col.bind === "string" ? col.bind : "",
      label: typeof col.label === "string" ? col.label : "",
      align: normalizeAlign(col.align),
      widthPct: typeof col.widthPct === "number" ? col.widthPct : undefined,
      show: col.show !== false,
      noTotal: col.noTotal === true,
    } satisfies ItemColumn;
  });
}

function normalizeElement(raw: unknown, fallbackW: number): Element | null {
  const b = (raw && typeof raw === "object" ? raw : {}) as Record<string, unknown>;
  const common = {
    id: typeof b.id === "string" && b.id ? b.id : newElId(),
    x: num(b.x, 0),
    y: num(b.y, 0),
    w: num(b.w, fallbackW),
    h: b.h === undefined ? undefined : num(b.h, 6),
    align: normalizeAlign(b.align),
    bold: b.bold === true,
    size: normalizeSize(b.size),
    hidden: b.hidden === true,
  };
  switch (b.type) {
    case "text":
      return { ...common, type: "text", value: typeof b.value === "string" ? b.value : undefined, bind: typeof b.bind === "string" ? b.bind : undefined, prefix: typeof b.prefix === "string" ? b.prefix : undefined };
    case "pair":
      return { ...common, type: "pair", label: typeof b.label === "string" ? b.label : "", labelBind: typeof b.labelBind === "string" ? b.labelBind : undefined, bind: typeof b.bind === "string" ? b.bind : "", prefix: typeof b.prefix === "string" ? b.prefix : undefined };
    case "items":
      return { ...common, type: "items", source: typeof b.source === "string" && b.source ? b.source : "items", heading: typeof b.heading === "string" ? b.heading : undefined, showHeader: b.showHeader !== false, total: b.total === true, layout: b.layout === "receipt" ? "receipt" : "table", columns: normalizeColumns(b.columns) };
    case "line":
      return { ...common, type: "line" };
    case "spacer":
      return { ...common, type: "spacer", lines: typeof b.lines === "number" ? b.lines : 1 };
    default:
      return null;
  }
}

/** Tinggi default (mm) sebuah band v1 saat di-stack jadi elemen v2. */
function upgradeHeight(type: unknown, lines: unknown): number {
  if (type === "items") return 40;
  if (type === "line") return 2;
  if (type === "spacer") return Math.max(1, num(lines, 1)) * 4;
  return 6;
}

/**
 * Tahan file lama/rusak; upgrade v1 (array `bands`) → v2 dengan menumpuk elemen
 * secara vertikal selebar area isi. Tidak melempar.
 */
export function normalizeTemplate(raw: unknown, fallbackKind: string): ReportTemplate {
  const obj = (raw && typeof raw === "object" ? raw : {}) as Record<string, unknown>;
  const paper = normalizePaper(obj.paper);
  const cw = contentWidthMm(paper);

  let elements: Element[];
  if (Array.isArray(obj.elements)) {
    elements = obj.elements.map((e) => normalizeElement(e, cw)).filter((e): e is Element => e !== null);
  } else if (Array.isArray(obj.bands)) {
    // Upgrade v1: band berurutan → elemen full-width bertumpuk.
    let cursor = 0;
    elements = (obj.bands as unknown[])
      .map((band) => {
        const bb = (band && typeof band === "object" ? band : {}) as Record<string, unknown>;
        const h = upgradeHeight(bb.type, bb.lines);
        const el = normalizeElement({ ...bb, x: 0, y: cursor, w: cw, h }, cw);
        cursor += h + 1;
        return el;
      })
      .filter((e): e is Element => e !== null);
  } else {
    elements = [];
  }

  return {
    greport: typeof obj.greport === "number" ? obj.greport : GREPORT_VERSION,
    kind: typeof obj.kind === "string" && obj.kind ? obj.kind : fallbackKind,
    name: typeof obj.name === "string" && obj.name ? obj.name : "Tanpa Nama",
    paper,
    elements,
  };
}

export function templateToJson(t: ReportTemplate): string {
  return JSON.stringify(t, null, 2);
}
