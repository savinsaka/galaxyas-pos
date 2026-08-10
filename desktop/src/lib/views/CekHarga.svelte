<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR } from "$lib/format";
  import { toastError } from "$lib/toast";
  import { activeTabId } from "$lib/stores/tabs";
  import ProductSearchPopup from "$lib/components/ProductSearchPopup.svelte";
  import type { DiscountPeriod, ProductWithStock } from "$lib/types";

  /**
   * Layar cek harga untuk pegawai/pelanggan: barang masuk lewat scan/cari, harga
   * & diskon keluar. Sengaja tidak menampilkan stok, harga modal, atau tombol
   * apa pun — modul ini dipakai oleh akun yang cuma boleh melihat harga.
   */

  let { tabId }: { tabId?: string } = $props();

  interface Hasil {
    name: string;
    barcode: string | null;
    brand: string | null;
    price: number;
    discount: number;
    /** true kalau diskonnya dari Diskon Periodik (bukan diskon default barang). */
    periodic: boolean;
  }

  let search = $state("");
  let searchBusy = $state(false);
  let hasil = $state<Hasil | null>(null);
  let discounts = $state<DiscountPeriod[]>([]);
  let searchInputEl = $state<HTMLInputElement | null>(null);
  let showSearchPopup = $state(false);
  let popupQuery = $state("");

  onMount(() => {
    loadDiscounts();
    searchInputEl?.focus();
  });

  async function loadDiscounts() {
    try {
      discounts = await api.listDiscounts();
    } catch (e) {
      toastError(e);
    }
  }

  // JS getDay(): 0=Min..6=Sab → kunci hari di UI Diskon Periodik.
  const DAY_KEYS = ["min", "sen", "sel", "rab", "kam", "jum", "sab"];

  function dayMatches(days: string, key: string): boolean {
    if (days === "everyday") return true;
    return days.split(",").map((d) => d.trim()).includes(key);
  }

  function discountValue(d: DiscountPeriod, price: number): number {
    return d.discount_type === "percent" ? (price * d.value) / 100 : d.value;
  }

  /**
   * Aturannya harus sama persis dengan kasir (lihat applyDiscount di
   * KasirPOS.svelte): diskon periodik menang atas diskon default, item menang
   * atas merek, dan kalau ada beberapa yang cocok diambil nominal terbesar.
   * Dihitung untuk 1 buah.
   */
  function hitungDiskon(p: ProductWithStock): { discount: number; periodic: boolean } {
    const key = DAY_KEYS[new Date().getDay()];
    const matches = discounts.filter(
      (d) =>
        d.is_active &&
        dayMatches(d.days, key) &&
        ((d.scope === "item" && d.target === p.id) || (d.scope === "brand" && d.target === p.brand)),
    );
    let discount: number;
    let periodic: boolean;
    if (matches.length) {
      const items = matches.filter((d) => d.scope === "item");
      const pool = items.length ? items : matches;
      discount = Math.max(...pool.map((d) => discountValue(d, p.sell_price)));
      periodic = true;
    } else {
      discount = p.default_discount;
      periodic = false;
    }
    return { discount: Math.min(Math.max(discount, 0), p.sell_price), periodic };
  }

  function tampilkan(p: ProductWithStock) {
    const { discount, periodic } = hitungDiskon(p);
    hasil = {
      name: p.name,
      barcode: p.barcode,
      brand: p.brand,
      price: p.sell_price,
      discount,
      periodic,
    };
  }

  /**
   * Alurnya sama seperti kolom scan di kasir: Enter mencari barcode persis dulu,
   * kalau tidak ketemu popup pencarian nama terbuka dengan kata yang sudah
   * diketik. Jadi barcode rusak/tidak terbaca tetap bisa dicari manual.
   */
  async function onSearchKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      search = "";
      hasil = null;
      return;
    }
    if (e.key !== "Enter") return;
    e.preventDefault();
    const term = search.trim();
    if (!term) return;
    searchBusy = true;
    let p: ProductWithStock | null = null;
    try {
      p = await api.findByBarcode(term);
    } catch (err) {
      toastError(err);
    }
    // searchBusy dimatikan SEBELUM fokus balik ke search — input yang masih
    // disabled tidak bisa menerima focus().
    searchBusy = false;
    await tick();
    if (p) {
      tampilkan(p);
      search = "";
      searchInputEl?.focus();
    } else {
      openSearchPopup(term);
    }
  }

  function openSearchPopup(term: string) {
    popupQuery = term;
    showSearchPopup = true;
  }

  async function closeSearchPopup() {
    showSearchPopup = false;
    await tick();
    searchInputEl?.focus();
  }

  async function pickFromPopup(p: ProductWithStock) {
    tampilkan(p);
    showSearchPopup = false;
    search = "";
    await tick();
    searchInputEl?.focus();
  }

  // F5 = cari nama barang, sama seperti di kasir.
  function onGlobalKey(e: KeyboardEvent) {
    if (tabId && $activeTabId !== tabId) return;
    if (e.key === "F5") {
      e.preventDefault();
      openSearchPopup(search.trim());
    }
  }
  onMount(() => window.addEventListener("keydown", onGlobalKey));
  onDestroy(() => window.removeEventListener("keydown", onGlobalKey));

  const netto = $derived(hasil ? Math.max(hasil.price - hasil.discount, 0) : 0);
  const discPercent = $derived(
    hasil && hasil.price > 0 ? Math.round((hasil.discount / hasil.price) * 1000) / 10 : 0,
  );
