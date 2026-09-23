/**
 * Renderer template laporan v2 (elemen berposisi). Dua target:
 *  - `renderToEscPos`: elemen diurutkan by-Y → baris teks ESC/POS (cetak thermal
 *    tajam; struk/dok tetap byte-identik dengan jalur lama untuk template default).
 *  - `renderToDom`: elemen `position:absolute` (WYSIWYG) untuk pratinjau canvas &
 *    cetak A4/PDF/custom.
 * Keduanya dari template yang sama → tampilan editor == hasil cetak.
 */
import { EscPosBuilder, money, formatQty } from "$lib/escpos";
import { paperCols } from "$lib/receipt";
import { contentWidthMm, type Element, type ItemsElement, type ReportTemplate, type TextElement } from "./template";
import { fieldFormats, findKind, type FieldFormat, type ReportContext } from "./kinds";

export function formatValue(raw: unknown, fmt: FieldFormat | undefined): string {
  if (raw === null || raw === undefined) return "";
  switch (fmt) {
    case "money": return money(Number(raw) || 0);
    case "qty": return formatQty(Number(raw) || 0);
    case "int": return String(Math.round(Number(raw) || 0));
    default: return String(raw);
  }
}

function formatsFor(ctx: ReportContext): Record<string, FieldFormat> {
  const kind = findKind(ctx.kind);
  return kind ? fieldFormats(kind) : {};
}

function scalarText(el: TextElement, ctx: ReportContext, fmts: Record<string, FieldFormat>): string {
  if (el.bind) {
    const base = formatValue(ctx.scalars[el.bind], fmts[el.bind]);
    // Field terikat kosong → dilewati (termasuk prefix), sama seperti struk lama.
    return base ? (el.prefix ?? "") + base : "";
  }
  return (el.prefix ?? "") + (el.value ?? "");
}

function visibleSorted(els: Element[]): Element[] {
  // Urutan Y menentukan urutan baris cetak thermal; stabil untuk Y sama.
  return els.filter((e) => !e.hidden).map((e, i) => [e, i] as const)
    .sort((a, b) => a[0].y - b[0].y || a[1] - b[1])
    .map(([e]) => e);
}

// ---------------- ESC/POS (thermal) ----------------

function thermalCols(t: ReportTemplate): number {
  return paperCols(t.paper.widthMm <= 58 ? "58" : "80");
}

export function renderToEscPos(template: ReportTemplate, ctx: ReportContext): Uint8Array {
  const w = thermalCols(template);
  const fmts = formatsFor(ctx);
  const rule = () => "-".repeat(w);
  const two = (l: string, r: string) => {
    const left = l.slice(0, Math.max(0, w - r.length - 1));
    const space = Math.max(1, w - left.length - r.length);
    return left + " ".repeat(space) + r;
  };

  const b = new EscPosBuilder();
  b.init();

  for (const el of visibleSorted(template.elements)) {
    switch (el.type) {
      case "text": {
        const text = scalarText(el, ctx, fmts);
        b.align(el.align ?? "left");
        el.bold && b.bold(true);
        el.size === "lg" && b.doubleHeight(true);
        text.split("\n").map((x) => x.trim()).filter(Boolean)
          .forEach((ln) => b.line(ln.slice(0, el.size === "lg" ? Math.ceil(w / 2) : w)));
        el.size === "lg" && b.doubleHeight(false);
        el.bold && b.bold(false);
        break;
      }
      case "pair": {
        const label = el.labelBind ? formatValue(ctx.scalars[el.labelBind], fmts[el.labelBind]) : el.label;
        const val = (el.prefix ?? "") + formatValue(ctx.scalars[el.bind], fmts[el.bind]);
        b.align("left").bold(!!el.bold).line(two(label, val)).bold(false);
        break;
      }
      case "line":
        b.align("left").line(rule());
        break;
      case "spacer":
        b.feed(Math.max(1, el.lines ?? 1));
        break;
      case "items":
        renderItemsEscPos(b, el, ctx, fmts, w, two);
        break;
    }
  }

  b.align("center").feed(3).cut(true);
  return b.build();
}

function renderItemsEscPos(
  b: EscPosBuilder,
  el: ItemsElement,
  ctx: ReportContext,
  fmts: Record<string, FieldFormat>,
  w: number,
  two: (l: string, r: string) => string,
) {
  const rows = ctx.datasets[el.source] ?? [];
  const cols = el.columns.filter((c) => c.show !== false);
  b.align("left");
  if (el.heading) b.bold(true).line(el.heading.slice(0, w)).bold(false);

  if (el.layout === "receipt") {
    for (const row of rows) {
      const name = String(row["name"] ?? row["product_name"] ?? "");
      const qty = Number(row["qty"] ?? 0);
      const price = Number(row["price"] ?? 0);
      const lineTotal = row["line_total"] !== undefined ? Number(row["line_total"]) : qty * price;
      const discount = Number(row["discount"] ?? 0);
      const note = String(row["note"] ?? "");
      b.line(name.slice(0, w));
      if (row["price"] !== undefined) {
        b.line(two(`  ${formatQty(qty)} x ${money(price)}`, money(lineTotal)));
        if (discount > 0) b.line(two("  Diskon", "-" + money(discount)));
      } else {
        b.line(two(`  Qty: ${formatQty(qty)}`, ""));
        if (note) b.line(`  Ket: ${note}`.slice(0, w));
      }
    }
    return;
  }

  for (const row of rows) {
    const [first, ...rest] = cols;
    if (first) b.line(formatValue(row[first.bind], fmts[`${el.source}.${first.bind}`]).slice(0, w));
    for (const c of rest) {
      const val = formatValue(row[c.bind], fmts[`${el.source}.${c.bind}`]);
      if (val) b.line(two(`  ${c.label}`, val));
    }
  }
}

