<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR, formatQty } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { openTab } from "$lib/stores/tabs";
  import type { ProductInput, ProductWithStock } from "$lib/types";
  import ProductForm from "$lib/components/ProductForm.svelte";

  let products = $state<ProductWithStock[]>([]);
  let search = $state("");
  let includeInactive = $state(true);
  let selectedId = $state<string | null>(null);
  let modal = $state<{ mode: "edit" | "duplicate"; data: ProductInput } | null>(null);

  const selected = $derived(products.find((p) => p.id === selectedId) ?? null);

  async function load() {
    try {
      products = await api.listProducts(search, includeInactive);
    } catch (e) {
      toastError(e);
    }
  }
  onMount(load);

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
  <div class="row">
    <input placeholder="Cari nama / barcode…" bind:value={search} oninput={load} style="width:260px;" />
    <label class="row" style="margin:0; gap:0.4rem; font-weight:400; white-space:nowrap;">
      <input type="checkbox" bind:checked={includeInactive} onchange={load} style="width:auto;" /> non-aktif
    </label>
  </div>
</div>

<div class="card" style="padding:0; overflow:hidden;">
  <div style="max-height:calc(100vh - 320px); overflow:auto;">
    <table>
      <thead>
        <tr>
          <th>Nama</th><th>Barcode</th><th>Kategori</th><th>Merek</th>
          <th class="text-right">H. Pokok</th><th class="text-right">H. Jual</th>
          <th class="text-right">Stok</th><th>Status</th>
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
          <tr><td colspan="8" class="text-dim">Belum ada barang.</td></tr>
        {/each}
      </tbody>
    </table>
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
