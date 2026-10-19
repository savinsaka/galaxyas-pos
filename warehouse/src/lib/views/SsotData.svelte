<script lang="ts">
  import { onMount, tick } from "svelte";
  import { api } from "$lib/api";
  import { formatDateTime, formatIDR } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { confirmDialog } from "$lib/dialog";
  import { debounce } from "$lib/debounce";
  import { formatMoneyInput, onMoneyInput } from "$lib/moneyInput";
  import type { Product, ProductInput } from "$lib/types";
  import BrandPicker from "$lib/components/BrandPicker.svelte";
  import { syncAccount, refreshSyncAccount, openSyncAkun } from "$lib/stores/syncAccount";

  // Isi SSoT (server jjapps.net) dimuat sekali lalu dicari di memori, jadi
  // scan barcode langsung ketemu tanpa bolak-balik ke server.
  const PAGE_SIZE = 50;
  const APPLY_LOCAL_KEY = "ssot_apply_local";

  let all = $state<Product[]>([]);
  let loading = $state(false);
  let loadError = $state<string | null>(null);
  let search = $state("");
  /** Kata kunci yang sudah di-debounce untuk memfilter tabel. */
  let term = $state("");
  let showInactive = $state(false);
  let page = $state(0);
  let highlight = $state(0);
  let searchEl = $state<HTMLInputElement | null>(null);

  let form = $state<ProductInput | null>(null);
  let formUpdatedAt = $state<string | null>(null);
  let saving = $state(false);
  let applyLocal = $state(readApplyLocal());

  function readApplyLocal(): boolean {
    try {
      return localStorage.getItem(APPLY_LOCAL_KEY) !== "0";
    } catch {
      return true;
    }
  }
  $effect(() => {
    const v = applyLocal ? "1" : "0";
    try {
      localStorage.setItem(APPLY_LOCAL_KEY, v);
    } catch {
      /* abaikan */
    }
  });

  function byName(a: Product, b: Product) {
    return a.name.localeCompare(b.name, "id", { sensitivity: "base" });
  }

  function matches(p: Product, t: string): boolean {
    if (!t) return true;
    return [p.name, p.barcode, p.brand, p.category].some((v) => v?.toLowerCase().includes(t));
  }

  function filterRows(t: string): Product[] {
    const key = t.trim().toLowerCase();
    return all.filter((p) => (showInactive || p.is_active) && matches(p, key));
  }

  const filtered = $derived(filterRows(term));
  const totalPages = $derived(Math.max(1, Math.ceil(filtered.length / PAGE_SIZE)));
  const pageRows = $derived(filtered.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE));
  const inactiveCount = $derived(all.filter((p) => !p.is_active).length);

  async function load() {
    loading = true;
    loadError = null;
    try {
      all = (await api.ssotListProducts()).sort(byName);
    } catch (e) {
      loadError = e instanceof Error ? e.message : String(e);
      toastError(e);
    } finally {
      loading = false;
    }
  }
  onMount(() => {
    refreshSyncAccount();
  });

  // Muat sekali begitu PC ini login (bisa juga terjadi saat tab ini sudah
  // terbuka lalu login dilakukan di tab Akun Server).
  let loadedOnce = false;
  $effect(() => {
    if ($syncAccount?.logged_in && !loadedOnce) {
      loadedOnce = true;
      load().then(async () => {
        // Input masih disabled sampai render berikutnya — focus() ke input disabled diabaikan.
        await tick();
        searchEl?.focus();
      });
    }
  });

  const applyTerm = debounce(() => {
    term = search;
    page = 0;
    highlight = 0;
  }, 150);

  function setHighlight(i: number) {
    if (!filtered.length) return;
    highlight = Math.max(0, Math.min(i, filtered.length - 1));
    page = Math.floor(highlight / PAGE_SIZE);
    tick().then(() =>
      document.querySelector(`[data-ssot-index="${highlight}"]`)?.scrollIntoView({ block: "nearest" }),
    );
  }

  // Mekanisme cari ala kasir: Enter = barcode persis → langsung buka; kalau
  // tidak ada, buka baris yang di-highlight (↑/↓ memindah highlight).
  async function onSearchKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlight(highlight + 1);
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlight(highlight - 1);
      return;
    }
    if (e.key === "Escape") {
      search = "";
      term = "";
      page = 0;
      highlight = 0;
      return;
    }
    if (e.key !== "Enter") return;
    e.preventDefault();
    const t = search.trim();
    if (!t) return;

    const exact = all.find((p) => (p.barcode ?? "").trim() === t);
    if (exact) return openEdit(exact);

    if (term !== search) {
      // Scanner mengetik lebih cepat dari debounce — samakan dulu.
      term = search;
      page = 0;
      highlight = 0;
    }
    const rows = filterRows(t);
    if (rows.length) return openEdit(rows[highlight] ?? rows[0]);

    const looksLikeBarcode = /^\S+$/.test(t) && /\d/.test(t);
    const ok = await confirmDialog(`"${t}" tidak ada di SSoT. Tambah sebagai barang baru?`, {
      title: "Barang tidak ditemukan",
      okText: "Tambah Baru",
      icon: "🔍",
    });
    if (ok) openNew(looksLikeBarcode ? { barcode: t } : { name: t });
    else searchEl?.focus();
  }

  function toInput(p: Product): ProductInput {
    return {
      id: p.id, name: p.name, barcode: p.barcode, category: p.category, brand: p.brand,
      unit: p.unit, sell_price: p.sell_price, cost_price: p.cost_price,
      default_discount: p.default_discount, is_active: p.is_active,
    };
  }

  function openEdit(p: Product) {
    form = toInput(p);
    formUpdatedAt = p.updated_at;
  }

  function openNew(prefill: Partial<ProductInput> = {}) {
    form = {
      id: null, name: "", barcode: "", category: "", brand: "", unit: "pcs",
      sell_price: 0, cost_price: 0, default_discount: 0, is_active: true,
      ...prefill,
    };
    formUpdatedAt = null;
  }

  async function closeForm() {
    form = null;
    search = "";
    term = "";
    page = 0;
    highlight = 0;
    await tick();
    searchEl?.focus();
  }

  async function save(e: Event) {
    e.preventDefault();
    if (!form) return;
    const bc = form.barcode?.trim() ?? "";
    if (!form.name.trim()) return showToast("Nama barang wajib diisi.", "error");
    if (!bc) return showToast("Barcode wajib diisi.", "error");
    if (!form.brand?.trim()) return showToast("Merek wajib diisi.", "error");
    if ((form.sell_price ?? 0) <= 0) return showToast("Harga jual harus lebih dari 0.", "error");
    if ((form.cost_price ?? 0) <= 0) return showToast("Harga pokok harus lebih dari 0.", "error");
    const clash = all.find((p) => (p.barcode ?? "").trim() === bc && p.id !== form!.id);
    if (clash) return showToast(`Barcode "${bc}" sudah dipakai oleh "${clash.name}" di SSoT.`, "error");

    saving = true;
    try {
      const res = await api.ssotSaveProduct({ ...form, barcode: bc }, applyLocal);
      all = [...all.filter((p) => p.id !== res.product.id), res.product].sort(byName);
      showToast(`"${res.product.name}" tersimpan di SSoT${applyLocal && !res.local_error ? " & PC ini" : ""}.`, "success");
      if (res.local_error) {
        showToast(`Tersimpan di SSoT, tapi gagal diterapkan ke PC ini: ${res.local_error}`, "error", 8000);
      }
      await closeForm();
    } catch (err) {
      toastError(err);
    } finally {
      saving = false;
    }
  }

  function onFormKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      closeForm();
    }
  }

  function goPage(delta: number) {
    const next = page + delta;
    if (next < 0 || next >= totalPages) return;
    page = next;
    highlight = page * PAGE_SIZE;
  }
