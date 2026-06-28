<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatQty } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { openTab } from "$lib/stores/tabs";
  import { currentUser } from "$lib/stores/auth";
  import type { ProductWithStock } from "$lib/types";

  let products = $state<ProductWithStock[]>([]);
  let search = $state("");
  let target = $state<ProductWithStock | null>(null);
  let qty = $state(0);
  let note = $state("");
  let busy = $state(false);

  async function load() {
    try {
      products = await api.listProducts(search, true);
    } catch (e) {
      toastError(e);
    }
  }
  onMount(load);

  function open(p: ProductWithStock) {
    target = p;
    qty = p.stock_qty;
    note = "";
  }

  async function apply() {
    if (!target) return;
    busy = true;
    try {
      await api.createStockMovement({
        product_id: target.id,
        kind: "opname",
        qty,
        note: note || "Stok opname",
        user_id: $currentUser?.username ?? null,
      });
      showToast("Opname tersimpan.", "success");
      target = null;
      await load();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="page-head">
  <h1>Stok Opname</h1>
  <input placeholder="Cari barang…" bind:value={search} oninput={load} style="width:260px;" />
</div>

<div class="card" style="padding:0; overflow:hidden;">
  <div style="max-height:calc(100vh - 320px); overflow:auto;">
    <table>
      <thead><tr><th>Nama</th><th>Kategori</th><th class="text-right">Stok Sistem</th><th></th></tr></thead>
      <tbody>
        {#each products as p (p.id)}
          <tr>
            <td>{p.name}</td>
            <td>{p.category ?? "-"}</td>
            <td class="text-right mono">{formatQty(p.stock_qty)} {p.unit ?? ""}</td>
            <td><button class="btn-ghost" onclick={() => open(p)}>Opname</button></td>
          </tr>
        {:else}
          <tr><td colspan="4" class="text-dim">Belum ada barang.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<div class="bottom-bar">
  <button class="btn-primary" onclick={() => openTab({ viewKey: "laporan-persediaan", title: "Laporan Persediaan", icon: "📈", singleton: true })}>📊 Laporan</button>
</div>

{#if target}
  <div class="modal-backdrop" onclick={() => (target = null)} role="presentation">
    <div class="modal" onclick={(e) => e.stopPropagation()} role="presentation">
      <h2>Opname: {target.name}</h2>
      <p class="text-dim">Stok sistem saat ini: {formatQty(target.stock_qty)}</p>
      <label>Stok Fisik (hasil hitung)</label>
      <input type="number" min="0" bind:value={qty} />
      <label style="margin-top:0.6rem;">Catatan</label>
      <input bind:value={note} placeholder="opsional" />
      <div class="row" style="justify-content:flex-end; margin-top:1rem;">
        <button onclick={() => (target = null)}>Batal</button>
        <button class="btn-primary" disabled={busy} onclick={apply}>Simpan Opname</button>
      </div>
    </div>
  </div>
{/if}
