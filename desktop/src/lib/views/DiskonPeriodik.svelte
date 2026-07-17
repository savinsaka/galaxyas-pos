<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import type { DiscountPeriod, DiscountPeriodInput, ProductWithStock } from "$lib/types";

  const DAYS = [
    { k: "sen", l: "Senin" }, { k: "sel", l: "Selasa" }, { k: "rab", l: "Rabu" },
    { k: "kam", l: "Kamis" }, { k: "jum", l: "Jumat" }, { k: "sab", l: "Sabtu" }, { k: "min", l: "Minggu" },
  ];

  // ── State utama ──────────────────────────────────────────────────────────────
  let list = $state<DiscountPeriod[]>([]);
  let products = $state<ProductWithStock[]>([]);

  // Form: header grup diskon
  interface DForm {
    id: string | null;
    code: string;
    discount_type: "amount" | "percent";
    value: number;
    days: string;     // "everyday" | csv
    is_active: boolean;
    priority: number;
  }
  // Baris item dalam grup (batch)
  interface DItem {
    uid: number;
    scope: "item" | "brand";
    target: string;
    target_label: string;
    search: string;
    dropOpen: boolean;
  }

  let form = $state<DForm>(blankForm());
  let everyday = $state(true);
  let dayset = $state<Record<string, boolean>>({});
  let itemUid = 1;
  let items = $state<DItem[]>([newItem()]);
  let busy = $state(false);

  // Kode-kode unik yang sudah ada (untuk memilih grup yang mau di-edit)
  const groups = $derived.by(() => {
    const seen = new Set<string>();
    const out: { code: string; count: number; active: boolean }[] = [];
    for (const d of list) {
      if (!seen.has(d.code)) {
        seen.add(d.code);
        const members = list.filter((x) => x.code === d.code);
        out.push({ code: d.code, count: members.length, active: members.some((x) => x.is_active) });
      }
    }
    return out;
  });

  const brands = $derived([...new Set(products.map((p) => p.brand).filter(Boolean))] as string[]);

  // Produk yang sudah di-scan (dropdown per baris)
  const filteredFor = (item: DItem) => {
    const q = item.search.trim().toLowerCase();
    if (!q || item.scope === "brand") return [];
    return products.filter(
      (p) => p.name.toLowerCase().includes(q) || (p.barcode ?? "").toLowerCase().includes(q),
    ).slice(0, 8);
  };

  function blankForm(): DForm {
    return { id: null, code: "", discount_type: "amount", value: 0, days: "everyday", is_active: true, priority: 1 };
  }
  function newItem(): DItem {
    return { uid: itemUid++, scope: "item", target: "", target_label: "", search: "", dropOpen: false };
  }

  async function load() {
    try {
      [list, products] = await Promise.all([api.listDiscounts(), api.listProducts("", false)]);
    } catch (e) { toastError(e); }
  }
  onMount(load);

  // ── Edit grup yang sudah ada ──────────────────────────────────────────────
  function editGroup(code: string) {
    const members = list.filter((d) => d.code === code);
    if (!members.length) return;
    const first = members[0];
    form = {
      id: null, // biarkan null — simpan akan upsert per-item
      code: first.code,
      discount_type: first.discount_type as "amount" | "percent",
      value: first.value,
      days: first.days,
      is_active: first.is_active,
      priority: first.priority,
    };
    everyday = first.days === "everyday";
    dayset = {};
    if (!everyday) first.days.split(",").forEach((k) => (dayset[k] = true));
    items = members.map((d) => ({
      uid: itemUid++,
      scope: d.scope as "item" | "brand",
      target: d.target,
      target_label: d.target_label ?? d.target,
      search: d.target_label ?? d.target,
      dropOpen: false,
    }));
  }

  function resetForm() {
    form = blankForm();
    everyday = true;
    dayset = {};
    items = [newItem()];
  }

  // ── Hapus seluruh grup ─────────────────────────────────────────────────────
  async function removeGroup(code: string) {
    if (!confirm(`Hapus semua diskon dengan kode "${code}"?`)) return;
    const members = list.filter((d) => d.code === code);
    for (const d of members) {
      try { await api.deleteDiscount(d.id); } catch (e) { toastError(e); }
    }
    showToast("Grup diskon dihapus.", "success");
    await load();
  }

  // ── Simpan ────────────────────────────────────────────────────────────────
  async function save() {
    if (!form.code.trim()) return showToast("Kode potongan wajib diisi.", "error");
    const days = everyday ? "everyday" : DAYS.filter((d) => dayset[d.k]).map((d) => d.k).join(",");
    if (!everyday && !days) return showToast("Pilih minimal satu hari.", "error");
    const validItems = items.filter((it) => it.target.trim());
    if (!validItems.length) return showToast("Tambahkan minimal satu item.", "error");

    busy = true;
    try {
      // Hapus anggota lama grup ini agar tidak ada duplikat saat edit
      const oldMembers = list.filter((d) => d.code === form.code);
      for (const d of oldMembers) await api.deleteDiscount(d.id);

      // Simpan ulang semua item baru
      for (const it of validItems) {
        let label = it.target_label || it.target;
        if (it.scope === "item") {
          label = products.find((p) => p.id === it.target)?.name ?? it.target;
        }
        const inp: DiscountPeriodInput = {
          id: null,
          code: form.code.trim(),
          scope: it.scope,
          target: it.target,
          target_label: label,
          discount_type: form.discount_type,
          value: form.value,
          days,
          is_active: form.is_active,
          priority: form.priority,
        };
        await api.saveDiscount(inp);
      }
      showToast("Diskon tersimpan.", "success");
      resetForm();
      await load();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  // ── Helpers ───────────────────────────────────────────────────────────────
  const daysLabel = (d: string) =>
    d === "everyday" ? "Setiap hari" : d.split(",").map((k) => DAYS.find((x) => x.k === k)?.l ?? k).join(", ");

  function selectItemProduct(item: DItem, p: ProductWithStock) {
    item.target = p.id;
    item.target_label = p.name;
    item.search = p.name;
    item.dropOpen = false;
    items = [...items];
  }
</script>

<div class="page-head"><h1>Diskon Periodik</h1></div>

<div class="dp-layout">
  <!-- ── Kiri: form ── -->
  <div class="card form-side">
    <div class="form-head">
      <h2>{form.code ? `Edit: ${form.code}` : "Tambah Diskon Baru"}</h2>
      {#if form.code}<button class="btn-ghost" onclick={resetForm}>Batal Edit</button>{/if}
    </div>

    <!-- Baris 1: Kode + Aktif + Prioritas -->
    <div class="hrow">
      <div style="flex:1;">
        <label>Kode Potongan *</label>
        <input bind:value={form.code} placeholder="Mis. PROMO-LEBARAN" />
      </div>
      <div>
        <label>Prioritas</label>
        <input type="number" min="1" max="99" bind:value={form.priority} style="width:70px;" />
      </div>
      <label class="row" style="align-self:flex-end; margin:0 0 0.3rem; gap:0.3rem; white-space:nowrap; font-weight:400;">
        <input type="checkbox" bind:checked={form.is_active} style="width:auto;" /> Aktif
      </label>
    </div>

    <!-- Baris 2: Tipe + Nilai -->
    <div class="hrow">
      <div>
        <label>Sistem Potongan</label>
        <select bind:value={form.discount_type}>
          <option value="amount">Nominal (Rp)</option>
          <option value="percent">Persen (%)</option>
        </select>
      </div>
      <div style="flex:1;">
        <label>Nilai {form.discount_type === "percent" ? "(%)" : "(Rp)"}</label>
        <input type="number" min="0" bind:value={form.value} />
      </div>
    </div>

    <!-- Pilihan Hari -->
    <div class="day-section">
      <label class="row" style="margin:0 0 0.4rem; gap:0.3rem; font-weight:400;">
        <input type="checkbox" bind:checked={everyday} style="width:auto;" /> Berlaku Setiap Hari
      </label>
      {#if !everyday}
        <div class="day-row">
          {#each DAYS as d}
            <label class="day-cb">
              <input type="checkbox" bind:checked={dayset[d.k]} style="width:auto;" />
              {d.l}
            </label>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Tabel batch item -->
    <div class="items-section">
      <div class="items-head">
        <span>Item / Brand Berlaku</span>
        <button onclick={() => { items = [...items, newItem()]; }}>+ Tambah Baris</button>
      </div>
      <div class="items-table-wrap">
        <table class="items-table">
          <thead>
            <tr>
              <th style="width:80px">Berlaku</th>
              <th>Kode/Nama Item atau Brand</th>
              <th style="width:2rem;"></th>
            </tr>
          </thead>
          <tbody>
            {#each items as item (item.uid)}
              <tr>
                <td>
                  <select class="cell-sel" bind:value={item.scope} onchange={() => { item.target = ""; item.target_label = ""; item.search = ""; items = [...items]; }}>
                    <option value="item">Per Item</option>
                    <option value="brand">Per Brand</option>
                  </select>
                </td>
                <td style="position:relative;">
                  {#if item.scope === "brand"}
                    <select class="cell-sel" bind:value={item.target} onchange={() => { item.target_label = item.target; items = [...items]; }}>
                      <option value="">— pilih brand —</option>
                      {#each brands as b}<option value={b}>{b}</option>{/each}
                    </select>
                  {:else}
                    <input
                      class="cell-input"
                      placeholder="Ketik nama atau scan barcode…"
                      bind:value={item.search}
                      oninput={() => { item.dropOpen = true; items = [...items]; }}
                      onfocus={() => { item.dropOpen = true; items = [...items]; }}
                      onblur={() => setTimeout(() => { item.dropOpen = false; items = [...items]; }, 200)}
                    />
                    {#if item.dropOpen && filteredFor(item).length > 0}
                      <div class="item-drop">
                        {#each filteredFor(item) as p (p.id)}
                          <button class="id-row" onmousedown={() => selectItemProduct(item, p)}>
                            <span>{p.name}</span>
                            <span class="text-dim" style="font-size:0.78rem;">{p.barcode ?? ""}</span>
                          </button>
                        {/each}
                      </div>
                    {/if}
                  {/if}
                </td>
                <td>
                  <button class="del-btn" onclick={() => { if (items.length > 1) items = items.filter((x) => x.uid !== item.uid); }}>✕</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>

    <div class="row" style="justify-content:flex-end; margin-top:0.8rem;">
      <button class="btn-primary" disabled={busy} onclick={save}>
        💾 Simpan Diskon
      </button>
    </div>
  </div>

  <!-- ── Kanan: daftar grup ── -->
  <div class="card" style="padding:0; overflow:hidden; align-self:start;">
    <div style="padding:0.7rem 0.9rem; font-weight:650; border-bottom:1px solid var(--border);">
      Daftar Kode Potongan
    </div>
    <table class="group-table">
      <thead>
        <tr>
          <th>Kode Potongan</th>
          <th>Tipe Periode</th>
          <th>Hari</th>
          <th style="text-align:center;">Item</th>
          <th style="text-align:center;">Aktif</th>
          <th style="text-align:center;">Prioritas</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each groups as g (g.code)}
          {@const members = list.filter((d) => d.code === g.code)}
          {@const first = members[0]}
          <tr class:selected-row={form.code === g.code}>
            <td class="mono fw-600">{g.code}</td>
            <td class="text-dim">
              {first.discount_type === "percent" ? `${first.value}%` : formatIDR(first.value)}
            </td>
            <td class="text-dim">{daysLabel(first.days)}</td>
            <td class="text-center">{members.length}</td>
            <td class="text-center">
              <span class="badge {g.active ? 'on' : 'off'}">{g.active ? "Aktif" : "Off"}</span>
            </td>
            <td class="text-center">{first.priority}</td>
            <td>
              <div class="row" style="gap:0.3rem;">
                <button class="btn-ghost" onclick={() => editGroup(g.code)}>Edit</button>
                <button class="btn-ghost" style="color:var(--danger);" onclick={() => removeGroup(g.code)}>Hapus</button>
              </div>
            </td>
          </tr>
        {:else}
          <tr><td colspan="7" class="text-dim">Belum ada diskon.</td></tr>
        {/each}
      </tbody>
    </table>

    <!-- Detail item-item dalam grup terpilih -->
    {#if form.code && list.filter((d) => d.code === form.code).length > 0}
      <div style="padding:0.5rem 0.9rem; border-top:2px solid var(--primary); background:var(--baby-blue-bg);">
        <div style="font-size:0.8rem; font-weight:650; margin-bottom:0.3rem; color:var(--primary);">
          Detail item dalam grup: {form.code}
        </div>
        {#each list.filter((d) => d.code === form.code) as d (d.id)}
          <div style="font-size:0.82rem; padding:0.15rem 0; display:flex; justify-content:space-between;">
            <span>{d.target_label ?? d.target}</span>
            <span class="text-dim">{d.scope}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .dp-layout { display: grid; grid-template-columns: 520px 1fr; gap: 1rem; align-items: start; }
  .form-side { display: flex; flex-direction: column; gap: 0.55rem; }
  .form-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.2rem; }
  .form-head h2 { margin: 0; }

  .hrow { display: flex; gap: 0.7rem; align-items: flex-end; }
  .hrow > div { display: flex; flex-direction: column; gap: 0.25rem; }

  .day-section { background: var(--baby-blue-bg); border-radius: var(--radius); padding: 0.6rem 0.75rem; }
  .day-row { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-top: 0.3rem; }
  .day-cb { display: flex; align-items: center; gap: 0.25rem; font-weight: 400; font-size: 0.85rem; white-space: nowrap; }

  .items-section { border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; }
  .items-head { display: flex; justify-content: space-between; align-items: center; padding: 0.45rem 0.7rem; background: var(--baby-blue-bg); border-bottom: 1px solid var(--border); font-size: 0.82rem; font-weight: 650; }
  .items-table-wrap { max-height: 260px; overflow-y: auto; }
  .items-table { width: 100%; border-collapse: collapse; }
  .items-table thead th { background: var(--baby-blue-bg); padding: 0.4rem 0.6rem; font-size: 0.78rem; color: var(--text-dim); font-weight: 650; text-transform: uppercase; border-bottom: 1px solid var(--border); position: sticky; top: 0; }
  .items-table tbody td { padding: 0.3rem 0.5rem; border-bottom: 1px solid var(--border); vertical-align: middle; }
  .items-table tbody tr:last-child td { border-bottom: none; }
  .cell-input, .cell-sel { width: 100%; border: 1px solid transparent; background: transparent; padding: 0.28rem 0.4rem; border-radius: 4px; font-size: 0.85rem; }
  .cell-input:focus, .cell-sel:focus { border-color: var(--primary); background: var(--white); outline: none; }

  .item-drop {
    position: absolute; left: 0; right: 0; top: 100%; z-index: 100;
    background: var(--white); border: 1px solid var(--border);
    border-radius: 0 0 var(--radius) var(--radius);
    box-shadow: var(--shadow); max-height: 200px; overflow-y: auto;
  }
  .id-row {
    display: flex; justify-content: space-between; width: 100%; text-align: left;
    border: none; border-radius: 0; border-bottom: 1px solid var(--border);
    padding: 0.4rem 0.7rem; font-size: 0.85rem; font-weight: 500;
  }
  .id-row:last-child { border-bottom: none; }
  .del-btn { padding: 0.15rem 0.35rem; color: var(--danger); border-color: transparent; background: transparent; }

  .group-table { width: 100%; border-collapse: collapse; }
  .group-table thead th { background: var(--baby-blue-bg); padding: 0.5rem 0.6rem; font-size: 0.78rem; color: var(--text-dim); font-weight: 650; text-transform: uppercase; border-bottom: 1px solid var(--border); }
  .group-table tbody td { padding: 0.45rem 0.6rem; border-bottom: 1px solid var(--border); }
  .group-table tbody tr:last-child td { border-bottom: none; }
  .group-table tbody tr:hover { background: var(--baby-blue-soft); }
  .selected-row { background: var(--baby-blue-soft) !important; }
  .fw-600 { font-weight: 600; }
  .text-center { text-align: center; }
</style>
