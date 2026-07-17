<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR, formatQty, formatTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import { debounce } from "$lib/debounce";
  import { createLiveClock } from "$lib/liveClock.svelte";
  import { todayIso, combineDateAndTime } from "$lib/dateTime";
  import { pendingSales, addPending, removePending } from "$lib/stores/pendingSales";
  import { parseReceiptConfig, type ReceiptConfig } from "$lib/receipt";
  import { buildReceiptEscPos } from "$lib/escpos";
  import type { Customer, DiscountPeriod, PaymentMethod, ProductWithStock, SaleInput, Shift, TransactionDetail } from "$lib/types";

  const clock = createLiveClock();
  onDestroy(() => clock.stop());

  let tanggal = $state(todayIso());

  interface CartLine {
    product_id: string;
    name: string;
    price: number;
    qty: number;
    discount: number;
    brand: string | null;
    default_discount: number;
    periodic: boolean;
    manualOverride: boolean;
    stock_qty: number;
  }

  let discounts = $state<DiscountPeriod[]>([]);
  let search = $state("");
  let searchBusy = $state(false);
  let stockAlert = $state<{ name: string; available: number } | null>(null);
  let showPendingList = $state(false);

  // Popup cari-nama: dipicu otomatis saat Enter tapi barcode tidak ditemukan.
  let showSearchPopup = $state(false);
  let popupQuery = $state("");
  let popupResults = $state<ProductWithStock[]>([]);
  let popupLoading = $state(false);
  let cart = $state<CartLine[]>([]);
  let paymentMethod = $state<PaymentMethod>("Tunai");
  let paid = $state(0);
  let receiptCfg = $state<ReceiptConfig | null>(null);
  let lastReceipt = $state<TransactionDetail | null>(null);
  let showPrintConfirm = $state(false);
  let busy = $state(false);

  let activeShift = $state<Shift | null>(null);
  let shiftChecked = $state(false);
  let openingCash = $state(0);
  let openingBusy = $state(false);

  let customers = $state<Customer[]>([]);
  let customerSearch = $state("");
  let selectedCustomer = $state<Customer | null>(null);

  const payments: PaymentMethod[] = ["Tunai", "QRIS", "Transfer", "Kartu"];

  const subtotal = $derived(cart.reduce((s, l) => s + l.price * l.qty, 0));
  const totalDiscount = $derived(cart.reduce((s, l) => s + l.discount, 0));
  const total = $derived(Math.max(subtotal - totalDiscount, 0));
  const change = $derived(Math.max(paid - total, 0));

  async function loadSettings() {
    try {
      const s = await api.getSettings();
      receiptCfg = parseReceiptConfig(s);
    } catch (e) {
      toastError(e);
    }
  }
  async function loadDiscounts() {
    try {
      discounts = await api.listDiscounts();
    } catch (e) {
      toastError(e);
    }
  }
  async function loadShift() {
    try {
      activeShift = await api.getActiveShift();
    } catch (e) {
      toastError(e);
    } finally {
      shiftChecked = true;
    }
  }
  async function loadCustomers() {
    try {
      customers = await api.listCustomers();
    } catch (e) {
      toastError(e);
    }
  }
  onMount(() => {
    loadSettings();
    loadDiscounts();
    loadShift();
    loadCustomers();
  });

  async function doOpenShift() {
    if (!$currentUser) return;
    openingBusy = true;
    try {
      activeShift = await api.openShift({
        user_id: $currentUser.username,
        user_name: $currentUser.name,
        opening_cash: openingCash,
      });
      showToast("Shift dibuka. Selamat berjualan!", "success");
      openingCash = 0;
    } catch (e) {
      toastError(e);
    } finally {
      openingBusy = false;
    }
  }

  // Sengaja cari berdasarkan No. HP saja (bukan nama) — verifikasi sederhana
  // supaya tidak sembarang orang bisa memakai akun pelanggan lain di kasir.
  const customerResults = $derived(
    customerSearch.trim()
      ? customers.filter((c) => (c.phone ?? "").toLowerCase().includes(customerSearch.trim().toLowerCase()))
      : [],
  );

  // JS getDay(): 0=Min..6=Sab → kunci hari di UI Diskon Periodik.
  const DAY_KEYS = ["min", "sen", "sel", "rab", "kam", "jum", "sab"];
  const todayKey = () => DAY_KEYS[new Date().getDay()];

  function dayMatches(days: string, key: string): boolean {
    if (days === "everyday") return true;
    return days.split(",").map((d) => d.trim()).includes(key);
  }

  function discountValue(d: DiscountPeriod, price: number, qty: number): number {
    return d.discount_type === "percent" ? (price * qty * d.value) / 100 : d.value * qty;
  }

  /** Hitung ulang diskon baris: diskon periodik (item > brand, ambil nominal terbesar) atau fallback default. */
  function applyDiscount(line: CartLine, qty: number) {
    const key = todayKey();
    const matches = discounts.filter(
      (d) =>
        d.is_active &&
        dayMatches(d.days, key) &&
        ((d.scope === "item" && d.target === line.product_id) ||
          (d.scope === "brand" && d.target === line.brand)),
    );
    let discount: number;
    if (matches.length) {
      const items = matches.filter((d) => d.scope === "item");
      const pool = items.length ? items : matches;
      discount = Math.max(...pool.map((d) => discountValue(d, line.price, qty)));
      line.periodic = true;
    } else {
      discount = line.default_discount * qty;
      line.periodic = false;
    }
    line.discount = Math.min(Math.max(discount, 0), line.price * qty);
  }

  function addToCart(p: ProductWithStock) {
    const ex = cart.find((l) => l.product_id === p.id);
    const currentQty = ex?.qty ?? 0;
    if (currentQty + 1 > p.stock_qty) {
      stockAlert = { name: p.name, available: p.stock_qty };
      return;
    }
    if (ex) {
      ex.qty += 1;
      ex.stock_qty = p.stock_qty;
      if (!ex.manualOverride) applyDiscount(ex, ex.qty);
      cart = [...cart];
    } else {
      const line: CartLine = {
        product_id: p.id,
        name: p.name,
        price: p.sell_price,
        qty: 1,
        discount: 0,
        brand: p.brand,
        default_discount: p.default_discount,
        periodic: false,
        manualOverride: false,
        stock_qty: p.stock_qty,
      };
      applyDiscount(line, 1);
      cart = [...cart, line];
    }
  }

  async function onSearchKey(e: KeyboardEvent) {
    if (e.key !== "Enter") return;
    const term = search.trim();
    if (!term) return;
    searchBusy = true;
    try {
      const p = await api.findByBarcode(term);
      if (p) {
        addToCart(p);
        search = "";
      } else {
        openSearchPopup(term);
      }
    } catch (e) {
      toastError(e);
    } finally {
      searchBusy = false;
    }
  }

  function openSearchPopup(term: string) {
    showSearchPopup = true;
    popupQuery = term;
    runPopupSearch(term);
  }

  async function runPopupSearch(term: string) {
    if (!term.trim()) {
      popupResults = [];
      return;
    }
    popupLoading = true;
    try {
      popupResults = await api.listProducts(term, false, 30);
    } catch (e) {
      toastError(e);
    } finally {
      popupLoading = false;
    }
  }
  const debouncedPopupSearch = debounce((term: string) => runPopupSearch(term), 300);
  function onPopupInput() {
    debouncedPopupSearch(popupQuery);
  }
  function pickFromPopup(p: ProductWithStock) {
    addToCart(p);
    showSearchPopup = false;
    popupQuery = "";
    popupResults = [];
    search = "";
  }

  function setQty(line: CartLine, qty: number) {
    const wanted = Math.max(1, qty);
    if (wanted > line.stock_qty) {
      stockAlert = { name: line.name, available: line.stock_qty };
      line.qty = Math.max(Math.min(line.qty, line.stock_qty), line.stock_qty > 0 ? 1 : line.qty);
    } else {
      line.qty = wanted;
    }
    if (!line.manualOverride) applyDiscount(line, line.qty);
    else line.discount = Math.min(line.discount, line.price * line.qty);
    cart = [...cart];
  }
  function setPrice(line: CartLine, price: number) {
    line.price = Math.max(0, price);
    line.discount = Math.min(line.discount, line.price * line.qty);
    line.manualOverride = true;
    cart = [...cart];
  }
  function setDiscount(line: CartLine, discount: number) {
    line.discount = Math.min(Math.max(0, discount), line.price * line.qty);
    line.manualOverride = true;
    line.periodic = false;
    cart = [...cart];
  }
  const removeLine = (id: string) => (cart = cart.filter((l) => l.product_id !== id));
  function clearCart() {
    cart = [];
    paid = 0;
  }

  function holdCurrentCart() {
    if (cart.length === 0) return;
    addPending({
      label: selectedCustomer?.name ?? `Pending ${$pendingSales.length + 1}`,
      cart: cart.map((l) => ({ ...l })),
      customerId: selectedCustomer?.id ?? null,
      paymentMethod,
      paid,
    });
    clearCart();
    selectedCustomer = null;
    customerSearch = "";
    showToast("Transaksi disimpan sebagai pending.", "success");
  }

  function resumePending(id: string) {
    const p = $pendingSales.find((x) => x.id === id);
    if (!p) return;
    if (cart.length > 0 && !confirm("Keranjang saat ini belum kosong. Timpa dengan transaksi pending ini?")) return;
    cart = p.cart.map((l) => ({ ...l }));
    selectedCustomer = p.customerId ? customers.find((c) => c.id === p.customerId) ?? null : null;
    customerSearch = "";
    paymentMethod = p.paymentMethod as PaymentMethod;
    paid = p.paid;
    removePending(id);
    showPendingList = false;
  }

  async function doCheckout() {
    if (cart.length === 0) return showToast("Keranjang kosong.", "info");
    if (paid < total) return showToast("Pembayaran kurang dari total.", "error");
    if (!activeShift) return showToast("Buka shift terlebih dahulu sebelum bertransaksi.", "error");
    busy = true;
    try {
      const sale: SaleInput = {
        cashier_id: $currentUser?.username ?? "admin",
        payment_method: paymentMethod,
        paid,
        items: cart.map((l) => ({
          product_id: l.product_id,
          name: l.name,
          price: l.price,
          qty: l.qty,
          discount: l.discount,
        })),
        customer_id: selectedCustomer?.id ?? null,
        shift_id: activeShift.id,
        created_at: combineDateAndTime(tanggal, clock.now),
      };
      const tx = await api.checkout(sale);
      lastReceipt = tx;
      showPrintConfirm = true;
      showToast(`Transaksi ${tx.invoice_no} tersimpan.`, "success");
      clearCart();
      selectedCustomer = null;
      customerSearch = "";
      tanggal = todayIso();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  async function doPrintReceipt() {
    if (!lastReceipt || !receiptCfg) return;
    try {
      await api.printEscposTo(receiptCfg.printer, buildReceiptEscPos(lastReceipt, receiptCfg));
      showToast("Struk dikirim ke printer.", "success");
    } catch (e) {
      toastError(e);
    } finally {
      showPrintConfirm = false;
    }
  }

  function onGlobalKey(e: KeyboardEvent) {
    if (!activeShift) return;
    if (e.key === "F3") {
      e.preventDefault();
      openSearchPopup(search.trim());
    } else if (e.key === "F5") {
      e.preventDefault();
      if (cart.length > 0) holdCurrentCart();
    } else if (e.key === "F6") {
      e.preventDefault();
      if (cart.length > 0) clearCart();
    } else if (e.key === "F9") {
      e.preventDefault();
      if (!busy) doCheckout();
    }
  }
  onMount(() => window.addEventListener("keydown", onGlobalKey));
  onDestroy(() => window.removeEventListener("keydown", onGlobalKey));
</script>

<div class="pos-page">
<!-- Informasi header transaksi -->
<div class="pos-header">
  <div class="pos-meta">
    <span class="meta-label">No. Struk</span>
    <span class="meta-val mono">{lastReceipt ? lastReceipt.invoice_no : "— (auto)"}</span>
  </div>
  <div class="pos-meta">
    <span class="meta-label">Tanggal</span>
    {#if $currentUser?.role === "admin"}
      <input class="mono tanggal-input" type="date" max={todayIso()} bind:value={tanggal} title="Admin bisa mundurkan tanggal untuk transaksi yang terlewat" />
    {:else}
      <span class="meta-val mono">{new Date(tanggal).toLocaleDateString("id-ID", { day:"2-digit", month:"short", year:"numeric" })}</span>
    {/if}
  </div>
  <div class="pos-meta">
    <span class="meta-label">Jam</span>
    <span class="meta-val mono">{formatTime(clock.now)}</span>
  </div>
  <div class="pos-meta">
    <span class="meta-label">Kasir</span>
    <span class="meta-val">{$currentUser?.name ?? $currentUser?.username ?? "—"}</span>
  </div>
  <div class="pos-meta pos-customer">
    <span class="meta-label">Pelanggan</span>
    {#if selectedCustomer}
      <div class="cust-selected">
        <span>{selectedCustomer.name}</span>
        <button class="btn-ghost" onclick={() => { selectedCustomer = null; customerSearch = ""; }}>✕</button>
      </div>
    {:else}
      <div style="position:relative;">
        <input placeholder="Umum (opsional, cari No. HP…)" bind:value={customerSearch} />
        {#if customerResults.length > 0}
          <div class="cust-drop">
            {#each customerResults as c (c.id)}
              <button class="cust-row" onclick={() => { selectedCustomer = c; customerSearch = ""; }}>
                <span class="mono">{c.phone}</span>
                <span class="text-dim" style="font-size:0.78rem;">{c.name}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
  <div class="header-actions">
    <button class="btn-ghost" disabled={cart.length === 0} onclick={holdCurrentCart} title="Tahan transaksi ini, layani pelanggan lain dulu (F5)">⏸ Pending (F5)</button>
    {#if $pendingSales.length > 0}
      <button class="btn-ghost" onclick={() => (showPendingList = true)}>📋 Pending ({$pendingSales.length})</button>
    {/if}
    <button class="btn-ghost" disabled={cart.length === 0} onclick={clearCart} title="Kosongkan keranjang (F6)">🗑 Kosongkan (F6)</button>
  </div>
</div>

{#if !shiftChecked}
  <div class="card text-dim" style="text-align:center; padding:2rem;">Memuat…</div>
{:else if !activeShift}
  <div class="card shift-gate">
    <h2>🟢 Buka Shift Dulu</h2>
    <p class="text-dim" style="margin-top:0;">
      Masukkan modal awal (uang tunai di laci) sebelum mulai melayani transaksi.
    </p>
    <label>Modal Awal (Rp)</label>
    <input type="number" min="0" bind:value={openingCash} />
    <button class="btn-primary" style="margin-top:1rem; width:100%;" disabled={openingBusy} onclick={doOpenShift}>
      Buka Shift &amp; Mulai Jualan
    </button>
  </div>
{:else}
<div class="pos">
  <!-- Kiri: pencarian + keranjang -->
  <section class="main-panel">
    <!-- Baris scan + item count -->
    <div class="scan-row">
      <input class="scan-input" placeholder="Scan barcode / cari nama lalu Enter…" bind:value={search} onkeydown={onSearchKey} disabled={searchBusy} />
      <button class="btn-ghost" title="Cari nama barang (F3)" onclick={() => openSearchPopup(search.trim())}>🔍</button>
      {#if cart.length > 0}
        <span class="item-count">{cart.reduce((s, l) => s + l.qty, 0)} item</span>
      {/if}
    </div>

    <!-- Tabel keranjang -->
    <div class="cart-table-wrap">
      <table class="cart-table">
        <thead>
          <tr>
            <th style="width:2rem">No</th>
            <th>Nama Barang</th>
            <th style="width:120px">Jumlah</th>
            <th style="width:90px" class="text-right">Harga</th>
            <th style="width:90px" class="text-right">Diskon</th>
            <th style="width:90px" class="text-right">Total</th>
            <th style="width:2rem"></th>
          </tr>
        </thead>
        <tbody>
          {#each cart as line, i (line.product_id)}
            <tr>
              <td class="mono text-dim">{i + 1}</td>
              <td>
                <div class="cl-name">
                  {line.name}
                  {#if line.discount > 0}<span class="disc-badge">Diskon</span>{/if}
                  {#if line.stock_qty < 3}<span class="stock-badge">tinggal {formatQty(line.stock_qty)}</span>{/if}
                </div>
              </td>
              <td>
                <div class="cl-qty">
                  <button onclick={() => setQty(line, line.qty - 1)}>−</button>
                  <input class="qty-input mono" type="number" min="1" value={line.qty} oninput={(e) => setQty(line, +e.currentTarget.value)} />
                  <button onclick={() => setQty(line, line.qty + 1)}>+</button>
                </div>
              </td>
              {#if $currentUser?.role === "admin"}
                <td>
                  <input
                    class="mono cell-num"
                    type="number"
                    min="0"
                    value={line.price}
                    oninput={(e) => setPrice(line, +e.currentTarget.value)}
                  />
                </td>
                <td>
                  <input
                    class="mono cell-num"
                    type="number"
                    min="0"
                    value={line.discount}
                    oninput={(e) => setDiscount(line, +e.currentTarget.value)}
                  />
                </td>
              {:else}
                <td class="text-right mono">{formatIDR(line.price)}</td>
                <td class="text-right mono">{line.discount > 0 ? "−" + formatIDR(line.discount) : "—"}</td>
              {/if}
              <td class="text-right mono fw-bold">{formatIDR(line.price * line.qty - line.discount)}</td>
              <td><button class="btn-ghost cl-del" onclick={() => removeLine(line.product_id)}>✕</button></td>
            </tr>
          {:else}
            <tr><td colspan="7" class="text-dim" style="text-align:center; padding:1.5rem 0;">Belum ada item — scan barcode atau cari nama barang.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>

  <!-- Kanan: panel pembayaran (tetap) -->
  <section class="pay-panel card">
    <div class="totals">
      <div class="trow"><span>Subtotal</span><span class="mono">{formatIDR(subtotal)}</span></div>
      <div class="trow"><span>Diskon</span><span class="mono">−{formatIDR(totalDiscount)}</span></div>
      <div class="trow grand"><span>Total</span><span class="mono">{formatIDR(total)}</span></div>
    </div>
    <div class="change-block">
      <div class="trow grand"><span>Kembalian</span><span class="mono">{formatIDR(change)}</span></div>
    </div>

    <div class="pay">
      <label>Metode Pembayaran</label>
      <div class="pay-methods">
        {#each payments as m}<button class:active={paymentMethod === m} onclick={() => (paymentMethod = m)}>{m}</button>{/each}
      </div>
      <label>Bayar</label>
      <input class="mono" type="number" min="0" bind:value={paid} />
      <div class="quick">
        <button onclick={() => (paid = total)}>Uang Pas</button>
        <button onclick={() => (paid = 50000)}>50rb</button>
        <button onclick={() => (paid = 100000)}>100rb</button>
      </div>
      <button class="btn-primary checkout" disabled={busy || cart.length === 0} onclick={doCheckout} title="Bayar & Simpan (F9)">Bayar &amp; Simpan (F9)</button>
    </div>
  </section>
</div>

<div class="shortcut-bar no-print">
  <span><kbd>F3</kbd> Cari Barang</span>
  <span><kbd>F5</kbd> Pending</span>
  <span><kbd>F6</kbd> Kosongkan</span>
  <span><kbd>F9</kbd> Bayar &amp; Simpan</span>
</div>
{/if}
</div>

{#if showPrintConfirm && lastReceipt}
  <div class="modal-backdrop" onclick={() => (showPrintConfirm = false)} role="presentation">
    <div class="modal print-confirm" onclick={(e) => e.stopPropagation()} role="presentation">
      <div class="stock-alert-icon">✅</div>
      <h2>Transaksi Tersimpan</h2>
      <p class="text-dim mono" style="margin:0.3rem 0 1rem;">{lastReceipt.invoice_no} · {formatIDR(lastReceipt.total)}</p>
      <div class="row" style="gap:0.5rem;">
        <button class="btn-ghost" style="flex:1;" onclick={() => (showPrintConfirm = false)}>Tidak</button>
        <button class="btn-primary" style="flex:1;" onclick={doPrintReceipt}>🖨️ Cetak Struk</button>
      </div>
    </div>
  </div>
{/if}

{#if stockAlert}
  <div class="modal-backdrop" onclick={() => (stockAlert = null)} role="presentation">
    <div class="modal stock-alert" onclick={(e) => e.stopPropagation()} role="presentation">
      <div class="stock-alert-icon">{stockAlert.available <= 0 ? "🚫" : "⚠️"}</div>
      <h2>{stockAlert.available <= 0 ? "Barang Kosong" : "Stok Tidak Cukup"}</h2>
      <p class="text-dim" style="margin:0.3rem 0 1rem;">
        {stockAlert.available <= 0
          ? `"${stockAlert.name}" stoknya kosong (tersedia 0).`
          : `Stok "${stockAlert.name}" tinggal ${formatQty(stockAlert.available)}, tidak cukup untuk ditambah lagi.`}
      </p>
      <button class="btn-primary" style="width:100%;" onclick={() => (stockAlert = null)}>Tutup</button>
    </div>
  </div>
{/if}

{#if showPendingList}
  <div class="modal-backdrop" onclick={() => (showPendingList = false)} role="presentation">
    <div class="modal pending-modal" onclick={(e) => e.stopPropagation()} role="presentation">
      <h2>📋 Transaksi Pending</h2>
      <div class="pending-list">
        {#each $pendingSales as p (p.id)}
          <div class="pending-row">
            <div class="pending-info">
              <b>{p.label}</b>
              <span class="text-dim" style="font-size:0.8rem;">
                {p.cart.reduce((s, l) => s + l.qty, 0)} item · {formatIDR(p.cart.reduce((s, l) => s + l.price * l.qty - l.discount, 0))} · {formatTime(new Date(p.createdAt))}
              </span>
            </div>
            <div class="row">
              <button class="btn-primary" onclick={() => resumePending(p.id)}>Lanjutkan</button>
              <button class="btn-ghost" style="color:var(--danger);" onclick={() => removePending(p.id)}>Hapus</button>
            </div>
          </div>
        {:else}
          <div class="text-dim" style="text-align:center; padding:1rem;">Tidak ada transaksi pending.</div>
        {/each}
      </div>
      <div class="row" style="justify-content:flex-end; margin-top:0.8rem;">
        <button class="btn-ghost" onclick={() => (showPendingList = false)}>Tutup</button>
      </div>
    </div>
  </div>
{/if}

{#if showSearchPopup}
  <div class="modal-backdrop" onclick={() => (showSearchPopup = false)} role="presentation">
    <div class="modal popup-search" onclick={(e) => e.stopPropagation()} role="presentation">
      <h2>🔍 Cari Barang</h2>
      <input
        class="mono"
        placeholder="Ketik nama atau barcode…"
        bind:value={popupQuery}
        oninput={onPopupInput}
        autofocus
      />
      <div class="popup-results">
        {#if popupLoading}
          <div class="sr-empty text-dim">Mencari…</div>
        {:else}
          {#each popupResults as p (p.id)}
            <button class="sr-row" onclick={() => pickFromPopup(p)}>
              <span class="sr-name">{p.name}</span>
              <span class="sr-meta text-dim">{p.barcode ?? ""}</span>
              <span class="sr-price mono">{formatIDR(p.sell_price)}</span>
              <span class="sr-stock text-dim">stok {formatQty(p.stock_qty)}</span>
            </button>
          {:else}
            <div class="sr-empty text-dim">{popupQuery.trim() ? "Tidak ditemukan." : "Ketik untuk mencari…"}</div>
          {/each}
        {/if}
      </div>
      <div class="row" style="justify-content:flex-end; margin-top:0.8rem;">
        <button class="btn-ghost" onclick={() => (showSearchPopup = false)}>Tutup</button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Header struk */
  /* Halaman: kolom flex penuh tinggi workspace — hanya daftar barang yang
     scroll sendiri, header/panel bayar/shortcut-bar tetap diam di tempat. */
  .pos-page { height:100%; min-height:0; display:flex; flex-direction:column; }

  .pos-header {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    background: var(--white);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.55rem 1rem;
    margin-bottom: 0.7rem;
    flex-wrap: wrap;
    flex-shrink: 0;
  }
  .pos-meta { display:flex; flex-direction:column; gap:0.05rem; }
  .meta-label { font-size:0.7rem; color:var(--text-dim); text-transform:uppercase; letter-spacing:0.05em; }
  .meta-val { font-size:0.9rem; font-weight:600; }
  .tanggal-input { width:150px; padding:0.3rem 0.5rem; font-size:0.85rem; }
  .pos-customer { min-width:220px; }
  .pos-customer input { width:220px; padding:0.35rem 0.5rem; font-size:0.85rem; }
  .header-actions { display:flex; align-items:center; gap:0.5rem; margin-left:auto; }

  /* Layout utama: sisa tinggi setelah header, hanya cart-table-wrap yang scroll */
  .pos { display:grid; grid-template-columns:1fr 340px; gap:0.9rem; flex:1; min-height:0; }

  /* Panel kiri */
  .main-panel { display:flex; flex-direction:column; gap:0.5rem; min-height:0; overflow:hidden; }

  /* Bar shortcut keyboard di bawah */
  .shortcut-bar {
    flex-shrink: 0;
    display: flex; flex-wrap: wrap; gap: 1rem;
    margin-top: 0.6rem; padding: 0.4rem 0.2rem;
    font-size: 0.78rem; color: var(--text-dim);
  }
  .shortcut-bar kbd {
    display: inline-block; min-width: 1.6rem; text-align: center;
    font-family: inherit; font-weight: 700; font-size: 0.72rem;
    background: var(--baby-blue-bg); border: 1px solid var(--border);
    border-radius: 4px; padding: 0.08rem 0.3rem; margin-right: 0.3rem;
  }

  /* Popup konfirmasi cetak struk setelah transaksi tersimpan */
  .print-confirm { max-width: 340px; text-align: center; }

  /* Baris scan */
  .scan-row { display:flex; align-items:center; gap:0.6rem; }
  .scan-input { flex:1; font-size:1.25rem; padding:0.85rem 1rem; }
  .item-count {
    white-space:nowrap; font-size:0.8rem; font-weight:700;
    background:var(--primary); color:#fff;
    padding:0.25rem 0.7rem; border-radius:999px;
  }

  /* Popup transaksi pending */
  .pending-modal { max-width: 480px; }
  .pending-list { margin-top: 0.6rem; max-height: 360px; overflow-y: auto; }
  .pending-row {
    display: flex; align-items: center; justify-content: space-between; gap: 0.6rem;
    padding: 0.6rem 0.2rem; border-bottom: 1px solid var(--border);
  }
  .pending-row:last-child { border-bottom: none; }
  .pending-info { display: flex; flex-direction: column; gap: 0.15rem; }

  /* Popup barang kosong / stok tidak cukup */
  .stock-alert { max-width: 360px; text-align: center; }
  .stock-alert-icon { font-size: 2.2rem; margin-bottom: 0.3rem; }
  .stock-alert h2 { margin: 0; }

  /* Popup cari nama barang */
  .popup-search { max-width: 480px; }
  .popup-results {
    margin-top: 0.6rem;
    background: var(--white);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    max-height: 320px;
    overflow-y: auto;
  }
  .sr-row {
    display: grid;
    grid-template-columns: 1fr auto auto auto;
    gap: 0.6rem;
    align-items: center;
    width: 100%;
    text-align: left;
    border: none;
    border-radius: 0;
    border-bottom: 1px solid var(--border);
    padding: 0.45rem 0.8rem;
    font-size: 0.85rem;
  }
  .sr-row:last-child { border-bottom: none; }
  .sr-name { font-weight: 600; }
  .sr-meta, .sr-stock { font-size: 0.78rem; }
  .sr-empty { padding: 0.7rem 0.8rem; font-size: 0.85rem; }

  /* Tabel keranjang */
  .cart-table-wrap { flex:1; min-height:0; overflow-y:auto; border:1px solid var(--border); border-radius:var(--radius); background:var(--white); }
  .cart-table { width:100%; border-collapse:collapse; }
  .cart-table thead th {
    background: var(--baby-blue-bg);
    padding: 0.5rem 0.6rem;
    font-size: 0.8rem;
    font-weight: 650;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
    z-index: 1;
  }
  .cart-table tbody td { padding: 0.45rem 0.6rem; border-bottom: 1px solid var(--border); vertical-align: middle; font-size: 0.95rem; }
  .cart-table tbody tr:last-child td { border-bottom: none; }
  .cart-table tbody tr:hover { background: var(--baby-blue-soft); }
  .cl-name { display:flex; align-items:center; gap:0.4rem; flex-wrap:wrap; font-weight:600; font-size:1.05rem; }
  .disc-badge { font-size:0.62rem; font-weight:700; text-transform:uppercase; letter-spacing:0.03em; padding:0.1rem 0.32rem; border-radius:999px; background:var(--primary); color:#fff; }
  .stock-badge { font-size:0.62rem; font-weight:700; text-transform:uppercase; letter-spacing:0.03em; padding:0.1rem 0.32rem; border-radius:999px; background:rgba(214, 69, 69, 0.55); color:#fff; opacity:0.85; }
  .cl-qty { display:flex; align-items:center; gap:0.2rem; }
  .cl-qty button { padding:0.15rem 0.4rem; font-size:0.85rem; }
  .qty-input { width:44px; text-align:center; padding:0.2rem; }
  .cell-num { width:100%; text-align:right; padding:0.25rem 0.4rem; }
  .fw-bold { font-weight:700; }
  .cl-del { padding:0.15rem 0.35rem; color:var(--danger); border-color:transparent; background:transparent; }

  /* Panel kanan — dikompakkan supaya selalu muat tanpa perlu scroll; kalau
     window benar-benar sempit, overflow-y:auto jadi jaring pengaman terakhir
     (jauh lebih baik daripada tombol Bayar & Simpan kepotong tak terlihat). */
  .pay-panel { display:flex; flex-direction:column; min-height:0; overflow-y:auto; }
  .totals { border-bottom:1px solid var(--border); padding-bottom:0.4rem; margin-bottom:0.35rem; flex-shrink:0; }
  .trow { display:flex; justify-content:space-between; padding:0.12rem 0; }
  .trow.grand { font-size:1.05rem; font-weight:700; border-top:1px solid var(--border); margin-top:0.2rem; padding-top:0.3rem; }
  .change-block { border-top:1px dashed var(--border); margin:0.25rem 0 0.4rem; padding-top:0.3rem; flex-shrink:0; }
  .pay { flex:1; min-height:0; display:flex; flex-direction:column; gap:0.3rem; }
  .pay label { font-size:0.78rem; font-weight:600; color:var(--text-dim); text-transform:uppercase; letter-spacing:0.04em; margin:0; }
  .pay-methods { display:grid; grid-template-columns:repeat(2,1fr); gap:0.3rem; }
  .pay-methods button.active { background:var(--primary); border-color:var(--primary); color:#fff; }
  .pay-methods button, .quick button { padding:0.4rem 0.5rem; }
  .quick { display:grid; grid-template-columns:repeat(3,1fr); gap:0.3rem; }
  .checkout { width:100%; padding:0.65rem; font-size:0.95rem; margin-top:auto; flex-shrink:0; }

  /* Gate buka shift */
  .shift-gate { max-width:420px; margin:0 auto; }

  /* Pemilih pelanggan */
  .cust-selected { display:flex; align-items:center; justify-content:space-between; background:var(--baby-blue-soft); border-radius:var(--radius); padding:0.4rem 0.6rem; }
  .cust-drop {
    position: absolute; left: 0; right: 0; top: 100%; z-index: 100;
    background: var(--white); border: 1px solid var(--border);
    border-radius: 0 0 var(--radius) var(--radius);
    box-shadow: var(--shadow); max-height: 180px; overflow-y: auto;
  }
  .cust-row {
    display: flex; justify-content: space-between; width: 100%; text-align: left;
    border: none; border-radius: 0; border-bottom: 1px solid var(--border);
    padding: 0.4rem 0.7rem; font-size: 0.85rem; font-weight: 500;
  }
  .cust-row:last-child { border-bottom: none; }
</style>
