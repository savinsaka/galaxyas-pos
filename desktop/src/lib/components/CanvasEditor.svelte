<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { fieldFormats, findKind, sampleContext, type FieldFormat, type ReportContext } from "$lib/report/kinds";
  import { previewContext } from "$lib/report/data";
  import { formatValue } from "$lib/report/render";
  import {
    PAPER_PRESETS,
    paperFromPreset,
    contentWidthMm,
    newElId,
    type Element,
    type ItemsElement,
    type PairElement,
    type PaperPreset,
    type ReportTemplate,
    type SpacerElement,
    type TextElement,
  } from "$lib/report/template";

  let {
    template,
    onSave,
    onClose,
  }: {
    template: ReportTemplate;
    onSave: (t: ReportTemplate) => void;
    onClose: () => void;
  } = $props();

  // Salinan lokal yang diedit (snapshot supaya tak mengubah objek pemanggil).
  let tpl = $state<ReportTemplate>(structuredClone($state.snapshot(template)) as ReportTemplate);
  let selectedId = $state<string | null>(tpl.elements[0]?.id ?? null);
  let scale = $state(3); // px per mm (zoom)

  // Undo/redo (snapshot JSON) + clipboard elemen.
  let past: string[] = [];
  let future: string[] = [];
  let clip = $state<Element | null>(null);

  /** Rekam state SEBELUM mutasi (dipanggil di awal aksi & saat mulai drag). */
  function record() {
    const snap = JSON.stringify($state.snapshot(tpl));
    if (past.length && past[past.length - 1] === snap) return; // dedup
    past.push(snap);
    if (past.length > 80) past.shift();
    future = [];
  }
  function undo() {
    if (!past.length) return;
    future.push(JSON.stringify($state.snapshot(tpl)));
    tpl = JSON.parse(past.pop()!) as ReportTemplate;
    if (!tpl.elements.some((e) => e.id === selectedId)) selectedId = tpl.elements[0]?.id ?? null;
  }
  function redo() {
    if (!future.length) return;
    past.push(JSON.stringify($state.snapshot(tpl)));
    tpl = JSON.parse(future.pop()!) as ReportTemplate;
    if (!tpl.elements.some((e) => e.id === selectedId)) selectedId = tpl.elements[0]?.id ?? null;
  }
  function copySel() {
    if (selected) clip = structuredClone($state.snapshot(selected)) as Element;
  }
  function pasteClip() {
    if (!clip) return;
    record();
    const el = { ...(structuredClone(clip) as Element), id: newElId(), x: Math.min(cw - 4, clip.x + 3), y: clip.y + 3 };
    tpl.elements = [...tpl.elements, el];
    selectedId = el.id;
  }
  function duplicateSel() {
    copySel();
    pasteClip();
  }

  const kind = $derived(findKind(tpl.kind)!);
  // Pratinjau pakai setelan toko nyata (struk asli), item tetap contoh.
  let ctx = $state<ReportContext>(sampleContext(tpl.kind));
  onMount(async () => { ctx = await previewContext(tpl.kind); });
  const fmts = $derived<Record<string, FieldFormat>>(kind ? fieldFormats(kind) : {});
  const cw = $derived(contentWidthMm(tpl.paper));
  const selected = $derived(tpl.elements.find((e) => e.id === selectedId) ?? null);
  const contentHeightMm = $derived(
    tpl.paper.heightMm != null
      ? tpl.paper.heightMm - tpl.paper.marginMm * 2
      : Math.max(60, ...tpl.elements.map((e) => e.y + (e.h ?? 6) + 5)),
  );

  // ---- tampilan isi elemen (WYSIWYG, pakai formatter yang sama dgn cetak) ----
  function textOf(el: TextElement): string {
    if (el.bind) {
      const base = formatValue(ctx.scalars[el.bind], fmts[el.bind]);
      return base ? (el.prefix ?? "") + base : `⟨${fieldLabel(el.bind)}⟩`;
    }
    return (el.prefix ?? "") + (el.value ?? "");
  }
  function pairLabel(el: PairElement): string {
    return el.labelBind ? formatValue(ctx.scalars[el.labelBind], fmts[el.labelBind]) || `⟨${fieldLabel(el.labelBind)}⟩` : el.label;
  }
  function pairValue(el: PairElement): string {
    return (el.prefix ?? "") + (formatValue(ctx.scalars[el.bind], fmts[el.bind]) || `⟨${fieldLabel(el.bind)}⟩`);
  }
  function fieldLabel(id: string): string {
    return kind?.scalars.find((f) => f.id === id)?.label ?? id;
  }
  function rowsOf(el: ItemsElement) {
    return ctx.datasets[el.source] ?? [];
  }
  function colsOf(el: ItemsElement) {
    return el.columns.filter((c) => c.show !== false);
  }
  function cellText(el: ItemsElement, row: Record<string, unknown>, bind: string): string {
    return formatValue(row[bind], fmts[`${el.source}.${bind}`]);
  }
  function datasetFields(source: string) {
    return kind?.datasets.find((d) => d.id === source)?.fields ?? [];
  }

  // ---- drag & resize (pointer) ----
  let drag: { id: string; mode: "move" | "resize"; sx: number; sy: number; ox: number; oy: number; ow: number; oh: number } | null = null;

  function startDrag(e: PointerEvent, el: Element, mode: "move" | "resize") {
    e.stopPropagation();
    selectedId = el.id;
    record();
    drag = { id: el.id, mode, sx: e.clientX, sy: e.clientY, ox: el.x, oy: el.y, ow: el.w, oh: el.h ?? 6 };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", endDrag);
  }
  function onMove(e: PointerEvent) {
    if (!drag) return;
    const el = tpl.elements.find((x) => x.id === drag!.id);
    if (!el) return;
    const dxMm = Math.round((e.clientX - drag.sx) / scale);
    const dyMm = Math.round((e.clientY - drag.sy) / scale);
    if (drag.mode === "move") {
      el.x = Math.max(0, Math.min(cw - 2, drag.ox + dxMm));
      el.y = Math.max(0, drag.oy + dyMm);
    } else {
      el.w = Math.max(6, Math.min(cw - el.x, drag.ow + dxMm));
      el.h = Math.max(3, drag.oh + dyMm);
    }
  }
  function endDrag() {
    drag = null;
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", endDrag);
  }

  // ---- tambah / hapus elemen ----
  function nextY(): number {
    return tpl.elements.length ? Math.max(...tpl.elements.map((e) => e.y + (e.h ?? 6))) + 2 : 0;
  }
  function addElement(type: Element["type"]) {
    record();
    const y = nextY();
    const base = { id: newElId(), x: 0, y, w: cw, align: "left" as const };
    let el: Element;
    if (type === "text") el = { ...base, type: "text", value: "Teks", h: 6 };
    else if (type === "pair") el = { ...base, type: "pair", label: "Label", bind: "", h: 6 };
    else if (type === "items") el = { ...base, type: "items", source: kind.datasets[0]?.id ?? "items", columns: [], showHeader: true, layout: kind.shape === "receipt" ? "receipt" : "table", h: 40 };
    else if (type === "spacer") el = { ...base, type: "spacer", lines: 1, h: 4 };
    else el = { ...base, type: "line", h: 3 };
    tpl.elements = [...tpl.elements, el];
    selectedId = el.id;
  }
  function removeSelected() {
    if (!selected) return;
    record();
    tpl.elements = tpl.elements.filter((e) => e.id !== selected.id);
    selectedId = tpl.elements[0]?.id ?? null;
  }

  // ---- kolom (items) ----
  function addColumn(el: ItemsElement) {
    record();
    const ds = kind.datasets.find((d) => d.id === el.source);
    const used = new Set(el.columns.map((c) => c.bind));
    const f = ds?.fields.find((x) => !used.has(x.id)) ?? ds?.fields[0];
    if (!f) return;
    el.columns = [...el.columns, { bind: f.id, label: f.label, show: true, align: "left" }];
  }
  function removeColumn(el: ItemsElement, i: number) {
    record();
    el.columns = el.columns.filter((_, idx) => idx !== i);
  }

  // ---- kertas ----
  function applyPreset(preset: PaperPreset) {
    record();
    const keepEls = tpl.elements;
    tpl.paper = paperFromPreset(preset);
    tpl.elements = keepEls; // elemen tetap; user sesuaikan lebar bila perlu
  }

  function onKeydown(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    const inField = !!t && (t.tagName === "INPUT" || t.tagName === "SELECT" || t.tagName === "TEXTAREA" || t.isContentEditable);
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key.toLowerCase() === "z") { e.preventDefault(); e.shiftKey ? redo() : undo(); return; }
    if (mod && e.key.toLowerCase() === "y") { e.preventDefault(); redo(); return; }
    if (inField) return; // dalam input teks: biarkan copy/paste/hapus native
    if (e.key === "Delete" || e.key === "Backspace") { e.preventDefault(); removeSelected(); return; }
    if (mod && e.key.toLowerCase() === "c") { e.preventDefault(); copySel(); return; }
    if (mod && e.key.toLowerCase() === "v") { e.preventDefault(); pasteClip(); return; }
    if (mod && e.key.toLowerCase() === "d") { e.preventDefault(); duplicateSel(); return; }
    if (selected && ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) {
      e.preventDefault();
      record();
      const step = e.shiftKey ? 5 : 1;
      if (e.key === "ArrowLeft") selected.x = Math.max(0, selected.x - step);
      else if (e.key === "ArrowRight") selected.x = Math.min(cw - 2, selected.x + step);
      else if (e.key === "ArrowUp") selected.y = Math.max(0, selected.y - step);
      else selected.y = selected.y + step;
    }
  }
  onMount(() => window.addEventListener("keydown", onKeydown));
  onDestroy(() => window.removeEventListener("keydown", onKeydown));

  const PRESET_KEYS = Object.keys(PAPER_PRESETS) as PaperPreset[];
