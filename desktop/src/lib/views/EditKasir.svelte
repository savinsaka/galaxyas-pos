<script lang="ts">
  import { get } from "svelte/store";
  import { api } from "$lib/api";
  import { formatIDR, formatQty } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { debounce } from "$lib/debounce";
  import { activeTabId, closeTab } from "$lib/stores/tabs";
  import { markTransactionsDirty } from "$lib/stores/txSignal";
  import { formatMoneyInput, onMoneyInput } from "$lib/moneyInput";
  import type { Customer, PaymentMethod, ProductWithStock, SaleInput, TransactionDetail } from "$lib/types";

  let { transactionId }: { transactionId: string } = $props();

  interface EditLine {
    product_id: string;
    name: string;
    price: number;
    qty: number;
    discount: number;
    /**
     * Persen yang diketik untuk baris ini (null = diskonnya nominal).
     * Sama seperti di Kasir POS: `discount` yang disimpan selalu nominal,
     * persennya cuma cara mengetik supaya "10%" tetap 10% walau Jumlah/Harga
     * diubah.
     */
    manualPercent: number | null;
  }

  let detail = $state<TransactionDetail | null>(null);
  let loading = $state(true);
  let lines = $state<EditLine[]>([]);
  let paymentMethod = $state<PaymentMethod>("Tunai");
  let paid = $state(0);
  let paidCash = $state(0);
  let paidQris = $state(0);
  let busy = $state(false);

  let search = $state("");
  let searchInputEl = $state<HTMLInputElement>();
  /**
   * Cara mengisi kolom Diskon (persis Kasir POS): nominal rupiah atau persen
   * dari harga baris. Satu switch untuk semua baris; default persen.
   */
  let discountMode = $state<"rp" | "percent">("percent");
  let showSearchPopup = $state(false);
  let popupResults = $state<ProductWithStock[]>([]);
  let popupLoading = $state(false);
  let popupHighlight = $state(0);

  let customers = $state<Customer[]>([]);
  let customerSearch = $state("");
  let selectedCustomer = $state<Customer | null>(null);

  const payments: PaymentMethod[] = ["Tunai", "QRIS", "Kombinasi", "Kartu"];

  function onPaidCashInput(e: Event) {
    onMoneyInput(e, (n) => { paidCash = n; paid = paidCash + paidQris; });
  }
  function onPaidQrisInput(e: Event) {
    onMoneyInput(e, (n) => { paidQris = n; paid = paidCash + paidQris; });
  }

  const subtotal = $derived(lines.reduce((s, l) => s + l.price * l.qty, 0));
  const totalDiscount = $derived(lines.reduce((s, l) => s + l.discount, 0));
  const total = $derived(Math.max(subtotal - totalDiscount, 0));
  const change = $derived(Math.max(paid - total, 0));

  // Sengaja cari berdasarkan No. HP saja (bukan nama) — konsisten dengan Kasir POS,
  // verifikasi sederhana supaya tidak sembarang orang memakai akun pelanggan lain.
  const customerResults = $derived(
    customerSearch.trim()
      ? customers.filter((c) => (c.phone ?? "").toLowerCase().includes(customerSearch.trim().toLowerCase()))
      : [],
  );

  async function load() {
    loading = true;
    try {
      const [d, c] = await Promise.all([api.getTransaction(transactionId), api.listCustomers()]);
      customers = c;
      if (!d) {
        showToast("Transaksi tidak ditemukan.", "error");
        detail = null;
        return;
      }
      detail = d;
      lines = d.items.map((it) => ({
        product_id: it.product_id,
        name: it.name,
        price: it.price,
        qty: it.qty,
        discount: it.discount,
        manualPercent: null,
      }));
      paymentMethod = (d.payment_method as PaymentMethod) ?? "Tunai";
      paid = d.paid;
      paidCash = d.paid_cash ?? 0;
      paidQris = d.paid_qris ?? 0;
      selectedCustomer = d.customer_id ? customers.find((x) => x.id === d.customer_id) ?? null : null;
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    transactionId;
    load();
  });

  function addLine(p: ProductWithStock) {
    const ex = lines.find((l) => l.product_id === p.id);
    if (ex) {
      ex.qty += 1;
      lines = [...lines];
    } else {
      lines = [...lines, { product_id: p.id, name: p.name, price: p.sell_price, qty: 1, discount: 0, manualPercent: null }];
    }
  }

  async function onSearchKey(e: KeyboardEvent) {
    if (e.key !== "Enter") return;
    const term = search.trim();
    if (!term) return;
    try {
      const p = await api.findByBarcode(term);
      if (p) {
        addLine(p);
        search = "";
        return;
      }
    } catch (e) {
      toastError(e);
    }
    openSearchPopup(term);
  }

  function openSearchPopup(term: string) {
    showSearchPopup = true;
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
      popupHighlight = 0;
    } catch (e) {
      toastError(e);
    } finally {
      popupLoading = false;
    }
  }
  const debouncedPopupSearch = debounce((term: string) => runPopupSearch(term), 300);
  function onPopupInput() {
    debouncedPopupSearch(search);
  }
  function pickFromPopup(p: ProductWithStock) {
    addLine(p);
    showSearchPopup = false;
    search = "";
    popupResults = [];
  }
  function scrollPopupHighlightIntoView() {
    document.querySelector(`[data-sr-index="${popupHighlight}"]`)?.scrollIntoView({ block: "nearest" });
  }
  function onPopupKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (popupResults.length) popupHighlight = Math.min(popupHighlight + 1, popupResults.length - 1);
      scrollPopupHighlightIntoView();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (popupResults.length) popupHighlight = Math.max(popupHighlight - 1, 0);
      scrollPopupHighlightIntoView();
    } else if (e.key === "Enter") {
      e.preventDefault();
      const p = popupResults[popupHighlight];
      if (p) pickFromPopup(p);
    }
  }

  function setQty(line: EditLine, qty: number) {
    line.qty = Math.max(0.01, qty);
    // Diskon persen ikut jumlah: "10%" tetap 10% walau jumlahnya berubah.
    line.discount =
      line.manualPercent !== null
        ? discountFromPercent(line, line.manualPercent)
        : Math.min(line.discount, line.price * line.qty);
    lines = [...lines];
  }
  function setPrice(line: EditLine, price: number) {
    line.price = Math.max(0, price);
    line.discount =
      line.manualPercent !== null
        ? discountFromPercent(line, line.manualPercent)
        : Math.min(line.discount, line.price * line.qty);
    lines = [...lines];
  }

  /**
   * Ganti cara mengisi diskon untuk semua baris. Nominal yang sudah ada TIDAK
   * diubah — di mode persen angkanya ditampilkan sebagai persen dari harga baris.
   */
  function toggleDiscountMode() {
    discountMode = discountMode === "percent" ? "rp" : "percent";
    searchInputEl?.focus();
  }

  const lineGross = (line: EditLine) => line.price * line.qty;

  /** Nominal dari persen, dibulatkan ke rupiah terdekat (bukan pecahan sen). */
  function discountFromPercent(line: EditLine, percent: number): number {
    const pct = Math.min(Math.max(0, percent), 100);
    return Math.round((lineGross(line) * pct) / 100);
  }

  /**
   * Angka yang tampil di kolom Diskon saat mode persen: yang diketik bila ada,
   * kalau tidak diturunkan dari nominal (mis. diskon dari transaksi lama),
   * dibulatkan 2 desimal supaya tidak jadi 9.999999999.
   */
  function linePercent(line: EditLine): number {
    if (line.manualPercent !== null) return line.manualPercent;
    const gross = lineGross(line);
    if (gross <= 0) return 0;
    return Math.round((line.discount / gross) * 10000) / 100;
  }

  /** Isi kolom Diskon — artinya tergantung switch Rp/% di header. */
  function setDiscount(line: EditLine, value: number) {
    if (discountMode === "percent") {
      const pct = Math.min(Math.max(0, value), 100);
      line.manualPercent = pct;
      line.discount = discountFromPercent(line, pct);
    } else {
      // Nominal yang diketik = angka tetap, tidak lagi mengikuti persen.
      line.manualPercent = null;
      line.discount = Math.min(Math.max(0, value), lineGross(line));
    }
    lines = [...lines];
  }
  function removeLine(id: string) {
    lines = lines.filter((l) => l.product_id !== id);
  }

  function batal() {
    closeTab(get(activeTabId) ?? "");
  }

  async function save() {
    if (!detail) return;
    if (lines.length === 0) return showToast("Transaksi tidak boleh kosong.", "error");
    if (paid < total) return showToast("Pembayaran kurang dari total.", "error");
    busy = true;
    try {
      const sale: SaleInput = {
        cashier_id: detail.cashier_id,
        payment_method: paymentMethod,
        paid,
        items: lines.map((l) => ({
          product_id: l.product_id,
          name: l.name,
          price: l.price,
          qty: l.qty,
          discount: l.discount,
        })),
        customer_id: selectedCustomer?.id ?? null,
        shift_id: detail.shift_id,
        ...(paymentMethod === "Kombinasi" ? { paid_cash: paidCash, paid_qris: paidQris } : {}),
      };
      await api.updateTransaction(detail.id, sale);
      showToast(`Transaksi ${detail.invoice_no} diperbarui.`, "success");
      markTransactionsDirty();
      closeTab(get(activeTabId) ?? "");
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="page-head">
  <h1>Edit Kasir{detail ? ` — ${detail.invoice_no}` : ""}</h1>
</div>

{#if loading}
  <div class="card text-dim" style="text-align:center; padding:2rem;">Memuat…</div>
{:else if !detail}
  <div class="card text-dim" style="text-align:center; padding:2rem;">Transaksi tidak ditemukan.</div>
{:else}
  <div class="scan-row">
    <input
      placeholder="Scan barcode / cari nama lalu Enter…"
      bind:this={searchInputEl}
      bind:value={search}
      onkeydown={onSearchKey}
    />
    <button class="btn-ghost" title="Cari nama barang" onclick={() => openSearchPopup(search.trim())}>🔍</button>
  </div>

  <div class="card" style="padding:0; overflow:hidden; margin:0.8rem 0;">
    <table>
      <thead>
        <tr>
          <th>Barang</th>
          <th style="width:100px">Jumlah</th>
          <th style="width:130px" class="text-right">Harga</th>
          <th style="width:130px" class="text-right">
            <!-- Switch berlaku untuk semua baris; yang berubah cuma cara
                 mengetik, nominal yang sudah ada tidak ikut berubah. -->
            <span class="disc-head">
              Diskon
              <button
                class="disc-toggle"
                class:disc-toggle-on={discountMode === "percent"}
                title={discountMode === "percent"
                  ? "Sekarang isi diskon dalam persen — klik untuk ganti ke nominal Rp"
                  : "Sekarang isi diskon dalam Rupiah — klik untuk ganti ke persen"}
                onclick={toggleDiscountMode}
              >
                {discountMode === "percent" ? "%" : "Rp"}
              </button>
            </span>
          </th>
          <th style="width:120px" class="text-right">Total</th>
          <th style="width:2rem"></th>
        </tr>
      </thead>
      <tbody>
        {#each lines as line (line.product_id)}
          <tr>
            <td>{line.name}</td>
            <td>
              <input
                class="mono cell-num"
                type="number"
                min="0.01"
                step="0.01"
                value={line.qty}
                oninput={(e) => setQty(line, +e.currentTarget.value)}
              />
            </td>
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
              {#if discountMode === "percent"}
                <div class="cl-disc-pct">
                  <input
                    class="mono cell-num"
                    type="number"
                    min="0"
                    max="100"
                    value={linePercent(line)}
                    oninput={(e) => setDiscount(line, +e.currentTarget.value)}
                  />
                  <span class="pct-sign text-dim">%</span>
                </div>
                <!-- Nominalnya tetap ditampilkan (itu yang tersimpan), hanya
                     kalau ada diskon supaya baris tanpa diskon tidak ikut tinggi. -->
                {#if line.discount > 0}
                  <div class="cl-disc-hint mono text-dim">−{formatIDR(line.discount)}</div>
                {/if}
              {:else}
                <input
                  class="mono cell-num"
                  type="number"
                  min="0"
                  value={line.discount}
                  oninput={(e) => setDiscount(line, +e.currentTarget.value)}
                />
              {/if}
            </td>
            <td class="text-right mono fw-bold">{formatIDR(line.price * line.qty - line.discount)}</td>
            <td><button class="btn-ghost" style="color:var(--danger);" onclick={() => removeLine(line.product_id)}>✕</button></td>
          </tr>
        {:else}
          <tr><td colspan="6" class="text-dim" style="text-align:center; padding:1rem 0;">Belum ada item.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>

  <div class="edit-bottom">
    <div class="edit-left card">
      <label>Pelanggan</label>
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

      <label style="margin-top:0.8rem;">Metode Pembayaran</label>
      <div class="pay-methods">
        {#each payments as m}<button class:active={paymentMethod === m} onclick={() => (paymentMethod = m)}>{m}</button>{/each}
      </div>
    </div>

    <div class="edit-right card">
      <div class="trow"><span>Subtotal</span><span class="mono">{formatIDR(subtotal)}</span></div>
      <div class="trow"><span>Diskon</span><span class="mono">−{formatIDR(totalDiscount)}</span></div>
      <div class="trow grand"><span>Total</span><span class="mono">{formatIDR(total)}</span></div>
      {#if paymentMethod === "Kombinasi"}
        <label style="margin-top:0.6rem;">QRIS</label>
        <input class="mono" type="text" inputmode="numeric" value={formatMoneyInput(paidQris)} oninput={onPaidQrisInput} />
        <label style="margin-top:0.4rem;">Tunai</label>
        <input class="mono" type="text" inputmode="numeric" value={formatMoneyInput(paidCash)} oninput={onPaidCashInput} />
      {:else}
        <label style="margin-top:0.6rem;">Bayar</label>
        <input class="mono" type="number" min="0" bind:value={paid} />
      {/if}
      <div class="trow"><span>Kembalian</span><span class="mono">{formatIDR(change)}</span></div>
    </div>
  </div>

  <div class="bottom-bar">
    <button onclick={batal}>Batal</button>
    <button class="btn-primary" disabled={busy} onclick={save}>💾 Simpan Perubahan</button>
  </div>
{/if}

{#if showSearchPopup}
  <div class="modal-backdrop" onclick={() => (showSearchPopup = false)} role="presentation">
    <div class="modal popup-search" onclick={(e) => e.stopPropagation()} role="presentation">
      <h2>🔍 Cari Barang</h2>
      <input class="mono" placeholder="Ketik nama atau barcode…" bind:value={search} oninput={onPopupInput} onkeydown={onPopupKey} autofocus />
      <div class="popup-results">
        {#if popupLoading}
          <div class="sr-empty text-dim">Mencari…</div>
        {:else}
          {#each popupResults as p, i (p.id)}
            <button class="sr-row" class:active={i === popupHighlight} data-sr-index={i} onclick={() => pickFromPopup(p)}>
              <span class="sr-name">{p.name}</span>
              <span class="sr-meta text-dim">{p.barcode ?? ""}</span>
              <span class="sr-price mono">{formatIDR(p.sell_price)}</span>
              <span class="sr-stock text-dim">stok {formatQty(p.stock_qty)}</span>
            </button>
          {:else}
            <div class="sr-empty text-dim">{search.trim() ? "Tidak ditemukan." : "Ketik untuk mencari…"}</div>
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
  .scan-row { display: flex; align-items: center; gap: 0.6rem; }
  .cell-num { width: 100%; text-align: right; padding: 0.25rem 0.4rem; }
  /* Switch Rp/% di header kolom Diskon — sama persis dengan Kasir POS. */
  .disc-head { display: inline-flex; align-items: center; gap: 0.35rem; }
  .disc-toggle {
    padding: 0.05rem 0.32rem;
    font-size: 0.7rem;
    font-weight: 700;
    line-height: 1.4;
    min-width: 1.9rem;
  }
  .disc-toggle-on { background: var(--primary); color: #fff; border-color: var(--primary); }
  .cl-disc-pct { display: flex; align-items: center; gap: 0.15rem; }
  .pct-sign { font-size: 0.78rem; }
  .cl-disc-hint { font-size: 0.7rem; text-align: right; margin-top: 0.1rem; }
  .fw-bold { font-weight: 700; }

  .edit-bottom { display: grid; grid-template-columns: 1fr 300px; gap: 1rem; }
  .edit-left { display: flex; flex-direction: column; gap: 0.3rem; }
  .pay-methods { display: grid; grid-template-columns: repeat(2, 1fr); gap: 0.3rem; }
  .pay-methods button.active { background: var(--primary); border-color: var(--primary); color: #fff; }

  .trow { display: flex; justify-content: space-between; padding: 0.18rem 0; }
  .trow.grand { font-size: 1.1rem; font-weight: 700; border-top: 1px solid var(--border); margin-top: 0.25rem; padding-top: 0.4rem; }

  .cust-selected { display: flex; align-items: center; justify-content: space-between; background: var(--baby-blue-soft); border-radius: var(--radius); padding: 0.4rem 0.6rem; }
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

  .popup-search { max-width: 480px; }
  .popup-results {
    margin-top: 0.6rem; background: var(--white); border: 1px solid var(--border);
    border-radius: var(--radius); max-height: 320px; overflow-y: auto;
  }
  .sr-row {
    display: grid; grid-template-columns: 1fr auto auto auto; gap: 0.6rem;
    align-items: center; width: 100%; text-align: left; border: none; border-radius: 0;
    border-bottom: 1px solid var(--border); padding: 0.45rem 0.8rem; font-size: 0.85rem;
  }
  .sr-row:last-child { border-bottom: none; }
  .sr-row.active { background: var(--baby-blue-soft); }
  .sr-name { font-weight: 600; }
  .sr-meta, .sr-stock { font-size: 0.78rem; }
  .sr-empty { padding: 0.7rem 0.8rem; font-size: 0.85rem; }
</style>
