<script lang="ts">
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import type { Product, ProductInput } from "$lib/types";
  import BrandPicker from "$lib/components/BrandPicker.svelte";

  let {
    initial = null,
    submitLabel = "Simpan",
    resetAfterSave = false,
    onSaved = () => {},
  }: {
    initial?: ProductInput | null;
    submitLabel?: string;
    resetAfterSave?: boolean;
    onSaved?: (p: Product) => void;
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

  async function save(e: Event) {
    e.preventDefault();
    if (!form.name.trim()) {
      showToast("Nama barang wajib diisi.", "error");
      return;
    }
    busy = true;
    try {
      const saved = await api.saveProduct(form);
      showToast("Barang tersimpan.", "success");
      onSaved(saved);
      if (resetAfterSave) form = blank();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<form onsubmit={save}>
  <div class="grid-2">
    <div style="grid-column:1/3;">
      <label>Nama Barang *</label>
      <input bind:value={form.name} />
    </div>
    <div>
      <label>Barcode</label>
      <input bind:value={form.barcode} />
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
      <label>Merek</label>
      <BrandPicker bind:value={form.brand} />
    </div>
    <div>
      <label>Harga Pokok</label>
      <input type="number" min="0" bind:value={form.cost_price} />
    </div>
    <div>
      <label>Harga Jual</label>
      <input type="number" min="0" bind:value={form.sell_price} />
    </div>
    <div>
      <label>Default Diskon (Rp)</label>
      <input type="number" min="0" bind:value={form.default_discount} />
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