</script>

<div class="page-head"><h1>Cek Harga</h1></div>

<div class="cek-wrap">
  <div class="card">
    <div class="scan-row">
      <input
        class="scan-input"
        bind:this={searchInputEl}
        placeholder="Scan barcode / cari nama lalu Enter…"
        bind:value={search}
        onkeydown={onSearchKey}
        disabled={searchBusy}
      />
      <button class="btn-ghost" title="Cari nama barang (F5)" onclick={() => openSearchPopup(search.trim())}>🔍</button>
    </div>
    <p class="text-dim" style="margin:0.5rem 0 0; font-size:0.8rem;">
      Enter untuk cek · Esc untuk mengosongkan · F5 cari nama barang
    </p>
  </div>

  {#if hasil}
    <div class="card cek-hasil">
      <div class="cek-nama">{hasil.name}</div>
      <div class="text-dim mono" style="font-size:0.82rem;">
        {hasil.barcode ?? "-"}{hasil.brand ? ` · ${hasil.brand}` : ""}
      </div>

      <div class="cek-baris">
        <span>Harga</span>
        <span class="mono" class:coret={hasil.discount > 0}>{formatIDR(hasil.price)}</span>
      </div>
      <div class="cek-baris">
        <span>Diskon {hasil.periodic ? "(periodik)" : ""}</span>
        <span class="mono cek-disk">
          {hasil.discount > 0 ? `−${formatIDR(hasil.discount)} (${discPercent}%)` : "—"}
        </span>
      </div>

      <div class="cek-netto">
        <span>Harga Bayar</span>
        <span class="mono">{formatIDR(netto)}</span>
      </div>
    </div>
  {/if}
</div>

{#if showSearchPopup}
  <!-- showStock=false: akun cek-harga tidak boleh melihat stok. -->
  <ProductSearchPopup initialQuery={popupQuery} showStock={false} onClose={closeSearchPopup} onPick={pickFromPopup} />
{/if}

<style>
  .cek-wrap {
    max-width: 620px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .scan-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .scan-input {
    flex: 1;
    font-size: 1.25rem;
    padding: 0.85rem 1rem;
  }
  .cek-nama {
    font-size: 1.35rem;
    font-weight: 700;
    line-height: 1.25;
  }
  .cek-baris {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 1rem;
    margin-top: 0.8rem;
    font-size: 1.05rem;
  }
  .coret {
    text-decoration: line-through;
    opacity: 0.65;
  }
  .cek-disk {
    color: var(--danger);
  }
  .cek-netto {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 1rem;
    margin-top: 1rem;
    padding-top: 0.8rem;
    border-top: 1px solid var(--border);
    font-size: 1.15rem;
    font-weight: 700;
  }
  .cek-netto .mono {
    font-size: 2rem;
  }
</style>
