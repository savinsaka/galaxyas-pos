<script lang="ts">
  // Daftar Barang (read-only di Phase 1) — bukti alur rpc end-to-end ke
  // Server Pusat: search + infinite scroll di atas list_products_page.
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { toastError } from "$lib/toast";
  import { formatIDR, formatQty } from "$lib/format";
  import { debounce } from "$lib/debounce";
  import type { ProductWithStock } from "$lib/types";

  const PAGE = 40;

  let items = $state<ProductWithStock[]>([]);
  let total = $state(0);
  let search = $state("");
  let loading = $state(false);
  let exhausted = $derived(items.length >= total);

  async function load(reset: boolean) {
    if (loading) return;
    loading = true;
    try {
      const page = await api.listProductsPage({
        search,
        limit: PAGE,
        offset: reset ? 0 : items.length,
      });
      items = reset ? page.items : [...items, ...page.items];
      total = page.total;
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }

  const searchDebounced = debounce(() => load(true), 300);

  function onScroll(e: Event) {
    const el = e.target as HTMLElement;
    if (el.scrollHeight - el.scrollTop - el.clientHeight < 300 && !exhausted) load(false);
  }

  onMount(() => load(true));
</script>

<div class="produk">
  <div class="search-bar">
    <input
      placeholder="Cari nama / barcode…"
      bind:value={search}
      oninput={() => searchDebounced()}
    />
  </div>

  <div class="list" onscroll={onScroll}>
    {#each items as p (p.id)}
      <div class="card item">
        <div class="item-main">
          <div class="item-name">{p.name}</div>
          <div class="item-meta text-dim">
            {#if p.brand}<span>{p.brand}</span> · {/if}
            {#if p.barcode}<span class="mono">{p.barcode}</span>{:else}<span>tanpa barcode</span>{/if}
          </div>
        </div>
        <div class="item-side">
          <div class="item-price mono">{formatIDR(p.sell_price)}</div>
          <div class="item-stock text-dim">stok {formatQty(p.stock_qty)}</div>
        </div>
      </div>
    {/each}

    {#if loading}
      <p class="text-dim center">Memuat…</p>
    {:else if items.length === 0}
      <p class="text-dim center">Tidak ada barang{search ? ` untuk "${search}"` : ""}.</p>
    {:else if exhausted}
      <p class="text-dim center">{total} barang.</p>
    {/if}
  </div>
</div>

<style>
  .produk {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .search-bar {
    flex-shrink: 0;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding-bottom: 0.5rem;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.7rem 0.85rem;
  }
  .item-main {
    flex: 1;
    min-width: 0;
  }
  .item-name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .item-meta {
    font-size: 0.76rem;
    margin-top: 0.15rem;
  }
  .item-side {
    text-align: right;
    flex-shrink: 0;
  }
  .item-price {
    font-weight: 700;
    color: var(--primary-dark);
  }
  .item-stock {
    font-size: 0.74rem;
  }
  .center {
    text-align: center;
    padding: 0.6rem 0;
  }
</style>
