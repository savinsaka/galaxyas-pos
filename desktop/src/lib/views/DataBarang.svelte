<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR, formatQty } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { openTab } from "$lib/stores/tabs";
  import { debounce } from "$lib/debounce";
  import type { Brand, ProductInput, ProductWithStock } from "$lib/types";
  import ProductForm from "$lib/components/ProductForm.svelte";

  const PAGE_SIZE = 50;
  const SORT_OPTIONS: { value: string; label: string }[] = [
    { value: "name", label: "Nama" },
    { value: "barcode", label: "Barcode" },
    { value: "sell_price", label: "Harga Jual" },
    { value: "cost_price", label: "Harga Pokok" },
    { value: "stock_qty", label: "Stok" },
    { value: "updated_at", label: "Terakhir Diubah" },
  ];

  let products = $state<ProductWithStock[]>([]);
  let total = $state(0);
  let page = $state(0);
  let search = $state("");
  let includeInactive = $state(true);
  let brands = $state<Brand[]>([]);
  let brandFilter = $state("");
  let sortBy = $state("name");
  let sortDir = $state<"asc" | "desc">("asc");
  let loading = $state(false);
  let selectedId = $state<string | null>(null);
  let modal = $state<{ mode: "edit" | "duplicate"; data: ProductInput } | null>(null);

  const selected = $derived(products.find((p) => p.id === selectedId) ?? null);
  const totalPages = $derived(Math.max(1, Math.ceil(total / PAGE_SIZE)));

  async function load() {
    loading = true;
    try {
      const res = await api.listProductsPage({
        search,
        includeInactive,
        brand: brandFilter || null,
        sortBy,
        sortDir,
        limit: PAGE_SIZE,
        offset: page * PAGE_SIZE,
      });
      products = res.items;
      total = res.total;
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }

  const debouncedSearch = debounce(() => {
    page = 0;
    load();
  }, 300);

  function onSearchInput() {
    debouncedSearch();
  }
  function onFilterChange() {
    page = 0;
    load();
  }
  function toggleSort(col: string) {
    if (sortBy === col) {
      sortDir = sortDir === "asc" ? "desc" : "asc";
    } else {
      sortBy = col;
      sortDir = "asc";
    }
    page = 0;
    load();
  }
  function goPage(delta: number) {
    const next = page + delta;
    if (next < 0 || next >= totalPages) return;
    page = next;
    load();
  }

  onMount(async () => {
    load();
    try {
      brands = await api.listBrands();
    } catch (e) {
      toastError(e);
    }
  });

  function toInput(p: ProductWithStock, keepId: boolean): ProductInput {
    return {
      id: keepId ? p.id : null,
      name: keepId ? p.name : `${p.name} (copy)`,
      barcode: keepId ? p.barcode : "",
      category: p.category,
      brand: p.brand,
      unit: p.unit,
      sell_price: p.sell_price,
      cost_price: p.cost_price,
      default_discount: p.default_discount,
      is_active: p.is_active,
    };
  }

  function edit() {
    if (!selected) return showToast("Pilih barang dulu.", "info");
    modal = { mode: "edit", data: toInput(selected, true) };
  }
  function duplicate() {
    if (!selected) return showToast("Pilih barang dulu.", "info");
    modal = { mode: "duplicate", data: toInput(selected, false) };
  }
  async function hapus() {
    if (!selected) return showToast("Pilih barang dulu.", "info");
    if (!confirm(`Hapus barang "${selected.name}"?`)) return;
    try {
      await api.deleteProduct(selected.id);
      showToast("Barang dihapus.", "success");
      selectedId = null;
      await load();
    } catch (e) {
      toastError(e);
    }
  }
</script>

<div class="page-head">
  <h1>Data Barang</h1>
  <div class="row" style="flex-wrap:wrap;">
    <input placeholder="Cari nama / barcode…" bind:value={search} oninput={onSearchInput} style="width:220px;" />
    <select bind:value={brandFilter} onchange={onFilterChange} style="width:150px;">
      <option value="">Semua Merek</option>
      {#each brands as b}<option value={b.name}>{b.name}</option>{/each}
    </select>
    <select bind:value={sortBy} onchange={onFilterChange} style="width:150px;">
      {#each SORT_OPTIONS as o}<option value={o.value}>Urutkan: {o.label}</option>{/each}
    </select>
    <button class="btn-ghost" title={sortDir === "asc" ? "Naik" : "Turun"} onclick={() => { sortDir = sortDir === "asc" ? "desc" : "asc"; onFilterChange(); }}>
      {sortDir === "asc" ? "↑" : "↓"}
    </button>
    <label class="row" style="margin:0; gap:0.4rem; font-weight:400; white-space:nowrap;">
      <input type="checkbox" bind:checked={includeInactive} onchange={onFilterChange} style="width:auto;" /> non-aktif
    </label>
    <button class="btn-ghost" title="Muat ulang" onclick={load}>↻ Refresh</button>
  </div>
</div>

<div class="card" style="padding:0; overflow:hidden;">
  <div style="max-height:calc(100vh - 360px); overflow:auto; position:relative;">
    {#if loading}<div class="loading-overlay">Memuat…</div>{/if}
    <table>
      <thead>
        <tr>
          <th class="sortable" onclick={() => toggleSort("name")}>Nama{sortBy === "name" ? (sortDir === "asc" ? " ↑" : " ↓") : ""}</th>
          <th class="sortable" onclick={() => toggleSort("barcode")}>Barcode{sortBy === "barcode" ? (sortDir === "asc" ? " ↑" : " ↓") : ""}</th>
          <th>Kategori</th><th>Merek</th>
          <th class="text-right sortable" onclick={() => toggleSort("cost_price")}>H. Pokok{sortBy === "cost_price" ? (sortDir === "asc" ? " ↑" : " ↓") : ""}</th>
          <th class="text-right sortable" onclick={() => toggleSort("sell_price")}>H. Jual{sortBy === "sell_price" ? (sortDir === "asc" ? " ↑" : " ↓") : ""}</th>
          <th class="text-right sortable" onclick={() => toggleSort("stock_qty")}>Stok{sortBy === "stock_qty" ? (sortDir === "asc" ? " ↑" : " ↓") : ""}</th>
          <th>Status</th>
        </tr>
      </thead>
      <tbody>
        {#each products as p (p.id)}
          <tr
            onclick={() => (selectedId = p.id)}
            style={selectedId === p.id ? "background:var(--baby-blue-soft);" : ""}
          >
            <td>{p.name}</td>
            <td class="mono">{p.barcode ?? "-"}</td>
            <td>{p.category ?? "-"}</td>
            <td>{p.brand ?? "-"}</td>
            <td class="text-right mono">{formatIDR(p.cost_price)}</td>
            <td class="text-right mono">{formatIDR(p.sell_price)}</td>
            <td class="text-right mono">{formatQty(p.stock_qty)} {p.unit ?? ""}</td>
            <td><span class="badge {p.is_active ? 'on' : 'off'}">{p.is_active ? "Aktif" : "Non-aktif"}</span></td>
          </tr>
        {:else}
          <tr><td colspan="8" class="text-dim">{loading ? "Memuat…" : "Belum ada barang."}</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
  <div class="pager">
    <span class="text-dim" style="font-size:0.82rem;">
      {total.toLocaleString("id-ID")} barang · Hal {page + 1} / {totalPages}
    </span>
    <div class="row" style="gap:0.3rem;">
      <button disabled={page === 0} onclick={() => goPage(-1)}>‹ Sebelumnya</button>
      <button disabled={page + 1 >= totalPages} onclick={() => goPage(1)}>Berikutnya ›</button>
    </div>
  </div>
</div>

<div class="bottom-bar">
  <button class="btn-primary" onclick={() => openTab({ viewKey: "tambah-barang", title: "Tambah Barang", icon: "➕", singleton: true })}>
    ➕ Tambah Barang
  </button>
  <button onclick={duplicate} disabled={!selected}>📑 Duplikasi</button>
  <button onclick={edit} disabled={!selected}>✏️ Edit Barang</button>
  <button class="btn-danger" onclick={hapus} disabled={!selected}>🗑️ Hapus Barang</button>
  <span class="text-dim" style="margin-left:auto; align-self:center;">
    {selected ? `Terpilih: ${selected.name}` : "Klik baris untuk memilih"}
  </span>
</div>

{#if modal}
  <div class="modal-backdrop" onclick={() => (modal = null)} role="presentation">
    <div class="modal" onclick={(e) => e.stopPropagation()} role="presentation">
      <h2>{modal.mode === "edit" ? "Edit Barang" : "Duplikasi Barang"}</h2>
      <ProductForm
        initial={modal.data}
        submitLabel={modal.mode === "edit" ? "Simpan Perubahan" : "Simpan Duplikat"}
        onSaved={async () => { modal = null; await load(); }}
      />
    </div>
  </div>
{/if}

<style>
  .sortable { cursor: pointer; user-select: none; }
  .sortable:hover { color: var(--primary); }
  .loading-overlay {
    position: sticky; top: 0; left: 0; right: 0; z-index: 2;
    text-align: center; padding: 0.3rem; font-size: 0.78rem;
    background: var(--baby-blue-bg); color: var(--text-dim);
  }
  .pager {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.5rem 0.9rem; border-top: 1px solid var(--border);
  }
</style>
