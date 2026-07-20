<script lang="ts">
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import { markStockBatchesDirty } from "$lib/stores/stockBatchSignal";
  import type { PullItem } from "$lib/types";

  let { onConfirmed }: { onConfirmed?: () => void } = $props();

  let pending = $state<PullItem[]>([]);
  let editQty = $state<Record<string, number>>({});
  let editBarcode = $state<Record<string, string>>({});
  let knownLocally = $state<Record<string, boolean>>({});
  let loading = $state(false);
  let checked = $state(false);
  let busy = $state(false);

  async function checkPending() {
    loading = true;
    try {
      pending = await api.bridgeListPending();
      editQty = Object.fromEntries(pending.map((it) => [it.id, it.qty_dikirim]));
      editBarcode = Object.fromEntries(pending.map((it) => [it.id, it.barcode]));
      knownLocally = Object.fromEntries(pending.map((it) => [it.id, it.known_locally]));
      checked = true;
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }

  /** Dipanggil pas kasir selesai edit barcode — cek ulang apa barcode baru ini match produk lokal. */
  async function checkBarcode(id: string) {
    const barcode = editBarcode[id]?.trim();
    if (!barcode) return;
    editBarcode[id] = barcode;
    try {
      knownLocally[id] = (await api.findByBarcode(barcode)) !== null;
    } catch (e) {
      toastError(e);
    }
  }

  async function confirmAll() {
    if (pending.length === 0) return;

    const items: PullItem[] = pending.map((it) => ({
      ...it,
      barcode: editBarcode[it.id] ?? it.barcode,
      qty_dikirim: editQty[it.id] ?? it.qty_dikirim,
    }));
    const unmatched = items.filter((it) => !knownLocally[it.id]);
    const matched = items.filter((it) => knownLocally[it.id]);

    if (unmatched.length > 0) {
      const list = unmatched.map((it) => `- ${it.barcode} (${it.name ?? "tanpa nama"})`).join("\n");
      const ok = confirm(
        `Barang berikut tidak ditemukan di database barang toko ini:\n${list}\n\n` +
          "Lanjutkan akan MENGHAPUS barang ini dari daftar Pull — tidak masuk stok, tidak masuk sistem. " +
          "Klik Batal dulu kalau mau perbaiki barcode-nya."
      );
      if (!ok) return;
    }

    busy = true;
    try {
      for (const it of unmatched) {
        await api.bridgeRejectPull(it.id);
      }
      if (matched.length > 0) {
        const detail = await api.bridgeConfirmPull(matched, $currentUser?.username ?? null);
        showToast(`${detail.no}: ${detail.items.length} item dari app mobile masuk ke stok.`, "success");
        markStockBatchesDirty();
      } else {
        showToast("Semua barang ditolak (barcode tidak ditemukan).", "info");
      }
      pending = [];
      editQty = {};
      editBarcode = {};
      knownLocally = {};
      onConfirmed?.();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  async function rejectRow(item: PullItem) {
    if (!confirm(`Tolak "${item.name ?? item.barcode}"? Tidak masuk ke stok.`)) return;
    busy = true;
    try {
      await api.bridgeRejectPull(item.id);
      showToast("Baris ditolak.", "info");
      pending = pending.filter((p) => p.id !== item.id);
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="card" style="margin-bottom:0.8rem;">
  <div class="row" style="align-items:center; gap:0.6rem; justify-content:space-between;">
    <div class="row" style="align-items:center; gap:0.6rem;">
      <span style="font-weight:650;">📥 Pull dari App Mobile</span>
      {#if checked && pending.length > 0}
        <span class="pending-badge">{pending.length} menunggu</span>
      {/if}
    </div>
    <button onclick={checkPending} disabled={loading}>
      {loading ? "Mengecek..." : "🔄 Cek Pengiriman Baru"}
    </button>
  </div>

  {#if checked && pending.length === 0}
    <p class="text-dim" style="font-size:0.82rem; margin:0.5rem 0 0;">Tidak ada baris yang siap di-pull.</p>
  {/if}

  {#if pending.length > 0}
    <p class="text-dim" style="font-size:0.82rem; margin:0.6rem 0 0;">
      Cek qty fisik yang diterima. Koreksi kalau ternyata lebih/kurang dari yang dicatat Pengirim.
    </p>
    <table class="batch-table" style="margin-top:0.5rem;">
      <thead>
        <tr>
          <th>Barang</th>
          <th style="width:150px;">Barcode</th>
          <th style="width:70px;">Klaim</th>
          <th style="width:110px;">Qty Diterima</th>
          <th style="width:2rem;"></th>
        </tr>
      </thead>
      <tbody>
        {#each pending as item (item.id)}
          <tr>
            <td>
              {#if !knownLocally[item.id]}
                <span
                  class="warn-icon"
                  title="Barcode ini belum ada di data barang toko — perbaiki barcode-nya, atau kalau tetap dibiarkan barang ini akan dihapus dari daftar Pull (gak masuk stok) saat konfirmasi."
                  >⚠️</span
                >
              {/if}
              {item.name ?? "(barang baru)"}
            </td>
            <td>
              <input
                class="cell-input mono"
                type="text"
                bind:value={editBarcode[item.id]}
                onblur={() => checkBarcode(item.id)}
              />
            </td>
            <td class="text-dim mono">{item.qty_dikirim}</td>
            <td>
              <input class="cell-input mono" type="number" min="0" step="0.01" bind:value={editQty[item.id]} />
            </td>
            <td>
              <button class="del-btn" title="Tolak" onclick={() => rejectRow(item)} disabled={busy}>✕</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    <div class="row" style="justify-content:flex-end; margin-top:0.6rem;">
      <button class="btn-primary" onclick={confirmAll} disabled={busy}>
        {busy ? "Menyimpan..." : `💾 Konfirmasi Semua & Masukkan ke Stok (${pending.length})`}
      </button>
    </div>
  {/if}
</div>

<style>
  .pending-badge {
    font-size: 0.72rem;
    font-weight: 700;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    background: var(--danger);
    color: #fff;
  }
  .del-btn {
    padding: 0.15rem 0.35rem;
    color: var(--danger);
    border-color: transparent;
    background: transparent;
  }
  .warn-icon {
    margin-right: 0.3rem;
    cursor: help;
  }
</style>
