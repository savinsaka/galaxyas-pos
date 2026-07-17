<script lang="ts">
  import { get } from "svelte/store";
  import { api } from "$lib/api";
  import { formatIDR, formatQty } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { debounce } from "$lib/debounce";
  import { activeTabId, closeTab } from "$lib/stores/tabs";
  import { markTransactionsDirty } from "$lib/stores/txSignal";
  import type { Customer, PaymentMethod, ProductWithStock, SaleInput, TransactionDetail } from "$lib/types";

  let { transactionId }: { transactionId: string } = $props();

  interface EditLine {
    product_id: string;
    name: string;
    price: number;
    qty: number;
    discount: number;
  }

  let detail = $state<TransactionDetail | null>(null);
  let loading = $state(true);
  let lines = $state<EditLine[]>([]);
  let paymentMethod = $state<PaymentMethod>("Tunai");
  let paid = $state(0);
  let busy = $state(false);

  let search = $state("");
  let showSearchPopup = $state(false);
  let popupResults = $state<ProductWithStock[]>([]);
  let popupLoading = $state(false);

  let customers = $state<Customer[]>([]);
  let customerSearch = $state("");
  let selectedCustomer = $state<Customer | null>(null);

  const payments: PaymentMethod[] = ["Tunai", "QRIS", "Transfer", "Kartu"];

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
      }));
      paymentMethod = (d.payment_method as PaymentMethod) ?? "Tunai";
      paid = d.paid;
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
      lines = [...lines, { product_id: p.id, name: p.name, price: p.sell_price, qty: 1, discount: 0 }];
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

  function setQty(line: EditLine, qty: number) {
    line.qty = Math.max(0.01, qty);
    line.discount = Math.min(line.discount, line.price * line.qty);
    lines = [...lines];
  }
  function setPrice(line: EditLine, price: number) {
    line.price = Math.max(0, price);
    line.discount = Math.min(line.discount, line.price * line.qty);
    lines = [...lines];
  }
  function setDiscount(line: EditLine, discount: number) {
    line.discount = Math.min(Math.max(0, discount), line.price * line.qty);
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
          <th style="width:130px" class="text-right">Diskon</th>
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
              <input
                class="mono cell-num"
                type="number"
                min="0"
                value={line.discount}
                oninput={(e) => setDiscount(line, +e.currentTarget.value)}
              />
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
      <label style="margin-top:0.6rem;">Bayar</label>
      <input class="mono" type="number" min="0" bind:value={paid} />
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
      <input class="mono" placeholder="Ketik nama atau barcode…" bind:value={search} oninput={onPopupInput} autofocus />
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
  .sr-name { font-weight: 600; }
  .sr-meta, .sr-stock { font-size: 0.78rem; }
  .sr-empty { padding: 0.7rem 0.8rem; font-size: 0.85rem; }
</style>
