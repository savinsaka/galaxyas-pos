<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { api } from "$lib/api";
  import { formatQty, formatTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import { createLiveClock } from "$lib/liveClock.svelte";
  import { markStockBatchesDirty } from "$lib/stores/stockBatchSignal";
  import { setTabDirty, clearTabDirty } from "$lib/stores/tabGuard";
  import { activeTabId } from "$lib/stores/tabs";
  import type { ProductWithStock, DestStore, StockMovementBatchDetail } from "$lib/types";
  import ProductSearchPopup from "$lib/components/ProductSearchPopup.svelte";
  import StockDocPrint from "$lib/components/StockDocPrint.svelte";
  import ShortcutBar from "$lib/components/ShortcutBar.svelte";

  let { tabId }: { tabId?: string } = $props();

  const clock = createLiveClock();
  onDestroy(() => clock.stop());
  onDestroy(() => { if (tabId) clearTabDirty(tabId); });

  interface CartLine {
    product_id: string;
    name: string;
    barcode: string | null;
    qty: number;
    stock_qty: number;
  }

  let products = $state<ProductWithStock[]>([]);
  let stores = $state<DestStore[]>([]);
  let storeId = $state("");
  let storesLoading = $state(false);
  let storesError = $state("");
  let catatan = $state("");

  let search = $state("");
  let scanQty = $state(1);
  // Alur: scan barcode DULU -> item jadi "pending" -> isi Jumlah -> Enter baru
  // masuk keranjang (bukan langsung masuk). Sesuai permintaan.
  let pendingScanProduct = $state<ProductWithStock | null>(null);
  let searchBusy = $state(false);
  let cart = $state<CartLine[]>([]);
  let stockAlert = $state<{ name: string; available: number } | null>(null);
  let showPopup = $state(false);
  let busy = $state(false);
  let lastSaved = $state<StockMovementBatchDetail | null>(null);
  let showPrint = $state(false);

  let scanInputEl = $state<HTMLInputElement>();
  let scanQtyEl = $state<HTMLInputElement>();

  const selectedStore = $derived(stores.find((s) => s.id === storeId) ?? null);
  const totalItem = $derived(cart.reduce((s, l) => s + l.qty, 0));

  $effect(() => {
    if (tabId) setTabDirty(tabId, cart.length > 0);
  });

  async function loadProducts() {
    try {
      products = await api.listProducts("", true);
      // Segarkan stok baris keranjang yang sudah ada.
      cart = cart.map((l) => {
        const p = products.find((x) => x.id === l.product_id);
        return p ? { ...l, stock_qty: p.stock_qty } : l;
      });
    } catch (e) { toastError(e); }
  }

  async function loadStores() {
    storesLoading = true;
    storesError = "";
    try {
      stores = await api.shipListStores();
      if (stores.length && !storeId) storeId = stores[0].id;
    } catch (e) {
      storesError = e instanceof Error ? e.message : String(e);
    } finally {
      storesLoading = false;
    }
  }

  onMount(() => {
    loadProducts();
    loadStores();
    tick().then(() => scanInputEl?.focus());
  });

  /** Tambah ke keranjang dengan guard stok — tidak boleh melebihi stok gudang
   * (tidak boleh minus), persis seperti kasir. */
  function addToCart(p: ProductWithStock, addQty: number) {
    if (!(p.barcode ?? "").trim()) {
      showToast(`"${p.name}" belum punya barcode — tidak bisa dikirim.`, "error", 5000);
      return false;
    }
    const wanted = Math.max(1, Math.floor(addQty) || 1);
    const ex = cart.find((l) => l.product_id === p.id);
    const currentQty = ex?.qty ?? 0;
    if (p.stock_qty <= 0 || currentQty + wanted > p.stock_qty) {
      stockAlert = { name: p.name, available: p.stock_qty };
      return false;
    }
    if (ex) {
      ex.qty += wanted;
      ex.stock_qty = p.stock_qty;
      cart = [...cart];
    } else {
      cart = [...cart, { product_id: p.id, name: p.name, barcode: p.barcode, qty: wanted, stock_qty: p.stock_qty }];
    }
    lastSaved = null;
    return true;
  }

  function setQty(line: CartLine, qty: number) {
    const wanted = Math.max(1, Math.floor(qty) || 1);
    if (wanted > line.stock_qty) {
      stockAlert = { name: line.name, available: line.stock_qty };
      line.qty = Math.max(Math.min(line.qty, line.stock_qty), 1);
    } else {
      line.qty = wanted;
    }
    cart = [...cart];
  }

  const removeLine = (id: string) => (cart = cart.filter((l) => l.product_id !== id));

  async function onSearchKey(e: KeyboardEvent) {
    if (e.key !== "Enter") return;
    const term = search.trim();
    if (!term) return;
    searchBusy = true;
    let p: ProductWithStock | null = null;
    try {
      p = await api.findByBarcode(term);
    } catch (err) { toastError(err); }
    searchBusy = false;
    await tick();
    if (p) {
      // Ketemu barcode -> jadi pending, tunggu isi Jumlah dulu.
      startPending(p);
    } else {
      showPopup = true;
    }
  }

  async function startPending(p: ProductWithStock) {
    if (p.stock_qty <= 0) {
      stockAlert = { name: p.name, available: 0 };
      search = "";
      await tick();
      scanInputEl?.focus();
      return;
    }
    pendingScanProduct = p;
    scanQty = 1;
    search = "";
    await tick();
    scanQtyEl?.focus();
    scanQtyEl?.select();
  }

  function commitPending() {
    if (!pendingScanProduct) return;
    const ok = addToCart(pendingScanProduct, scanQty);
    if (ok) {
      pendingScanProduct = null;
      scanQty = 1;
      scanInputEl?.focus();
    }
  }

  function onQtyKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      pendingScanProduct = null;
      scanQty = 1;
      scanInputEl?.focus();
      return;
    }
    if (e.key !== "Enter") return;
    e.preventDefault();
    commitPending();
  }

  async function kirim() {
    const store = selectedStore;
    if (!store) return showToast("Pilih toko tujuan dulu.", "error");
    if (!cart.length) return showToast("Belum ada barang untuk dikirim.", "error");

    busy = true;
    try {
      const batch = await api.shipSend(
        store.id,
        store.name,
        cart.map((l) => ({ product_id: l.product_id, barcode: (l.barcode ?? "").trim(), name: l.name, qty: l.qty })),
        $currentUser?.username ?? null,
        catatan || null,
      );
      showToast(`${batch.no}: ${cart.length} barang terkirim ke ${store.name}. Stok gudang berkurang.`, "success", 6000);
      markStockBatchesDirty();
      lastSaved = batch;
      cart = [];
      catatan = "";
      await loadProducts();
      scanInputEl?.focus();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  function onGlobalKey(e: KeyboardEvent) {
    if (tabId && $activeTabId !== tabId) return;
    if (e.key === "F9") {
      e.preventDefault();
      if (!busy) kirim();
    }
  }
  onMount(() => window.addEventListener("keydown", onGlobalKey));
  onDestroy(() => window.removeEventListener("keydown", onGlobalKey));
</script>

<div class="kirim-page">
  <!-- Header: toko tujuan + jam + catatan -->
  <div class="head-bar card">
    <div class="fld" style="min-width:240px;">
      <label>Toko Tujuan</label>
      {#if storesLoading}
        <span class="text-dim" style="font-size:0.85rem;">Memuat toko…</span>
      {:else if stores.length}
        <select bind:value={storeId}>
          {#each stores as s (s.id)}<option value={s.id}>{s.name} ({s.code})</option>{/each}
        </select>
      {:else}
        <button onclick={loadStores}>🔄 Muat daftar toko</button>
      {/if}
      {#if storesError}<span class="store-err" title={storesError}>⚠️ Gagal memuat toko — cek Pengaturan → Server Pengiriman.</span>{/if}
    </div>
    <div class="fld">
      <label>Jam</label>
      <span class="mono" style="font-weight:700;">{formatTime(clock.now)}</span>
    </div>
    <div class="fld" style="flex:1;">
      <label>Catatan Pengiriman</label>
      <input bind:value={catatan} placeholder="opsional (mis. no. surat jalan)" />
    </div>
  </div>

  <!-- Baris scan: barcode dulu -> lalu jumlah -->
  <div class="scan-row">
    <input
      class="scan-input"
      bind:this={scanInputEl}
      placeholder="Scan barcode / ketik nama lalu Enter…"
      bind:value={search}
      onkeydown={onSearchKey}
      disabled={searchBusy || !!pendingScanProduct}
    />
    {#if pendingScanProduct}
      <div class="qty-box">
        <span class="qty-lbl">Jumlah untuk <b>{pendingScanProduct.name}</b> (stok {formatQty(pendingScanProduct.stock_qty)}):</span>
        <input
          class="scan-qty mono"
          type="number"
          min="1"
          bind:this={scanQtyEl}
          bind:value={scanQty}
          onkeydown={onQtyKey}
        />
        <button class="btn-primary" onclick={commitPending}>Tambah</button>
        <button class="btn-ghost" onclick={() => { pendingScanProduct = null; scanInputEl?.focus(); }}>Batal (Esc)</button>
      </div>
    {:else}
      <button class="btn-ghost" title="Cari nama barang" onclick={() => (showPopup = true)}>🔍</button>
    {/if}
    {#if cart.length > 0}<span class="item-count">{totalItem} item</span>{/if}
  </div>

  <!-- Keranjang kirim -->
  <div class="cart-wrap card">
    <table class="cart-table">
      <thead>
        <tr>
          <th style="width:2rem;">No</th>
          <th>Nama Barang</th>
          <th style="width:130px;">Barcode</th>
          <th style="width:80px;" class="text-right">Stok</th>
          <th style="width:130px;">Jumlah Kirim</th>
          <th style="width:2rem;"></th>
        </tr>
      </thead>
      <tbody>
        {#each cart as line, i (line.product_id)}
          <tr>
            <td class="mono text-dim">{i + 1}</td>
            <td class="cl-name">
              {line.name}
              {#if line.stock_qty < 3}<span class="stock-badge">tinggal {formatQty(line.stock_qty)}</span>{/if}
            </td>
            <td class="mono cl-barcode" title={line.barcode ?? ""}>{line.barcode ?? "—"}</td>
            <td class="text-right mono text-dim">{formatQty(line.stock_qty)}</td>
            <td>
              <div class="cl-qty">
                <button onclick={() => setQty(line, line.qty - 1)}>−</button>
                <input class="qty-input mono" type="number" min="1" value={line.qty} oninput={(e) => setQty(line, +e.currentTarget.value)} />
                <button onclick={() => setQty(line, line.qty + 1)}>+</button>
              </div>
            </td>
            <td><button class="btn-ghost cl-del" onclick={() => removeLine(line.product_id)}>✕</button></td>
          </tr>
        {:else}
          <tr><td colspan="6" class="text-dim" style="text-align:center; padding:1.5rem 0;">Belum ada barang — scan barcode lalu isi jumlah.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>

  <div class="bottom-bar">
    <span class="text-dim" style="font-size:0.85rem;">{cart.length} jenis · {formatQty(totalItem)} qty</span>
    <span style="margin-left:auto;"></span>
    <button disabled={!lastSaved} onclick={() => (showPrint = true)} title={lastSaved ? "" : "Kirim dulu sebelum mencetak"}>🖨️ Cetak Surat Jalan</button>
    <button class="btn-primary kirim-btn" disabled={busy || !selectedStore || cart.length === 0} onclick={kirim}>
      🚚 Kirim ke {selectedStore ? selectedStore.name : "Toko"} (F9)
    </button>
  </div>
</div>

<ShortcutBar items={[
  { key: "F9", label: "Kirim", action: kirim, disabled: busy || !selectedStore || cart.length === 0 },
]} />

{#if showPrint && lastSaved}
  <StockDocPrint detail={lastSaved} onClose={() => (showPrint = false)} />
{/if}

{#if showPopup}
  <ProductSearchPopup
    initialQuery={search}
    onClose={() => (showPopup = false)}
    onPick={(p) => { showPopup = false; search = ""; startPending(p); }}
  />
{/if}

{#if stockAlert}
  <div class="modal-backdrop" onclick={() => (stockAlert = null)} role="presentation">
    <div class="modal stock-alert" onclick={(e) => e.stopPropagation()} role="presentation">
      <div class="stock-alert-icon">{stockAlert.available <= 0 ? "🚫" : "⚠️"}</div>
      <h2>{stockAlert.available <= 0 ? "Stok Kosong" : "Stok Tidak Cukup"}</h2>
      <p class="text-dim" style="margin:0.3rem 0 1rem;">
        {stockAlert.available <= 0
          ? `"${stockAlert.name}" stoknya kosong (0) — tidak bisa dikirim.`
          : `Stok "${stockAlert.name}" tinggal ${formatQty(stockAlert.available)}, tidak cukup untuk jumlah itu.`}
      </p>
      <button class="btn-primary" style="width:100%;" onclick={() => (stockAlert = null)}>Tutup</button>
    </div>
  </div>
{/if}

<style>
  .kirim-page { height:100%; min-height:0; display:flex; flex-direction:column; gap:0.6rem; }
  .head-bar { display:flex; align-items:flex-end; gap:1.5rem; flex-wrap:wrap; }
  .fld { display:flex; flex-direction:column; gap:0.2rem; }
  .fld label { font-size:0.78rem; color:var(--text-dim); margin:0; }
  .store-err { font-size:0.72rem; color:var(--danger); margin-top:0.2rem; }

  .scan-row { display:flex; align-items:center; gap:0.6rem; flex-wrap:wrap; }
  .scan-input { flex:1; min-width:240px; font-size:1.2rem; padding:0.8rem 1rem; }
  .qty-box { display:flex; align-items:center; gap:0.5rem; flex-wrap:wrap; background:var(--baby-blue-bg); border:1px solid var(--border); border-radius:var(--radius); padding:0.4rem 0.7rem; }
  .qty-lbl { font-size:0.85rem; color:var(--primary-dark); }
  .scan-qty { width:80px; text-align:center; font-size:1.1rem; padding:0.5rem 0.3rem; }
  .item-count { white-space:nowrap; font-size:0.8rem; font-weight:700; background:var(--primary); color:#fff; padding:0.25rem 0.7rem; border-radius:999px; }

  .cart-wrap { padding:0; overflow:auto; flex:1; min-height:0; }
  .cart-table { width:100%; border-collapse:collapse; }
  .cart-table thead th {
    background:var(--baby-blue-bg); padding:0.5rem 0.6rem; text-align:left;
    font-size:0.78rem; font-weight:650; color:var(--text-dim);
    text-transform:uppercase; letter-spacing:0.04em;
    border-bottom:1px solid var(--border); position:sticky; top:0; z-index:1;
  }
  .cart-table tbody td { padding:0.45rem 0.6rem; border-bottom:1px solid var(--border); vertical-align:middle; font-size:0.95rem; }
  .cart-table tbody tr:last-child td { border-bottom:none; }
  .text-right { text-align:right; }
  .cl-name { display:flex; align-items:center; gap:0.4rem; flex-wrap:wrap; font-weight:600; }
  .cl-barcode { font-size:0.78rem; color:var(--text-dim); white-space:nowrap; overflow:hidden; text-overflow:ellipsis; max-width:130px; }
  .stock-badge { font-size:0.62rem; font-weight:700; text-transform:uppercase; letter-spacing:0.03em; padding:0.1rem 0.32rem; border-radius:999px; background:rgba(214,69,69,0.55); color:#fff; }
  .cl-qty { display:flex; align-items:center; gap:0.2rem; }
  .cl-qty button { padding:0.15rem 0.4rem; font-size:0.85rem; }
  .qty-input { width:52px; text-align:center; padding:0.2rem; }
  .cl-del { padding:0.15rem 0.35rem; color:var(--danger); border-color:transparent; background:transparent; }

  .bottom-bar { display:flex; align-items:center; gap:0.6rem; flex-wrap:wrap; flex-shrink:0; }
  .kirim-btn { padding:0.6rem 1.2rem; font-size:0.95rem; }

  .stock-alert { max-width:360px; text-align:center; }
  .stock-alert-icon { font-size:2.2rem; margin-bottom:0.3rem; }
  .stock-alert h2 { margin:0; }
</style>
