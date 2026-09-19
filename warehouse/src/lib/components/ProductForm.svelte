<script lang="ts">
  import { onDestroy } from "svelte";
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import { setTabDirty, clearTabDirty } from "$lib/stores/tabGuard";
  import { closeTab } from "$lib/stores/tabs";
  import { formatMoneyInput, onMoneyInput } from "$lib/moneyInput";
  import type { Product, ProductInput } from "$lib/types";
  import BrandPicker from "$lib/components/BrandPicker.svelte";

  let {
    initial = null,
    submitLabel = "Simpan",
    resetAfterSave = false,
    onSaved = () => {},
    tabId,
  }: {
    initial?: ProductInput | null;
    submitLabel?: string;
    resetAfterSave?: boolean;
    onSaved?: (p: Product) => void;
    tabId?: string;
  } = $props();

  const blank = (): ProductInput => ({
    id: null,
    name: "",
    barcode: "",
    category: "",
    brand: "",
    unit: "pcs",
    sell_price: 0,
    cost_price: 0,
    default_discount: 0,
    is_active: true,
  });

  let form = $state<ProductInput>(initial ? { ...initial } : blank());
  let busy = $state(false);
  /** Pilihan setelah simpan (hanya saat resetAfterSave — dipakai tab "Tambah
   * Barang" — bukan saat form ini dipakai di modal edit/duplikasi). */
  let savedInput = $state<ProductInput | null>(null);

  // Dirty hanya dipantau kalau dipakai sebagai tab mandiri (Tambah Barang),
  // bukan saat dipakai di modal edit/duplikasi (tabId tidak dikirim).
  $effect(() => {
    if (tabId) setTabDirty(tabId, JSON.stringify(form) !== JSON.stringify(blank()));
  });
  onDestroy(() => { if (tabId) clearTabDirty(tabId); });

  async function save(e: Event) {
    e.preventDefault();
    const bc = form.barcode?.trim() ?? "";
    if (!form.name.trim()) { showToast("Nama barang wajib diisi.", "error"); return; }
    if (!bc) { showToast("Barcode wajib diisi.", "error"); return; }
    if (!form.brand?.trim()) { showToast("Merek wajib diisi.", "error"); return; }
    if ((form.sell_price ?? 0) <= 0) { showToast("Harga jual harus lebih dari 0.", "error"); return; }
    if ((form.cost_price ?? 0) <= 0) { showToast("Harga pokok harus lebih dari 0.", "error"); return; }
    busy = true;
    try {
      // Cek duplikat barcode di produk lain
      const existing = await api.findByBarcode(bc);
      if (existing && existing.id !== (form.id ?? "")) {
        showToast(`Barcode "${bc}" sudah dipakai oleh "${existing.name}".`, "error");
        return;
      }
      const saved = await api.saveProduct({ ...form, barcode: bc });
      showToast("Barang tersimpan.", "success");
      onSaved(saved);
      if (resetAfterSave) savedInput = { ...form, barcode: bc };
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  function afterSaveTutup() {
    if (tabId) { clearTabDirty(tabId); closeTab(tabId); }
    savedInput = null;
  }
  function afterSaveLanjut() {
    form = blank();
    savedInput = null;
  }
  function afterSaveDuplikat() {
    if (savedInput) form = { ...savedInput, id: null, barcode: "" };
    savedInput = null;
  }
</script>

<form onsubmit={save}>
  <div class="grid-2">
    <div style="grid-column:1/3;">
      <label>Nama Barang *</label>
      <input bind:value={form.name} />
    </div>
    <div>
      <label>Barcode *</label>
      <input bind:value={form.barcode} placeholder="Wajib & unik" />
    </div>
    <div>
      <label>Satuan</label>
      <input bind:value={form.unit} />
    </div>
    <div>
      <label>Kategori</label>
      <input bind:value={form.category} />
    </div>
    <div>
      <label>Merek *</label>
      <BrandPicker bind:value={form.brand} />
    </div>
    <div>
      <label>Harga Pokok *</label>
      <input class="mono" inputmode="decimal" value={formatMoneyInput(form.cost_price)} oninput={(e) => onMoneyInput(e, (n) => (form.cost_price = n))} />
    </div>
    <div>
      <label>Harga Jual *</label>
      <input class="mono" inputmode="decimal" value={formatMoneyInput(form.sell_price)} oninput={(e) => onMoneyInput(e, (n) => (form.sell_price = n))} />
    </div>
    <div>
      <label>Default Diskon (Rp)</label>
      <input class="mono" inputmode="decimal" value={formatMoneyInput(form.default_discount)} oninput={(e) => onMoneyInput(e, (n) => (form.default_discount = n))} />
    </div>
    <div>
      <label>Status</label>
      <label class="row" style="margin:0; gap:0.4rem; font-weight:400;">
        <input type="checkbox" bind:checked={form.is_active} style="width:auto;" /> Aktif
      </label>
    </div>
  </div>
  <div class="row" style="justify-content:flex-end; margin-top:1rem;">
    <button class="btn-primary" disabled={busy} type="submit">{submitLabel}</button>
  </div>
</form>

{#if savedInput}
  <div class="modal-backdrop" onclick={afterSaveLanjut} role="presentation">
    <div class="modal" style="max-width:380px; text-align:center;" onclick={(e) => e.stopPropagation()} role="presentation">
      <div style="font-size:2.2rem; margin-bottom:0.3rem;">✅</div>
      <h2>"{savedInput.name}" Tersimpan</h2>
      <p class="text-dim" style="margin:0.3rem 0 1rem;">Mau lanjut ke mana?</p>
      <div style="display:flex; flex-direction:column; gap:0.5rem;">
        <button class="btn-primary" onclick={afterSaveLanjut}>➕ Simpan &amp; Lanjut (form baru kosong)</button>
        <button onclick={afterSaveDuplikat}>📑 Simpan &amp; Duplikat (isian tetap, barcode dikosongkan)</button>
        <button class="btn-ghost" onclick={afterSaveTutup}>✔️ Simpan &amp; Tutup</button>
      </div>
    </div>
  </div>
{/if}
