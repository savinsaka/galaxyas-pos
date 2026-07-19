<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR, formatQty } from "$lib/format";
  import { toastError } from "$lib/toast";
  import { debounce } from "$lib/debounce";
  import type { ProductWithStock } from "$lib/types";

  let {
    initialQuery = "",
    onClose,
    onPick,
  }: {
    initialQuery?: string;
    onClose: () => void;
    onPick: (p: ProductWithStock) => void;
  } = $props();

  let query = $state(initialQuery);
  let results = $state<ProductWithStock[]>([]);
  let loading = $state(false);
  let highlightIndex = $state(0);

  async function run(term: string) {
    if (!term.trim()) {
      results = [];
      return;
    }
    loading = true;
    try {
      results = await api.listProducts(term, false, 30);
      highlightIndex = 0;
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }
  const debouncedRun = debounce((t: string) => run(t), 300);
  function onInput() {
    debouncedRun(query);
  }
  onMount(() => {
    if (initialQuery.trim()) run(initialQuery);
  });

  function scrollHighlightIntoView() {
    document.querySelector(`[data-sr-index="${highlightIndex}"]`)?.scrollIntoView({ block: "nearest" });
  }
  function onQueryKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (results.length) highlightIndex = Math.min(highlightIndex + 1, results.length - 1);
      scrollHighlightIntoView();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (results.length) highlightIndex = Math.max(highlightIndex - 1, 0);
      scrollHighlightIntoView();
    } else if (e.key === "Enter") {
      e.preventDefault();
      const p = results[highlightIndex];
      if (p) onPick(p);
    }
  }
</script>

<div class="modal-backdrop" onclick={onClose} role="presentation">
  <div class="modal popup-search" onclick={(e) => e.stopPropagation()} role="presentation">
    <h2>🔍 Cari Barang</h2>
    <input class="mono" placeholder="Ketik nama atau barcode…" bind:value={query} oninput={onInput} onkeydown={onQueryKey} autofocus />
    <div class="popup-results">
      {#if loading}
        <div class="sr-empty text-dim">Mencari…</div>
      {:else}
        {#each results as p, i (p.id)}
          <button class="sr-row" class:active={i === highlightIndex} data-sr-index={i} onclick={() => onPick(p)}>
            <span class="sr-name">{p.name}</span>
            <span class="sr-meta text-dim">{p.barcode ?? ""}</span>
            <span class="sr-price mono">{formatIDR(p.sell_price)}</span>
            <span class="sr-stock text-dim">stok {formatQty(p.stock_qty)}</span>
          </button>
        {:else}
          <div class="sr-empty text-dim">{query.trim() ? "Tidak ditemukan." : "Ketik untuk mencari…"}</div>
        {/each}
      {/if}
    </div>
    <div class="row" style="justify-content:flex-end; margin-top:0.8rem;">
      <button class="btn-ghost" onclick={onClose}>Tutup</button>
    </div>
  </div>
</div>

<style>
  .popup-search { max-width: 480px; }
  .popup-results {
    margin-top: 0.6rem;
    background: var(--white);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    max-height: 320px;
    overflow-y: auto;
  }
  .sr-row {
    display: grid;
    grid-template-columns: 1fr auto auto auto;
    gap: 0.6rem;
    align-items: center;
    width: 100%;
    text-align: left;
    border: none;
    border-radius: 0;
    border-bottom: 1px solid var(--border);
    padding: 0.45rem 0.8rem;
    font-size: 0.85rem;
  }
  .sr-row:last-child { border-bottom: none; }
  .sr-row.active { background: var(--baby-blue-soft); }
  .sr-name { font-weight: 600; }
  .sr-meta, .sr-stock { font-size: 0.78rem; }
  .sr-empty { padding: 0.7rem 0.8rem; font-size: 0.85rem; }
</style>
