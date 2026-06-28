<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatQty, formatDateTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import type { ProductWithStock, StockKind, StockMovement } from "$lib/types";

  let { kind, title }: { kind: StockKind; title: string } = $props();

  let rows = $state<StockMovement[]>([]);
  let products = $state<ProductWithStock[]>([]);
  let selectedId = $state<number | null>(null);
  let modal = $state<{ editId: number | null; product_id: string; qty: number; note: string } | null>(null);
  let busy = $state(false);

  const selected = $derived(rows.find((r) => r.id === selectedId) ?? null);
  const verb = kind === "in" ? "Masuk" : "Keluar";

  async function load() {
    try {
      [rows, products] = await Promise.all([
        api.listStockMovements(kind, null, null, 500),
        api.listProducts("", true),
      ]);
    } catch (e) {
      toastError(e);
    }
  }
  onMount(load);

  function openAdd() {
    modal = { editId: null, product_id: products[0]?.id ?? "", qty: 0, note: "" };
  }
  function openEdit() {
    if (!selected) return showToast("Pilih baris dulu.", "info");
    modal = { editId: selected.id, product_id: selected.product_id, qty: selected.qty, note: selected.note ?? "" };
  }
  async function save() {
    if (!modal) return;
    if (!modal.product_id) return showToast("Pilih barang.", "error");
    if (modal.qty <= 0) return showToast("Jumlah harus > 0.", "error");
    busy = true;
    try {
      if (modal.editId !== null) await api.deleteStockMovement(modal.editId);
      await api.createStockMovement({
        product_id: modal.product_id,
        kind,
        qty: modal.qty,
        note: modal.note || null,
        user_id: $currentUser?.username ?? null,
      });
      showToast(`Item ${verb.toLowerCase()} tersimpan.`, "success");
      modal = null;
      selectedId = null;
      await load();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
  async function hapus() {
    if (!selected) return showToast("Pilih baris dulu.", "info");
    if (!confirm("Hapus pergerakan ini? Stok akan dikoreksi.")) return;
    try {
      await api.deleteStockMovement(selected.id);
      showToast("Dihapus, stok dikoreksi.", "success");
      selectedId = null;
      await load();
    } catch (e) {
      toastError(e);
    }
  }
</script>

<div class="page-head"><h1>{title}</h1></div>

<div class="card" style="padding:0; overflow:hidden;">
  <div style="max-height:calc(100vh - 320px); overflow:auto;">
    <table>
      <thead><tr><th>Waktu</th><th>Barang</th><th class="text-right">Qty</th><th class="text-right">Stok Akhir</th><th>Catatan</th><th>Oleh</th></tr></thead>
      <tbody>
        {#each rows as r (r.id)}
          <tr onclick={() => (selectedId = r.id)} style={selectedId === r.id ? "background:var(--baby-blue-soft);" : ""}>
            <td>{formatDateTime(r.created_at)}</td>
            <td>{r.product_name}</td>
            <td class="text-right mono">{formatQty(r.qty)}</td>
            <td class="text-right mono">{formatQty(r.stock_after)}</td>
            <td>{r.note ?? "-"}</td>
            <td>{r.user_id ?? "-"}</td>
          </tr>
        {:else}
          <tr><td colspan="6" class="text-dim">Belum ada data.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<div class="bottom-bar">
  <button class="btn-primary" onclick={openAdd}>➕ Tambah Item {verb}</button>
  <button onclick={openEdit} disabled={!selected}>✏️ Edit</button>
  <button class="btn-danger" onclick={hapus} disabled={!selected}>🗑️ Hapus</button>
</div>

{#if modal}
  <div class="modal-backdrop" onclick={() => (modal = null)} role="presentation">
    <div class="modal" onclick={(e) => e.stopPropagation()} role="presentation">
      <h2>{modal.editId !== null ? "Edit" : "Tambah"} Item {verb}</h2>
      <label>Barang</label>
      <select bind:value={modal.product_id}>
        {#each products as p}<option value={p.id}>{p.name} (stok {formatQty(p.stock_qty)})</option>{/each}
      </select>
      <label style="margin-top:0.6rem;">Jumlah {verb}</label>
      <input type="number" min="1" bind:value={modal.qty} />
      <label style="margin-top:0.6rem;">Catatan</label>
      <input bind:value={modal.note} placeholder="opsional" />
      <div class="row" style="justify-content:flex-end; margin-top:1rem;">
        <button onclick={() => (modal = null)}>Batal</button>
        <button class="btn-primary" disabled={busy} onclick={save}>Simpan</button>
      </div>
    </div>
  </div>
{/if}