// ---------------- DOM (canvas / A4 absolut) ----------------

const SIZE_EM: Record<string, string> = { sm: "0.85em", md: "1em", lg: "1.3em" };

/**
 * Bangun elemen `#print-root` (lebar = area isi kertas, mm) dengan tiap elemen
 * di-posisikan absolut sesuai (x,y,w) — WYSIWYG. Dipakai pratinjau canvas & cetak.
 */
export function renderToDom(template: ReportTemplate, ctx: ReportContext): HTMLElement {
  const fmts = formatsFor(ctx);
  const cw = contentWidthMm(template.paper);
  const root = document.createElement("div");
  root.id = "print-root";
  root.style.position = "relative";
  root.style.width = `${cw}mm`;
  // Tinggi: kertas tetap (A4 dst) → tinggi area isi; struk (null) → auto dari elemen.
  if (template.paper.heightMm != null) {
    root.style.minHeight = `${template.paper.heightMm - template.paper.marginMm * 2}mm`;
  }

  for (const el of template.elements.filter((e) => !e.hidden)) {
    const box = document.createElement("div");
    box.style.position = "absolute";
    box.style.left = `${el.x}mm`;
    box.style.top = `${el.y}mm`;
    box.style.width = `${el.w}mm`;
    if (el.h != null) box.style.minHeight = `${el.h}mm`;
    box.style.textAlign = el.align ?? "left";
    if (el.bold) box.style.fontWeight = "700";
    if (el.size) box.style.fontSize = SIZE_EM[el.size];

    if (el.type === "text") {
      box.textContent = scalarText(el, ctx, fmts);
      box.style.whiteSpace = "pre-line";
    } else if (el.type === "pair") {
      box.style.display = "flex";
      box.style.justifyContent = "space-between";
      box.style.gap = "1em";
      const label = el.labelBind ? formatValue(ctx.scalars[el.labelBind], fmts[el.labelBind]) : el.label;
      const value = (el.prefix ?? "") + formatValue(ctx.scalars[el.bind], fmts[el.bind]);
      const l = document.createElement("span"); l.textContent = label;
      const r = document.createElement("span"); r.textContent = value;
      box.append(l, r);
    } else if (el.type === "line") {
      const hr = document.createElement("hr");
      hr.style.margin = "0";
      box.appendChild(hr);
    } else if (el.type === "spacer") {
      // kosong (hanya menempati ruang)
    } else if (el.type === "items") {
      box.appendChild(itemsTable(el, ctx, fmts));
    }
    root.appendChild(box);
  }
  return root;
}

function itemsTable(el: ItemsElement, ctx: ReportContext, fmts: Record<string, FieldFormat>): HTMLElement {
  const wrap = document.createElement("div");
  if (el.heading) {
    const h = document.createElement("div");
    const b = document.createElement("b");
    b.textContent = el.heading;
    h.appendChild(b);
    wrap.appendChild(h);
  }
  const rows = ctx.datasets[el.source] ?? [];
  const cols = el.columns.filter((c) => c.show !== false);
  const table = document.createElement("table");
  table.style.width = "100%";

  if (el.showHeader !== false) {
    const thead = document.createElement("thead");
    const tr = document.createElement("tr");
    for (const c of cols) {
      const th = document.createElement("th");
      th.textContent = c.label;
      if (c.align) th.style.textAlign = c.align;
      if (c.widthPct) th.style.width = `${c.widthPct}%`;
      tr.appendChild(th);
    }
    thead.appendChild(tr);
    table.appendChild(thead);
  }

  const tbody = document.createElement("tbody");
  if (rows.length === 0) {
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = Math.max(1, cols.length);
    td.textContent = "Tidak ada data";
    td.className = "text-dim";
    tr.appendChild(td);
    tbody.appendChild(tr);
  } else {
    for (const row of rows) {
      const tr = document.createElement("tr");
      for (const c of cols) {
        const td = document.createElement("td");
        td.textContent = formatValue(row[c.bind], fmts[`${el.source}.${c.bind}`]);
        if (c.align) td.style.textAlign = c.align;
        tr.appendChild(td);
      }
      tbody.appendChild(tr);
    }
  }
  table.appendChild(tbody);

  if (el.total && rows.length) {
    const tfoot = document.createElement("tfoot");
    const tr = document.createElement("tr");
    cols.forEach((c, idx) => {
      const td = document.createElement("td");
      const fmt = fmts[`${el.source}.${c.bind}`];
      if (idx === 0) { td.textContent = "Total"; td.style.fontWeight = "700"; }
      else if (!c.noTotal && (fmt === "money" || fmt === "qty" || fmt === "int")) {
        const sum = rows.reduce((s, r) => s + (Number(r[c.bind]) || 0), 0);
        td.textContent = formatValue(sum, fmt); td.style.fontWeight = "700";
      }
      if (c.align) td.style.textAlign = c.align;
      tr.appendChild(td);
    });
    tfoot.appendChild(tr);
    table.appendChild(tfoot);
  }

  wrap.appendChild(table);
  return wrap;
}
