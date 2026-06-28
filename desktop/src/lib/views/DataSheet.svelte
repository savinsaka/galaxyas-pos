<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import type { ProductWithStock } from "$lib/types";

  let rows = $state<ProductWithStock[]>([]);
  let search = $state("");
  let savingId = $state<string | null>(null);

  async function load() {
    try {
      rows = await api.listProducts(search, true);
    } catch (e) {
      toastError(e);
    }
  }
  onMount(load);

  async function saveRow(r: ProductWithStock) {
    savingId = r.id;
    try {
      await api.saveProduct({
        id: r.id,
        name: r.name,
        barcode: r.barcode, // tidak diubah
        category: r.category,
        brand: r.brand,
        unit: r.unit,
        sell_price: r.sell_price,
        cost_price: r.cost_price,
        default_discount: r.default_discount,
        is_active: r.is_active,
      });
      showToast(`"${r.name}" tersimpan.`, "success");
    } catch (e) {
      toastError(e);
    } finally {
      savingId = null;
    }
  }
</script>

<div class="page-head">
  <h1>Data Sheet</h1>
  <input placeholder="Cari…" bind:value={search} oninput={load} style="width:260px;" />
</div>

<div class="card" style="padding:0; overflow:auto; max-height:calc(100vh - 240px);">
  <table>
    <thead>
      <tr>
        <th style="width:140px;">Barcode</th>
        <th>Nama</th><th>Kategori</th><th>Merek</th><th>Satuan</th>
        <th class="text-right">H.Pokok</th><th class="text-right">H.Jual</th>
        <th class="text-right">Diskon</th><th>Aktif</th><th></th>
      </tr>
    </thead>
    <tbody>
      {#each rows as r (r.id)}
        <tr>
          <td class="mono text-dim">{r.barcode ?? "-"}</td>
          <td><input bind:value={r.name} /></td>
          <td><input bind:value={r.category} /></td>
          <td><input bind:value={r.brand} /></td>
          <td><input bind:value={r.unit} style="width:70px;" /></td>
          <td><input type="number" bind:value={r.cost_price} style="width:100px; text-align:right;" /></td>
          <td><input type="number" bind:value={r.sell_price} style="width:100px; text-align:right;" /></td>
          <td><input type="number" bind:value={r.default_discount} style="width:90px; text-align:right;" /></td>
          <td><input type="checkbox" bind:checked={r.is_active} style="width:auto;" /></td>
          <td>
            <button class="btn-primary" disabled={savingId === r.id} onclick={() => saveRow(r)}>Simpan</button>
          </td>
        </tr>
      {:else}
        <tr><td colspan="10" class="text-dim">Belum ada barang.</td></tr>
      {/each}
    </tbody>
  </table>
</div>
<p class="text-dim" style="font-size:0.78rem;">Barcode tidak bisa diubah di sini. Semua kolom lain bisa diedit langsung lalu klik <b>Simpan</b> per baris.</p>
