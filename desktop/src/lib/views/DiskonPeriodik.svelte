<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import type { DiscountPeriod, DiscountPeriodInput, ProductWithStock } from "$lib/types";

  const DAYS = [
    { k: "sen", l: "Sen" }, { k: "sel", l: "Sel" }, { k: "rab", l: "Rab" },
    { k: "kam", l: "Kam" }, { k: "jum", l: "Jum" }, { k: "sab", l: "Sab" }, { k: "min", l: "Min" },
  ];

  let list = $state<DiscountPeriod[]>([]);
  let products = $state<ProductWithStock[]>([]);
  let everyday = $state(true);
  let dayset = $state<Record<string, boolean>>({});
  let form = $state<DiscountPeriodInput>(blankForm());

  function blankForm(): DiscountPeriodInput {
    return {
      id: null, scope: "item", target: "", target_label: "",
      discount_type: "amount", value: 0, days: "everyday", is_active: true,
    };
  }

  const brands = $derived([...new Set(products.map((p) => p.brand).filter(Boolean))] as string[]);

  async function load() {
    try {
      [list, products] = await Promise.all([api.listDiscounts(), api.listProducts("", false)]);
    } catch (e) {
      toastError(e);
    }
  }
  onMount(load);

  function resetForm() {
    form = blankForm();
    everyday = true;
    dayset = {};
  }

  async function save() {
    const days = everyday ? "everyday" : DAYS.filter((d) => dayset[d.k]).map((d) => d.k).join(",");
    if (!everyday && !days) return showToast("Pilih minimal satu hari.", "error");
    if (!form.target) return showToast("Pilih item / brand target.", "error");

    // Lengkapi label target.
    let label = form.target_label;
    if (form.scope === "item") {
      label = products.find((p) => p.id === form.target)?.name ?? form.target;
    } else {
      label = form.target;
    }
    try {
      await api.saveDiscount({ ...form, days, target_label: label });
      showToast("Diskon tersimpan.", "success");
      resetForm();
      await load();
    } catch (e) {
      toastError(e);
    }
  }

  function editItem(d: DiscountPeriod) {
    form = {
      id: d.id, scope: d.scope, target: d.target, target_label: d.target_label,
      discount_type: d.discount_type, value: d.value, days: d.days, is_active: d.is_active,
    };
    everyday = d.days === "everyday";
    dayset = {};
    if (!everyday) d.days.split(",").forEach((k) => (dayset[k] = true));
  }

  async function remove(id: string) {
    if (!confirm("Hapus diskon ini?")) return;
    try {
      await api.deleteDiscount(id);
      await load();
    } catch (e) {
      toastError(e);
    }
  }

  const daysLabel = (d: string) =>
    d === "everyday" ? "Setiap hari" : d.split(",").map((k) => DAYS.find((x) => x.k === k)?.l ?? k).join(", ");
</script>

<div class="page-head"><h1>Diskon Periodik</h1></div>

<div class="grid-2" style="align-items:start;">
  <div class="card">
    <h2>{form.id ? "Edit Diskon" : "Diskon Baru"}</h2>

    <label>Berlaku untuk</label>
    <div class="row" style="margin-bottom:0.7rem;">
      <label class="row" style="margin:0; font-weight:400; gap:0.3rem;">
        <input type="radio" value="item" bind:group={form.scope} style="width:auto;" /> Per Item
      </label>
      <label class="row" style="margin:0; font-weight:400; gap:0.3rem;">
        <input type="radio" value="brand" bind:group={form.scope} style="width:auto;" /> Per Brand
      </label>
    </div>

    {#if form.scope === "item"}
      <label>Item</label>
      <select bind:value={form.target}>
        <option value="">— pilih item —</option>
        {#each products as p}<option value={p.id}>{p.name}</option>{/each}
      </select>
    {:else}
      <label>Brand</label>
      <select bind:value={form.target}>
        <option value="">— pilih brand —</option>
        {#each brands as b}<option value={b}>{b}</option>{/each}
      </select>
    {/if}

    <div class="grid-2" style="margin-top:0.7rem;">
      <div>
        <label>Tipe Diskon</label>
        <select bind:value={form.discount_type}>
          <option value="amount">Nominal (Rp)</option>
          <option value="percent">Persen (%)</option>
        </select>
      </div>
      <div>
        <label>Nilai</label>
        <input type="number" min="0" bind:value={form.value} />
      </div>
    </div>

    <label style="margin-top:0.7rem;">Hari Berlaku</label>
    <label class="row" style="margin:0 0 0.5rem; font-weight:400; gap:0.3rem;">
      <input type="checkbox" bind:checked={everyday} style="width:auto;" /> Setiap hari
    </label>
    {#if !everyday}
      <div class="row" style="flex-wrap:wrap;">
        {#each DAYS as d}
          <label class="row" style="margin:0; font-weight:400; gap:0.25rem;">
            <input type="checkbox" bind:checked={dayset[d.k]} style="width:auto;" /> {d.l}
          </label>
        {/each}
      </div>
    {/if}

    <label class="row" style="margin-top:0.7rem; font-weight:400; gap:0.3rem;">
      <input type="checkbox" bind:checked={form.is_active} style="width:auto;" /> Aktif
    </label>

    <div class="row" style="justify-content:flex-end; margin-top:1rem;">
      {#if form.id}<button onclick={resetForm}>Batal</button>{/if}
      <button class="btn-primary" onclick={save}>Simpan</button>
    </div>
  </div>

  <div class="card" style="padding:0; overflow:hidden;">
    <table>
      <thead><tr><th>Target</th><th>Diskon</th><th>Hari</th><th>Status</th><th></th></tr></thead>
      <tbody>
        {#each list as d (d.id)}
          <tr>
            <td>{d.target_label ?? d.target}<div class="text-dim">{d.scope}</div></td>
            <td class="mono">{d.discount_type === "percent" ? `${d.value}%` : formatIDR(d.value)}</td>
            <td>{daysLabel(d.days)}</td>
            <td><span class="badge {d.is_active ? 'on' : 'off'}">{d.is_active ? "Aktif" : "Off"}</span></td>
            <td>
              <div class="row">
                <button class="btn-ghost" onclick={() => editItem(d)}>Edit</button>
                <button class="btn-ghost" style="color:var(--danger);" onclick={() => remove(d.id)}>Hapus</button>
              </div>
            </td>
          </tr>
        {:else}
          <tr><td colspan="5" class="text-dim">Belum ada diskon.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
