<script lang="ts">
  // Pratinjau lampiran chat tanpa membuka aplikasi lain: foto, PDF (viewer
  // bawaan WebView2), Excel (SheetJS → tabel), Word docx (docx-preview).
  // Isi dibaca dari `bytes` (file yang belum/sedang dikirim) atau dari `path`
  // (file di cache chat PC ini). Mode `inline` dipakai panel lampiran sebelum
  // kirim; tanpa itu tampil sebagai modal layar penuh.
  import { onDestroy, untrack } from "svelte";
  import * as XLSX from "xlsx";
  import { api } from "$lib/api";
  import { canPreview, fileKind, fmtSize, isImagePreview, kindLabel, mimeOf, extOf } from "$lib/chatFiles";

  let {
    name,
    size = null,
    path = null,
    bytes = null,
    inline = false,
    onclose,
    ondownload = null,
  }: {
    name: string;
    size?: number | null;
    path?: string | null;
    bytes?: Uint8Array | null;
    inline?: boolean;
    onclose?: () => void;
    ondownload?: (() => void) | null;
  } = $props();

  const MAX_ROWS = 1000;

  let loading = $state(true);
  let error = $state("");
  let url = $state("");
  let sheets = $state<{ name: string; html: string; cut: boolean }[]>([]);
  let sheetIdx = $state(0);
  let docxHost = $state<HTMLDivElement | null>(null);
  let docxBytes = $state<Uint8Array | null>(null);
  let pdfHost = $state<HTMLDivElement | null>(null);
  let pdfBytes = $state<Uint8Array | null>(null);
  let pdfNote = $state("");

  const kind = $derived(fileKind(name));
  const previewable = $derived(canPreview(name));

  function clearUrl() {
    if (url) URL.revokeObjectURL(url);
    url = "";
  }

  async function readBytes(): Promise<Uint8Array> {
    if (bytes) return bytes;
    if (path) return new Uint8Array(await api.chatReadFile(path));
    throw new Error("File tidak tersedia di PC ini.");
  }

  // Muat ulang setiap kali file yang ditampilkan berganti.
  $effect(() => {
    const key = `${name}|${path ?? ""}|${bytes?.length ?? 0}`;
    void key;
    let cancelled = false;
    loading = true;
    error = "";
    untrack(clearUrl); // jangan jadikan `url` dependensi effect ini
    sheets = [];
    sheetIdx = 0;
    docxBytes = null;
    pdfBytes = null;
    pdfNote = "";
    if (!previewable) {
      loading = false;
      return;
    }
    readBytes()
      .then((data) => {
        if (cancelled) return;
        if (isImagePreview(name)) {
          url = URL.createObjectURL(new Blob([data as BlobPart], { type: mimeOf(name) }));
        } else if (extOf(name) === "pdf") {
          pdfBytes = data;
        } else if (kind === "excel") {
          const wb = XLSX.read(data, { type: "array", sheetRows: MAX_ROWS + 1 });
          sheets = wb.SheetNames.map((n) => {
            const ws = wb.Sheets[n];
            const range = ws["!ref"] ? XLSX.utils.decode_range(ws["!ref"]) : null;
            const cut = !!range && range.e.r >= MAX_ROWS;
            return { name: n, html: XLSX.utils.sheet_to_html(ws, { header: "", footer: "" }), cut };
          });
        } else if (extOf(name) === "docx") {
          docxBytes = data;
        }
        loading = false;
      })
      .catch((e) => {
        if (cancelled) return;
        error = e instanceof Error ? e.message : String(e);
        loading = false;
      });
    return () => {
      cancelled = true;
    };
  });

  // Word dirender setelah wadahnya ada di DOM.
  $effect(() => {
    const host = docxHost;
    const data = docxBytes;
    if (!host || !data) return;
    host.innerHTML = "";
    import("docx-preview")
      .then(({ renderAsync }) =>
        renderAsync(new Blob([data as BlobPart], { type: mimeOf(name) }), host, undefined, {
          inWrapper: true,
          breakPages: true,
          ignoreLastRenderedPageBreak: true,
          renderHeaders: true,
          renderFooters: true,
          experimental: false,
        }),
      )
      .catch((e) => (error = "Dokumen Word tidak bisa dibaca: " + (e instanceof Error ? e.message : String(e))));
  });

  // PDF digambar pdf.js ke canvas (tidak bergantung viewer PDF bawaan webview).
  const MAX_PAGES = 50;
  $effect(() => {
    const host = pdfHost;
    const data = pdfBytes;
    if (!host || !data) return;
    let stop = false;
    let task: { destroy: () => Promise<void> } | null = null;
    host.innerHTML = "";
    (async () => {
      const pdfjs = await import("pdfjs-dist");
      const worker = await import("pdfjs-dist/build/pdf.worker.min.mjs?url");
      pdfjs.GlobalWorkerOptions.workerSrc = worker.default;
      // pdf.js memindahkan (detach) buffer yang diberikan — pakai salinan
      // supaya bytes lampiran yang belum dikirim tetap utuh.
      const loading = pdfjs.getDocument({ data: data.slice() });
      task = loading;
      const pdf = await loading.promise;
      const total = Math.min(pdf.numPages, MAX_PAGES);
      if (pdf.numPages > MAX_PAGES) pdfNote = `Hanya ${MAX_PAGES} dari ${pdf.numPages} halaman yang ditampilkan — Download untuk melihat semuanya.`;
      const width = Math.max(320, Math.min(host.clientWidth - 32, 1000));
      for (let i = 1; i <= total && !stop; i++) {
        const page = await pdf.getPage(i);
        const base = page.getViewport({ scale: 1 });
        const scale = width / base.width;
        const ratio = window.devicePixelRatio || 1;
        const vp = page.getViewport({ scale: scale * ratio });
        const canvas = document.createElement("canvas");
        canvas.width = vp.width;
        canvas.height = vp.height;
        canvas.style.width = `${vp.width / ratio}px`;
        canvas.style.height = `${vp.height / ratio}px`;
        canvas.className = "pdf-page";
        host.appendChild(canvas);
        await page.render({ canvas, viewport: vp }).promise;
      }
    })().catch((e) => {
      if (!stop) error = "PDF tidak bisa dibaca: " + (e instanceof Error ? e.message : String(e));
    });
    return () => {
      stop = true;
      task?.destroy();
    };
  });

  onDestroy(clearUrl);

  function onKey(e: KeyboardEvent) {
    if (!inline && e.key === "Escape") {
      e.stopPropagation();
      onclose?.();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#snippet body()}
  {#if loading}
    <div class="center dim">Memuat pratinjau…</div>
  {:else if error}
    <div class="center">
      <div class="big-icon k-{kind}">{kindLabel(name)}</div>
      <div class="dim">{error}</div>
    </div>
  {:else if !previewable}
    <div class="center">
      <div class="big-icon k-{kind}">{kindLabel(name)}</div>
      <div class="fname">{name}</div>
      <div class="dim">Pratinjau tidak tersedia untuk file .{extOf(name)}{ondownload ? " — gunakan Download untuk membukanya." : "."}</div>
      {#if ondownload}<button class="btn-primary" onclick={ondownload}>⬇ Download</button>{/if}
    </div>
  {:else if url && kind === "image"}
    <div class="img-wrap"><img src={url} alt={name} /></div>
  {:else if pdfBytes}
    <div class="pdf">
      <div class="pdf-pages" bind:this={pdfHost}></div>
      {#if pdfNote}<div class="xl-cut dim">{pdfNote}</div>{/if}
    </div>
  {:else if kind === "excel"}
    <div class="xl">
      {#if sheets.length > 1}
        <div class="xl-tabs">
          {#each sheets as s, i}
            <button class:on={i === sheetIdx} onclick={() => (sheetIdx = i)}>{s.name}</button>
          {/each}
        </div>
      {/if}
      {#if sheets[sheetIdx]}
        <div class="xl-scroll">{@html sheets[sheetIdx].html}</div>
        {#if sheets[sheetIdx].cut}
          <div class="xl-cut dim">Hanya {MAX_ROWS.toLocaleString("id-ID")} baris pertama yang ditampilkan — Download untuk melihat semuanya.</div>
        {/if}
      {:else}
        <div class="center dim">Lembar kosong.</div>
      {/if}
    </div>
  {:else if docxBytes}
    <div class="docx" bind:this={docxHost}></div>
  {/if}
{/snippet}

{#if inline}
  <div class="pv inline">{@render body()}</div>
{:else}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={(e) => e.target === e.currentTarget && onclose?.()}>
    <div class="pv modal" role="dialog" aria-label="Pratinjau {name}">
      <header>
        <span class="mini-icon k-{kind}">{kindLabel(name)}</span>
        <div class="title">
          <div class="fname">{name}</div>
          {#if size}<div class="dim small">{fmtSize(size)}</div>{/if}
        </div>
        {#if ondownload}
          <button class="hbtn" onclick={ondownload} title="Simpan ke folder Downloads">⬇ Download</button>
        {/if}
        <button class="hbtn close" onclick={() => onclose?.()} title="Tutup (Esc)">✕</button>
      </header>
      <div class="content">{@render body()}</div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 900;
    background: color-mix(in srgb, #000 62%, transparent);
    display: grid;
    place-items: center;
    padding: 2.5vh 2.5vw;
  }
  .pv {
    color: var(--text);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .pv.modal {
    width: 100%;
    height: 100%;
    max-width: 1200px;
    background: var(--panel);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: 0 10px 40px color-mix(in srgb, #000 40%, transparent);
  }
  .pv.inline {
    flex: 1;
    height: 100%;
  }
  header {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.55rem 0.8rem;
    border-bottom: 1px solid var(--border);
    background: var(--baby-blue-bg);
  }
  .title {
    flex: 1;
    min-width: 0;
  }
  .fname {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .small {
    font-size: 0.75rem;
  }
  .dim {
    color: var(--text-dim);
  }
  .hbtn {
    font-size: 0.85rem;
    padding: 0.3rem 0.75rem;
  }
  .hbtn.close {
    font-size: 1rem;
    padding: 0.3rem 0.6rem;
  }
  .content {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: color-mix(in srgb, var(--text) 6%, var(--bg));
  }
  .center {
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
    text-align: center;
    padding: 2rem;
    max-width: 420px;
  }
  .img-wrap {
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
    padding: 0.8rem;
    overflow: auto;
  }
  .img-wrap img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 4px;
    box-shadow: 0 1px 6px color-mix(in srgb, #000 25%, transparent);
  }
  .pdf {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .pdf-pages {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.8rem;
    padding: 1rem;
  }
  .pdf-pages :global(.pdf-page) {
    background: #fff;
    box-shadow: 0 1px 6px color-mix(in srgb, #000 25%, transparent);
  }
  .xl {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .xl-tabs {
    display: flex;
    gap: 0.25rem;
    padding: 0.4rem 0.6rem 0;
    flex-wrap: wrap;
  }
  .xl-tabs button {
    font-size: 0.78rem;
    padding: 0.2rem 0.7rem;
    border-radius: 6px 6px 0 0;
  }
  .xl-tabs button.on {
    background: var(--primary);
    color: var(--white);
  }
  .xl-scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: var(--panel);
  }
  .xl-scroll :global(table) {
    border-collapse: collapse;
    font-size: 0.8rem;
    width: auto;
  }
  .xl-scroll :global(td) {
    border: 1px solid var(--border);
    padding: 0.2rem 0.45rem;
    white-space: nowrap;
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .xl-scroll :global(tr:first-child td) {
    background: var(--baby-blue-bg);
    font-weight: 600;
    position: sticky;
    top: 0;
  }
  .xl-cut {
    font-size: 0.75rem;
    padding: 0.3rem 0.6rem;
  }
  .docx {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .docx :global(.docx-wrapper) {
    background: transparent;
    padding: 1rem;
  }
  .big-icon,
  .mini-icon {
    display: grid;
    place-items: center;
    font-weight: 800;
    color: #fff;
    border-radius: 8px;
    background: var(--k, #64748b);
  }
  .big-icon {
    width: 72px;
    height: 88px;
    font-size: 1.1rem;
  }
  .mini-icon {
    width: 34px;
    height: 40px;
    font-size: 0.65rem;
    flex: none;
  }
  .k-pdf {
    --k: #d93025;
  }
  .k-word {
    --k: #2b579a;
  }
  .k-excel {
    --k: #217346;
  }
  .k-image {
    --k: #8e44ad;
  }
</style>