</script>

<div class="ce-root">
  <!-- Toolbar -->
  <div class="ce-toolbar card">
    <input class="name-input" bind:value={tpl.name} title="Nama template" />
    <span class="sep"></span>
    <button class="btn-ghost mini" onclick={() => addElement("text")}>+ Teks</button>
    <button class="btn-ghost mini" onclick={() => addElement("pair")}>+ Label:Nilai</button>
    <button class="btn-ghost mini" onclick={() => addElement("items")}>+ Tabel</button>
    <button class="btn-ghost mini" onclick={() => addElement("line")}>+ Garis</button>
    <span class="sep"></span>
    <button class="btn-ghost mini" title="Undo (Ctrl+Z)" onclick={undo}>↶</button>
    <button class="btn-ghost mini" title="Redo (Ctrl+Y)" onclick={redo}>↷</button>
    <button class="btn-ghost mini" title="Salin elemen (Ctrl+C)" onclick={copySel} disabled={!selected}>⧉ Salin</button>
    <button class="btn-ghost mini" title="Tempel (Ctrl+V)" onclick={pasteClip} disabled={!clip}>📋 Tempel</button>
    <button class="btn-ghost mini danger" title="Hapus (Del)" onclick={removeSelected} disabled={!selected}>🗑️</button>
    <span class="sep"></span>
    <label class="zoom">Zoom
      <input type="range" min="2" max="6" step="0.5" bind:value={scale} />
    </label>
    <div class="tb-right">
      <button class="btn-ghost" onclick={onClose}>Tutup</button>
      <button class="btn-primary" onclick={() => onSave($state.snapshot(tpl) as ReportTemplate)}>💾 Simpan</button>
    </div>
  </div>

  <div class="ce-body">
    <!-- Canvas -->
    <div class="ce-canvas-wrap">
      <div
        class="paper"
        role="presentation"
        style="width:{tpl.paper.widthMm * scale}px; padding:{tpl.paper.marginMm * scale}px;"
        onpointerdown={() => (selectedId = null)}
      >
        <div class="content" style="width:{cw * scale}px; height:{contentHeightMm * scale}px;">
          {#each tpl.elements as el (el.id)}
            <div
              class="el"
              class:sel={el.id === selectedId}
              class:dim={el.hidden}
              role="button"
              tabindex="0"
              style="left:{el.x * scale}px; top:{el.y * scale}px; width:{el.w * scale}px; min-height:{(el.h ?? 6) * scale}px; text-align:{el.align ?? 'left'}; font-weight:{el.bold ? 700 : 400}; font-size:{el.size === 'lg' ? 1.3 : el.size === 'sm' ? 0.85 : 1}em;"
              onpointerdown={(e) => startDrag(e, el, "move")}
            >
              {#if el.type === "text"}
                <span class="pre">{textOf(el)}</span>
              {:else if el.type === "pair"}
                <div class="pair"><span>{pairLabel(el)}</span><span>{pairValue(el)}</span></div>
              {:else if el.type === "line"}
                <hr />
              {:else if el.type === "spacer"}
                <span class="ghost">␣ {el.lines ?? 1} baris</span>
              {:else if el.type === "items"}
                <table>
                  {#if el.showHeader !== false}
                    <thead><tr>{#each colsOf(el) as c}<th style="text-align:{c.align ?? 'left'}">{c.label}</th>{/each}</tr></thead>
                  {/if}
                  <tbody>
                    {#each rowsOf(el).slice(0, 4) as row}
                      <tr>{#each colsOf(el) as c}<td style="text-align:{c.align ?? 'left'}">{cellText(el, row, c.bind)}</td>{/each}</tr>
                    {/each}
                  </tbody>
                </table>
              {/if}
              {#if el.id === selectedId}
                <span class="rz" role="presentation" onpointerdown={(e) => startDrag(e, el, "resize")}></span>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    </div>

    <!-- Properti -->
    <div class="ce-props card" onfocusin={record}>
      <div class="col-title">Kertas</div>
      <div class="prop">
        <label>Preset</label>
        <select value={tpl.paper.preset} onchange={(e) => applyPreset((e.currentTarget as HTMLSelectElement).value as PaperPreset)}>
          {#each PRESET_KEYS as p}<option value={p}>{PAPER_PRESETS[p].label}</option>{/each}
        </select>
      </div>
      <div class="grid3">
        <div class="prop"><label>Lebar mm</label><input type="number" min="20" bind:value={tpl.paper.widthMm} /></div>
        <div class="prop">
          <label>Tinggi mm</label>
          <input type="number" min="0" placeholder="auto"
            value={tpl.paper.heightMm ?? ""}
            onchange={(e) => { const v = (e.currentTarget as HTMLInputElement).value; tpl.paper.heightMm = v === "" ? null : Number(v); }} />
        </div>
        <div class="prop"><label>Margin mm</label><input type="number" min="0" bind:value={tpl.paper.marginMm} /></div>
      </div>
      <div class="prop"><label>Font (pt)</label><input type="number" min="6" max="40" step="0.5" bind:value={tpl.paper.fontPt} /></div>
      <div class="text-dim tiny">Tinggi kosong = mengalir (struk/roll thermal).</div>
      <hr />

      {#if !selected}
        <div class="text-dim">Klik sebuah elemen di kertas untuk mengaturnya, atau tambah lewat toolbar.</div>
      {:else}
        <div class="col-title row-between">
          Elemen: {selected.type}
          <button class="icon-btn" title="Hapus elemen" onclick={removeSelected}>🗑️</button>
        </div>
        <div class="grid4">
          <div class="prop"><label>X</label><input type="number" bind:value={selected.x} /></div>
          <div class="prop"><label>Y</label><input type="number" bind:value={selected.y} /></div>
          <div class="prop"><label>W</label><input type="number" bind:value={selected.w} /></div>
          <div class="prop"><label>H</label><input type="number" bind:value={selected.h} /></div>
        </div>
        <div class="row-2">
          <div class="prop">
            <label>Rata</label>
            <select bind:value={selected.align}><option value="left">Kiri</option><option value="center">Tengah</option><option value="right">Kanan</option></select>
          </div>
          <div class="prop">
            <label>Ukuran</label>
            <select bind:value={selected.size}><option value="sm">Kecil</option><option value="md">Normal</option><option value="lg">Besar</option></select>
          </div>
        </div>
        <label class="chk"><input type="checkbox" bind:checked={selected.bold} /> Tebal</label>

        {#if selected.type === "text"}
          {@const te = selected as TextElement}
          <div class="prop">
            <label>Isi dari field</label>
            <select bind:value={te.bind}>
              <option value={undefined}>— teks statis —</option>
              {#each kind.scalars as f}<option value={f.id}>{f.label}</option>{/each}
            </select>
          </div>
          {#if !te.bind}<div class="prop"><label>Teks</label><input bind:value={te.value} /></div>{/if}
          <div class="prop"><label>Awalan</label><input bind:value={te.prefix} placeholder="mis. NPWP: " /></div>
        {:else if selected.type === "pair"}
          {@const pe = selected as PairElement}
          <div class="prop">
            <label>Label dari field</label>
            <select bind:value={pe.labelBind}>
              <option value={undefined}>— teks statis —</option>
              {#each kind.scalars as f}<option value={f.id}>{f.label}</option>{/each}
            </select>
          </div>
          {#if !pe.labelBind}<div class="prop"><label>Label</label><input bind:value={pe.label} /></div>{/if}
          <div class="prop">
            <label>Nilai dari field</label>
            <select bind:value={pe.bind}>
              <option value="">—</option>
              {#each kind.scalars as f}<option value={f.id}>{f.label}</option>{/each}
            </select>
          </div>
          <div class="prop"><label>Awalan nilai</label><input bind:value={pe.prefix} placeholder='mis. "-"' /></div>
        {:else if selected.type === "items"}
          {@const ie = selected as ItemsElement}
          <div class="prop">
            <label>Sumber data</label>
            <select bind:value={ie.source}>{#each kind.datasets as d}<option value={d.id}>{d.label}</option>{/each}</select>
          </div>
          <div class="prop">
            <label>Tata letak</label>
            <select bind:value={ie.layout}><option value="table">Tabel (A4)</option><option value="receipt">Struk (thermal)</option></select>
          </div>
          <div class="prop"><label>Judul</label><input bind:value={ie.heading} placeholder="opsional" /></div>
          <label class="chk"><input type="checkbox" bind:checked={ie.showHeader} /> Header kolom</label>
          <label class="chk"><input type="checkbox" bind:checked={ie.total} /> Baris Total</label>
          <div class="cols-title">Kolom</div>
          {#each ie.columns as col, i (i)}
            <div class="col-edit">
              <input class="c-label" bind:value={col.label} placeholder="Judul" />
              <select class="c-bind" bind:value={col.bind}>{#each datasetFields(ie.source) as f}<option value={f.id}>{f.label}</option>{/each}</select>
              <select class="c-align" bind:value={col.align}><option value="left">◧</option><option value="center">▣</option><option value="right">◨</option></select>
              <label class="chk sm"><input type="checkbox" bind:checked={col.show} /></label>
              <button class="icon-btn" onclick={() => removeColumn(ie, i)}>✖</button>
            </div>
          {/each}
          <button class="btn-ghost mini" onclick={() => addColumn(ie)}>+ Kolom</button>
        {:else if selected.type === "spacer"}
          {@const se = selected as SpacerElement}
          <div class="prop"><label>Baris kosong</label><input type="number" min="1" max="10" bind:value={se.lines} /></div>
        {/if}
      {/if}
    </div>
  </div>
</div>

<style>
  .ce-root { display: flex; flex-direction: column; gap: 0.6rem; height: 100%; }
  .ce-toolbar { display: flex; align-items: center; gap: 0.4rem; flex-wrap: wrap; padding: 0.5rem 0.7rem; }
  .name-input { font-weight: 600; min-width: 160px; }
  .sep { width: 1px; align-self: stretch; background: var(--border); margin: 0 0.3rem; }
  .btn-ghost.mini { font-size: 0.75rem; padding: 0.25rem 0.5rem; }
  .zoom { font-size: 0.75rem; color: var(--text-dim); display: flex; align-items: center; gap: 0.3rem; }
  .tb-right { margin-left: auto; display: flex; gap: 0.4rem; }

  .ce-body { display: grid; grid-template-columns: 1fr 300px; gap: 0.8rem; align-items: start; min-height: 0; }
  @media (max-width: 1000px) { .ce-body { grid-template-columns: 1fr; } }

  .ce-canvas-wrap { overflow: auto; background: var(--baby-blue-bg, #eef3fb); padding: 1.2rem; border-radius: var(--radius); display: flex; justify-content: center; max-height: 72vh; }
  .paper { background: #fff; color: #000; box-shadow: 0 2px 10px rgba(0,0,0,0.18); box-sizing: content-box; }
  .content { position: relative; }
  .el { position: absolute; box-sizing: border-box; cursor: move; padding: 1px 2px; border: 1px dashed transparent; overflow: hidden; }
  .el:hover { border-color: #b9c6dd; }
  .el.sel { border: 1px solid var(--baby-blue, #4a86e8); box-shadow: 0 0 0 2px rgba(74,134,232,0.25); z-index: 2; }
  .el.dim { opacity: 0.45; }
  .el .pre { white-space: pre-line; }
  .el .pair { display: flex; justify-content: space-between; gap: 0.6em; }
  .el hr { border: none; border-top: 1px dashed #000; margin: 2px 0; }
  .el .ghost { color: #99a; font-style: italic; }
  .el table { width: 100%; border-collapse: collapse; font-size: 0.9em; }
  .el th, .el td { border: 1px solid #ccc; padding: 0 2px; }
  .el .rz { position: absolute; right: -4px; bottom: -4px; width: 10px; height: 10px; background: var(--baby-blue, #4a86e8); border: 1px solid #fff; border-radius: 2px; cursor: nwse-resize; z-index: 3; }

  .ce-props { max-height: 72vh; overflow: auto; }
  .col-title { font-weight: 700; margin-bottom: 0.5rem; }
  .row-between { display: flex; justify-content: space-between; align-items: center; }
  .prop { display: flex; flex-direction: column; gap: 0.15rem; margin-bottom: 0.45rem; }
  .prop > label { font-size: 0.72rem; color: var(--text-dim); font-weight: 600; }
  .grid3 { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 0.4rem; }
  .grid4 { display: grid; grid-template-columns: 1fr 1fr 1fr 1fr; gap: 0.4rem; }
  .row-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 0.4rem; }
  .chk { display: flex; align-items: center; gap: 0.4rem; font-size: 0.82rem; margin-bottom: 0.35rem; }
  .chk.sm { justify-content: center; margin: 0; }
  .tiny { font-size: 0.72rem; }
  .cols-title { font-weight: 600; font-size: 0.8rem; margin: 0.4rem 0; }
  .col-edit { display: grid; grid-template-columns: 1fr 1fr auto auto auto; gap: 0.25rem; align-items: center; margin-bottom: 0.25rem; }
  .col-edit input, .col-edit select { font-size: 0.75rem; padding: 0.15rem; }
  .icon-btn { background: none; border: none; cursor: pointer; font-size: 0.9rem; }
</style>