</script>

{#if $syncAccount && !$syncAccount.logged_in}
  <div class="page-head"><h1>Data SSoT</h1></div>
  <div class="card" style="max-width:460px;">
    <p style="margin-top:0;">Data SSoT hanya bisa dibuka setelah PC ini login ke server.</p>
    <button class="btn-primary" onclick={openSyncAkun}>🔑 Login di Akun Server</button>
  </div>
{:else}
<div class="view-flex">
  <div class="page-head">
    <h1>Data SSoT</h1>
    <div class="row" style="flex-wrap:wrap; gap:0.4rem;">
      <label class="row inactive-toggle">
        <input type="checkbox" bind:checked={showInactive} onchange={() => { page = 0; highlight = 0; }} />
        Tampilkan nonaktif ({inactiveCount})
      </label>
      <button class="btn-ghost" disabled={loading} onclick={load}>↻ Muat Ulang</button>
      <button class="btn-primary" onclick={() => openNew()}>＋ Barang Baru</button>
    </div>
  </div>

  <div class="card search-card">
    <input
      class="mono search-input"
      placeholder="Scan barcode atau ketik nama… (Enter = buka, ↑/↓ = pilih)"
      bind:this={searchEl}
      bind:value={search}
      oninput={applyTerm}
      onkeydown={onSearchKey}
      disabled={loading && !all.length}
    />
    <div class="text-dim search-hint">
      Perubahan disimpan langsung ke SSoT (server). Toko lain menerimanya saat Sync In.
    </div>
  </div>

  <div class="card list-card">
    <div class="list-scroll">
      <table>
        <thead>
          <tr>
            <th style="width:140px;">Barcode</th>
            <th>Nama</th><th>Merek</th><th>Kategori</th><th>Satuan</th>
            <th class="text-right">H.Pokok</th><th class="text-right">H.Jual</th>
            <th class="text-right">Diskon</th><th>Aktif</th><th>Diubah</th>
          </tr>
        </thead>
        <tbody>
          {#each pageRows as p, i (p.id)}
            {@const idx = page * PAGE_SIZE + i}
            <tr class="ssot-row" class:active={idx === highlight} class:inactive={!p.is_active} data-ssot-index={idx} onclick={() => openEdit(p)}>
              <td class="mono text-dim">{p.barcode ?? "-"}</td>
              <td>{p.name}</td>
              <td>{p.brand ?? "-"}</td>
              <td>{p.category ?? "-"}</td>
              <td>{p.unit ?? "-"}</td>
              <td class="mono text-right">{formatIDR(p.cost_price)}</td>
              <td class="mono text-right">{formatIDR(p.sell_price)}</td>
              <td class="mono text-right">{p.default_discount ? formatIDR(p.default_discount) : "-"}</td>
              <td>{p.is_active ? "✓" : "—"}</td>
              <td class="text-dim nowrap">{formatDateTime(p.updated_at)}</td>
            </tr>
          {:else}
            <tr>
              <td colspan="10" class="text-dim">
                {#if loading}Memuat data SSoT…{:else if loadError}Gagal memuat: {loadError}{:else if term.trim()}Tidak ada yang cocok — tekan Enter untuk menambah barang baru.{:else}SSoT masih kosong.{/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <div class="pager">
      <span class="text-dim" style="font-size:0.82rem;">
        {filtered.length.toLocaleString("id-ID")} dari {all.length.toLocaleString("id-ID")} barang · Hal {page + 1} / {totalPages}
      </span>
      <div class="row" style="gap:0.3rem;">
        <button disabled={page === 0} onclick={() => goPage(-1)}>‹ Sebelumnya</button>
        <button disabled={page + 1 >= totalPages} onclick={() => goPage(1)}>Berikutnya ›</button>
      </div>
    </div>
  </div>
</div>

{/if}

{#if form}
  <div class="modal-backdrop" onclick={closeForm} role="presentation">
    <div class="modal ssot-form" onclick={(e) => e.stopPropagation()} onkeydown={onFormKey} role="presentation">
      <h2>{form.id ? "Ubah Barang di SSoT" : "Barang Baru di SSoT"}</h2>
      {#if formUpdatedAt}
        <div class="text-dim" style="font-size:0.78rem; margin-bottom:0.6rem;">Terakhir diubah {formatDateTime(formUpdatedAt)}</div>
      {/if}
      <form onsubmit={save}>
        <div class="grid-2">
          <div style="grid-column:1/3;">
            <label>Nama Barang *</label>
            <!-- svelte-ignore a11y_autofocus -->
            <input bind:value={form.name} autofocus />
          </div>
          <div>
            <label>Barcode *</label>
            <input class="mono" bind:value={form.barcode} placeholder="Wajib & unik" />
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
            <input class="mono" inputmode="decimal" value={formatMoneyInput(form.cost_price)} oninput={(e) => onMoneyInput(e, (n) => (form!.cost_price = n))} />
          </div>
          <div>
            <label>Harga Jual *</label>
            <input class="mono" inputmode="decimal" value={formatMoneyInput(form.sell_price)} oninput={(e) => onMoneyInput(e, (n) => (form!.sell_price = n))} />
          </div>
          <div>
            <label>Default Diskon (Rp)</label>
            <input class="mono" inputmode="decimal" value={formatMoneyInput(form.default_discount)} oninput={(e) => onMoneyInput(e, (n) => (form!.default_discount = n))} />
          </div>
          <div>
            <label>Status</label>
            <label class="row check-row">
              <input type="checkbox" bind:checked={form.is_active} /> Aktif
            </label>
          </div>
        </div>
        <label class="row check-row" style="margin-top:0.8rem;">
          <input type="checkbox" bind:checked={applyLocal} /> Terapkan juga ke PC ini
        </label>
        <div class="row" style="justify-content:flex-end; margin-top:1rem; gap:0.5rem;">
          <button type="button" class="btn-ghost" onclick={closeForm}>Batal</button>
          <button class="btn-primary" disabled={saving} type="submit">{saving ? "Menyimpan…" : "Simpan ke SSoT"}</button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .view-flex { height: 100%; min-height: 0; display: flex; flex-direction: column; }
  .search-card { margin-bottom: 0.8rem; flex-shrink: 0; }
  .search-input { font-size: 1.05rem; padding: 0.7rem 0.8rem; }
  .search-hint { font-size: 0.78rem; margin-top: 0.4rem; }
  .list-card { padding: 0; overflow: hidden; flex: 1; min-height: 0; display: flex; flex-direction: column; }
  .list-scroll { flex: 1; min-height: 0; overflow: auto; }
  .pager {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.5rem 0.9rem; border-top: 1px solid var(--border); flex-shrink: 0;
  }
  .ssot-row { cursor: pointer; }
  .ssot-row:hover { background: var(--baby-blue-bg); }
  .ssot-row.active { background: var(--baby-blue-bg); box-shadow: inset 3px 0 0 var(--primary); }
  .ssot-row.inactive td { opacity: 0.55; }
  .nowrap { white-space: nowrap; font-size: 0.78rem; }
  .ssot-form { max-width: 560px; width: 100%; }
  /* `input { width:100% }` global ikut kena checkbox — kembalikan ke ukuran asli. */
  .check-row, .inactive-toggle { margin: 0; gap: 0.4rem; font-weight: 400; align-items: center; }
  .check-row input[type="checkbox"], .inactive-toggle input[type="checkbox"] { width: auto; flex: none; }
  .inactive-toggle { font-size: 0.85rem; }
</style>
