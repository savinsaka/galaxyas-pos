<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR, formatQty } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import type { PaymentMethod, ProductWithStock, SaleInput, TransactionDetail } from "$lib/types";
  import Receipt from "$lib/components/Receipt.svelte";

  interface CartLine {
    product_id: string;
    name: string;
    price: number;
    qty: number;
    discount: number;
  }

  let products = $state<ProductWithStock[]>([]);
  let search = $state("");
  let cart = $state<CartLine[]>([]);
  let paymentMethod = $state<PaymentMethod>("Tunai");
  let paid = $state(0);
  let storeName = $state("GALAXYAS POS");
  let receiptFooter = $state("");
  let lastReceipt = $state<TransactionDetail | null>(null);
  let busy = $state(false);

  const payments: PaymentMethod[] = ["Tunai", "QRIS", "Transfer", "Kartu"];

  const subtotal = $derived(cart.reduce((s, l) => s + l.price * l.qty, 0));
  const totalDiscount = $derived(cart.reduce((s, l) => s + l.discount, 0));
  const total = $derived(Math.max(subtotal - totalDiscount, 0));
  const change = $derived(Math.max(paid - total, 0));
  const filtered = $derived(
    products.filter(
      (p) =>
        p.name.toLowerCase().includes(search.toLowerCase()) ||
        (p.barcode ?? "").toLowerCase().includes(search.toLowerCase()),
    ),
  );

  async function loadProducts() {
    try {
      products = await api.listProducts("", false);
    } catch (e) {
      toastError(e);
    }
  }
  async function loadSettings() {
    try {
      const s = await api.getSettings();
      storeName = s.store_name || storeName;
      receiptFooter = s.receipt_footer || "";
    } catch (e) {
      toastError(e);
    }
  }
  onMount(() => {
    loadProducts();
    loadSettings();
  });

  function addToCart(p: ProductWithStock) {
    const ex = cart.find((l) => l.product_id === p.id);
    if (ex) {
      ex.qty += 1;
      ex.discount = ex.qty * p.default_discount;
      cart = [...cart];
    } else {
      cart = [...cart, { product_id: p.id, name: p.name, price: p.sell_price, qty: 1, discount: p.default_discount }];
    }
  }

  async function onSearchKey(e: KeyboardEvent) {
    if (e.key !== "Enter") return;
    const term = search.trim();
    if (!term) return;
    try {
      const p = await api.findByBarcode(term);
      if (p) {
        addToCart(p);
        search = "";
      } else if (filtered.length === 1) {
        addToCart(filtered[0]);
        search = "";
      }
    } catch (e) {
      toastError(e);
    }
  }

  function setQty(line: CartLine, qty: number) {
    line.qty = Math.max(1, qty);
    cart = [...cart];
  }
  const removeLine = (id: string) => (cart = cart.filter((l) => l.product_id !== id));
  function clearCart() {
    cart = [];
    paid = 0;
  }

  async function doCheckout() {
    if (cart.length === 0) return showToast("Keranjang kosong.", "info");
    if (paid < total) return showToast("Pembayaran kurang dari total.", "error");
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
      };
      const tx = await api.checkout(sale);
      lastReceipt = tx;
      showToast(`Transaksi ${tx.invoice_no} tersimpan.`, "success");
      clearCart();
      await loadProducts();
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="pos">
  <section class="picker card">
    <input placeholder="Scan barcode / cari barang lalu Enter…" bind:value={search} onkeydown={onSearchKey} />
    <div class="product-grid">
      {#each filtered as p (p.id)}
        <button class="product-tile" onclick={() => addToCart(p)}>
          <div class="pt-name">{p.name}</div>
          <div class="pt-meta"><span class="mono">{formatIDR(p.sell_price)}</span><span class="text-dim">stok {formatQty(p.stock_qty)}</span></div>
        </button>
      {:else}
        <p class="text-dim">Tidak ada barang aktif.</p>
      {/each}
    </div>
  </section>

  <section class="cart card">
    <h2>Keranjang</h2>
    <div class="cart-lines">
      {#each cart as line (line.product_id)}
        <div class="cart-line">
          <div class="cl-main"><div>{line.name}</div><div class="text-dim mono">{formatIDR(line.price)}</div></div>
          <div class="cl-qty">
            <button onclick={() => setQty(line, line.qty - 1)}>−</button>
            <input class="qty-input mono" type="number" min="1" value={line.qty} oninput={(e) => setQty(line, +e.currentTarget.value)} />
            <button onclick={() => setQty(line, line.qty + 1)}>+</button>
          </div>
          <div class="cl-total mono">{formatIDR(line.price * line.qty - line.discount)}</div>
          <button class="btn-ghost cl-del" onclick={() => removeLine(line.product_id)}>✕</button>
        </div>
      {:else}
        <p class="text-dim">Belum ada item.</p>
      {/each}
    </div>

    <div class="totals">
      <div class="trow"><span>Subtotal</span><span class="mono">{formatIDR(subtotal)}</span></div>
      <div class="trow"><span>Diskon</span><span class="mono">−{formatIDR(totalDiscount)}</span></div>
      <div class="trow grand"><span>Total</span><span class="mono">{formatIDR(total)}</span></div>
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
      <div class="trow"><span>Kembalian</span><span class="mono">{formatIDR(change)}</span></div>
      <button class="btn-primary checkout" disabled={busy || cart.length === 0} onclick={doCheckout}>Bayar &amp; Simpan</button>
    </div>
  </section>
</div>

{#if lastReceipt}
  <Receipt detail={lastReceipt} {storeName} footer={receiptFooter} onClose={() => (lastReceipt = null)} />
{/if}

<style>
  .pos { display:grid; grid-template-columns:1fr 360px; gap:1rem; height:calc(100vh - 230px); }
  .picker { display:flex; flex-direction:column; gap:0.8rem; overflow:hidden; }
  .product-grid { display:grid; grid-template-columns:repeat(auto-fill,minmax(140px,1fr)); gap:0.55rem; overflow-y:auto; align-content:start; }
  .product-tile { text-align:left; padding:0.65rem; display:flex; flex-direction:column; gap:0.35rem; min-height:68px; }
  .pt-name { font-weight:600; line-height:1.2; }
  .pt-meta { display:flex; justify-content:space-between; font-size:0.8rem; }
  .cart { display:flex; flex-direction:column; overflow:hidden; }
  .cart-lines { flex:1; overflow-y:auto; margin-bottom:0.5rem; }
  .cart-line { display:grid; grid-template-columns:1fr auto auto auto; align-items:center; gap:0.5rem; padding:0.45rem 0; border-bottom:1px solid var(--border); }
  .cl-qty { display:flex; align-items:center; gap:0.25rem; }
  .cl-qty button { padding:0.2rem 0.45rem; }
  .qty-input { width:46px; text-align:center; padding:0.25rem; }
  .cl-total { min-width:80px; text-align:right; font-weight:600; }
  .cl-del { padding:0.2rem 0.4rem; color:var(--danger); }
  .totals { border-top:1px solid var(--border); padding-top:0.5rem; }
  .trow { display:flex; justify-content:space-between; padding:0.18rem 0; }
  .trow.grand { font-size:1.1rem; font-weight:700; border-top:1px solid var(--border); margin-top:0.25rem; padding-top:0.45rem; }
  .pay-methods { display:grid; grid-template-columns:repeat(4,1fr); gap:0.3rem; margin-bottom:0.6rem; }
  .pay-methods button.active { background:var(--primary); border-color:var(--primary); color:#fff; }
  .quick { display:grid; grid-template-columns:repeat(3,1fr); gap:0.3rem; margin:0.45rem 0; }
  .checkout { width:100%; padding:0.75rem; font-size:1rem; margin-top:0.45rem; }
</style>
