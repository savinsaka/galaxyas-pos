<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR, formatQty, formatTime } from "$lib/format";
  import { showToast, toastError, errorMessage } from "$lib/toast";
  import { isRemoteClient } from "$lib/stores/activeServer";
  import { currentUser } from "$lib/stores/auth";
  import { debounce } from "$lib/debounce";
  import { createLiveClock } from "$lib/liveClock.svelte";
  import { todayIso, combineDateAndTime } from "$lib/dateTime";
  import { pendingSales, addPending, removePending } from "$lib/stores/pendingSales";
  import { markTransactionsDirty } from "$lib/stores/txSignal";
  import { activeTabId } from "$lib/stores/tabs";
  import { setTabDirty, clearTabDirty } from "$lib/stores/tabGuard";
  import { activeShiftStore } from "$lib/stores/shift";
  import { parseReceiptConfig, saleNeedsDrawer, type ReceiptConfig } from "$lib/receipt";
  import { buildDrawerKick, buildReceiptEscPos, withDrawerKick } from "$lib/escpos";
  import { formatMoneyInput, onMoneyInput } from "$lib/moneyInput";
  import ShortcutBar from "$lib/components/ShortcutBar.svelte";
  import type { Customer, DiscountPeriod, PaymentMethod, ProductWithStock, SaleInput, Shift, TransactionDetail } from "$lib/types";

  let { tabId }: { tabId?: string } = $props();

  const clock = createLiveClock();
  onDestroy(() => clock.stop());
  onDestroy(() => { if (tabId) clearTabDirty(tabId); });

  let tanggal = $state(todayIso());

  interface CartLine {
    product_id: string;
    name: string;
    /** Ditampilkan di samping nama supaya kasir bisa mencocokkan fisik barang. */
    barcode: string | null;
    price: number;
    qty: number;
    discount: number;
    brand: string | null;
    default_discount: number;
    periodic: boolean;
    manualOverride: boolean;
    /**
     * Persen yang diketik kasir untuk baris ini (null = diskonnya nominal).
     * Disimpan supaya diskon persen tetap "10%" walau Jumlah/Harga diubah —
     * `discount` sendiri selalu nominal, karena itu yang dikirim ke database.
     */
    manualPercent: number | null;
    stock_qty: number;
  }

  let discounts = $state<DiscountPeriod[]>([]);
  /**
   * Cara mengisi kolom Diskon: nominal rupiah atau persen dari harga baris.
   * Satu switch untuk SELURUH keranjang (di header kolom Diskon) — pilihan
   * pemilik toko. Yang berubah cuma cara mengetik; `discount` yang tersimpan
   * selalu nominal, jadi database & struk tidak terpengaruh.
   * Default persen — itu yang paling sering dipakai di toko.
   */
  let discountMode = $state<"rp" | "percent">("percent");
  /** Hanya admin yang boleh mengubah harga & diskon per baris. */
  const isAdmin = $derived($currentUser?.role === "admin");
  let search = $state("");
  let scanQty = $state(1);
  // Preferensi Kasir (Pengaturan → Preferensi Kasir): "scan_first" (default,
  // scan langsung masuk keranjang) atau "jumlah_first" (scan dulu, tunggu
  // isi Jumlah baru masuk keranjang).
  let scanMode = $state<"scan_first" | "jumlah_first">("scan_first");
  let pendingScanProduct = $state<ProductWithStock | null>(null);
  let searchBusy = $state(false);
  let stockAlert = $state<{ name: string; available: number } | null>(null);
  let warningModal = $state<string | null>(null);
  /** Judul modal peringatan; null = judul default "Belum Bisa Checkout". */
  let warningTitle = $state<string | null>(null);
  /**
   * Kunci idempoten untuk percobaan checkout yang sedang berjalan (lihat
   * `SaleInput.client_ref`). Bukan `$state` karena tidak dipakai di tampilan;
   * cukup bertahan selama keranjang ini belum selesai.
   */
  let checkoutRef: string | null = null;
  let showPendingList = $state(false);

  // Popup cari-nama: dipicu otomatis saat Enter tapi barcode tidak ditemukan.
  let showSearchPopup = $state(false);
  let popupQuery = $state("");
  let popupResults = $state<ProductWithStock[]>([]);
  let popupLoading = $state(false);
  let popupHighlight = $state(0);
  let cart = $state<CartLine[]>([]);
  let selectedCartId = $state<string | null>(null);
  let cartWrapEl = $state<HTMLDivElement>();
  let scanInputEl = $state<HTMLInputElement>();
  let scanQtyEl = $state<HTMLInputElement>();
  let paidInputEl = $state<HTMLInputElement>();
  let printNoBtnEl = $state<HTMLButtonElement>();
  let printYesBtnEl = $state<HTMLButtonElement>();
  let paymentMethod = $state<PaymentMethod>("Tunai");
  let paid = $state(0);
  // Field terpisah untuk metode "Kombinasi" (sebagian tunai, sebagian QRIS) — poin 7.
  let paidCash = $state(0);
  let paidQris = $state(0);
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

  const payments: PaymentMethod[] = ["Tunai", "QRIS", "Kombinasi", "Kartu"];

  /** Pilih metode bayar. QRIS otomatis isi "uang pas" (poin 8); Kombinasi
   * reset ke 2 field terpisah (Tunai/QRIS) yang dijumlah jadi `paid`. */
  function selectPaymentMethod(m: PaymentMethod) {
    paymentMethod = m;
    if (m === "QRIS") {
      paid = total;
    } else if (m === "Kombinasi") {
      paidCash = 0;
      paidQris = 0;
      paid = 0;
    }
  }
  function onPaidCashInput(e: Event) {
    onMoneyInput(e, (n) => { paidCash = n; paid = paidCash + paidQris; });
  }
  function onPaidQrisInput(e: Event) {
    onMoneyInput(e, (n) => { paidQris = n; paid = paidCash + paidQris; });
  }

  const totalQty = $derived(cart.reduce((s, l) => s + l.qty, 0));
  const subtotal = $derived(cart.reduce((s, l) => s + l.price * l.qty, 0));
  const totalDiscount = $derived(cart.reduce((s, l) => s + l.discount, 0));
  const total = $derived(Math.max(subtotal - totalDiscount, 0));
  const change = $derived(Math.max(paid - total, 0));

  $effect(() => {
    if (tabId) setTabDirty(tabId, cart.length > 0);
  });

  // Balik fokus ke scan-input begitu popup konfirmasi cetak struk ditutup
  // (klik Cetak, Tidak, atau backdrop) — siap untuk scan pelanggan berikutnya.
  let prevShowPrintConfirm = false;
  $effect(() => {
    if (prevShowPrintConfirm && !showPrintConfirm) scanInputEl?.focus();
    // Default fokus ke tombol Cetak (kanan) begitu popup muncul — Enter langsung
    // cetak, panah kiri/kanan pindah ke tombol Tidak kalau mau batal (poin 3).
    if (!prevShowPrintConfirm && showPrintConfirm) tick().then(() => printYesBtnEl?.focus());
    prevShowPrintConfirm = showPrintConfirm;
  });

  function onPrintConfirmKey(e: KeyboardEvent) {
    if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      e.preventDefault();
      (e.key === "ArrowLeft" ? printNoBtnEl : printYesBtnEl)?.focus();
    } else if (e.key === "Escape") {
      e.preventDefault();
      showPrintConfirm = false;
    }
  }

  async function loadSettings() {
    try {
      const s = await api.getSettings();
      receiptCfg = parseReceiptConfig(s);
      scanMode = s.kasir_scan_mode === "jumlah_first" ? "jumlah_first" : "scan_first";
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
      activeShiftStore.set(activeShift);
      // Default modal awal = uang fisik saat tutup shift SEBELUMNYA (poin 1) —
      // cuma prefill, tetap bisa diubah. Kalau belum ada shift terakhir sama
      // sekali (pertama kali), tetap 0.
      if (!activeShift) {
        const last = await api.listShifts(1);
        openingCash = last[0]?.closing_cash ?? 0;
      }
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
      activeShiftStore.set(activeShift);
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

  function addToCart(p: ProductWithStock, addQty = 1) {
    const ex = cart.find((l) => l.product_id === p.id);
    const currentQty = ex?.qty ?? 0;
    if (currentQty + addQty > p.stock_qty) {
      stockAlert = { name: p.name, available: p.stock_qty };
      return;
    }
    if (ex) {
      ex.qty += addQty;
      ex.stock_qty = p.stock_qty;
      if (!ex.manualOverride) applyDiscount(ex, ex.qty);
      cart = [...cart];
    } else {
      const line: CartLine = {
        product_id: p.id,
        name: p.name,
        barcode: p.barcode,
        price: p.sell_price,
        qty: addQty,
        discount: 0,
        brand: p.brand,
        default_discount: p.default_discount,
        periodic: false,
        manualOverride: false,
        manualPercent: null,
        stock_qty: p.stock_qty,
      };
      applyDiscount(line, addQty);
      cart = [...cart, line];
    }
  }

  function focusCartRow(id: string) {
    selectedCartId = id;
    cartWrapEl?.focus();
  }

  /** Field Jumlah ditaruh sebelum search bar (poin 2), tapi fokus tetap balik
   * ke search bar setelah Enter — alur: isi angka, Enter, langsung scan barcode. */
  function onQtyKeyBeforeScan(e: KeyboardEvent) {
    if (e.key === "Escape" && pendingScanProduct) {
      e.preventDefault();
      pendingScanProduct = null;
      scanQty = 1;
      scanInputEl?.focus();
      return;
    }
    if (e.key !== "Enter") return;
    e.preventDefault();
    if (scanMode === "jumlah_first" && pendingScanProduct) {
      addToCart(pendingScanProduct, scanQty);
      pendingScanProduct = null;
      scanQty = 1;
      search = "";
      scanInputEl?.focus();
      return;
    }
    scanInputEl?.focus();
  }

  /** Panah bawah dari search yang kosong: masuk ke mode pilih baris keranjang
   * (panah atas/bawah pindah baris, Del hapus item — lihat onCartKey). */
  function enterCartSelection() {
    if (cart.length === 0) return;
    focusCartRow(cart[0].product_id);
  }

  function onCartKey(e: KeyboardEvent) {
    const idx = cart.findIndex((l) => l.product_id === selectedCartId);
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (idx >= 0 && idx < cart.length - 1) selectedCartId = cart[idx + 1].product_id;
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (idx > 0) {
        selectedCartId = cart[idx - 1].product_id;
      } else {
        selectedCartId = null;
        scanInputEl?.focus();
      }
      return;
    }
    if (e.key === "Delete") {
      e.preventDefault();
      if (selectedCartId) removeLine(selectedCartId);
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      selectedCartId = null;
      scanInputEl?.focus();
    }
  }

  async function onSearchKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown" && !search.trim()) {
      e.preventDefault();
      enterCartSelection();
      return;
    }
    if (e.key !== "Enter") return;
    const term = search.trim();
    if (!term) {
      // Search kosong + Enter: lompat cepat ke pembayaran (sudah selesai scan).
      paidInputEl?.focus();
      paidInputEl?.select();
      return;
    }
    searchBusy = true;
    let p: ProductWithStock | null = null;
    try {
      p = await api.findByBarcode(term);
    } catch (e) {
      toastError(e);
    }
    // searchBusy dimatikan SEBELUM fokus balik ke search — input yang masih
    // disabled tidak bisa menerima focus(), itu sebabnya cursor tidak balik.
    searchBusy = false;
    await tick();
    if (p) {
      if (scanMode === "jumlah_first") {
        // Jangan langsung masuk keranjang — tunggu kasir isi Jumlah dulu (Enter
        // di field Jumlah yang benar-benar memasukkannya, lihat onQtyKeyBeforeScan).
        pendingScanProduct = p;
        search = "";
        await tick();
        scanQtyEl?.focus();
        scanQtyEl?.select();
      } else {
        addToCart(p, scanQty);
        search = "";
        scanQty = 1;
        scanInputEl?.focus();
      }
    } else {
      openSearchPopup(term);
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
      popupHighlight = 0;
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
  async function pickFromPopup(p: ProductWithStock) {
    addToCart(p, scanQty);
    showSearchPopup = false;
    popupQuery = "";
    popupResults = [];
    search = "";
    scanQty = 1;
    await tick();
    scanInputEl?.focus();
  }

  const paidDisplay = $derived(formatMoneyInput(paid));
  function onPaidInput(e: Event) {
    onMoneyInput(e, (n) => (paid = n));
  }
  function onPaidKey(e: KeyboardEvent) {
    if (e.key !== "Enter") return;
    e.preventDefault();
    if (!busy) doCheckout();
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
    // Diskon persen ikut jumlah: "10%" tetap 10% walau jumlahnya berubah.
    else if (line.manualPercent !== null) line.discount = discountFromPercent(line, line.manualPercent);
    else line.discount = Math.min(line.discount, line.price * line.qty);
    cart = [...cart];
  }
  function setPrice(line: CartLine, price: number) {
    line.price = Math.max(0, price);
    line.discount =
      line.manualPercent !== null
        ? discountFromPercent(line, line.manualPercent)
        : Math.min(line.discount, line.price * line.qty);
    line.manualOverride = true;
    cart = [...cart];
  }

  /**
   * Ganti cara mengisi diskon untuk seluruh keranjang. Nominal yang sudah ada
   * TIDAK diubah — di mode persen angkanya ditampilkan sebagai persen dari
   * harga baris. Fokus dikembalikan ke kolom scan supaya alur keyboard tidak
   * terputus setelah tombol ini diklik.
   */
  function toggleDiscountMode() {
    discountMode = discountMode === "percent" ? "rp" : "percent";
    scanInputEl?.focus();
  }

  const lineGross = (line: CartLine) => line.price * line.qty;

  /** Nominal dari persen, dibulatkan ke rupiah terdekat (bukan pecahan sen). */
  function discountFromPercent(line: CartLine, percent: number): number {
    const pct = Math.min(Math.max(0, percent), 100);
    return Math.round((lineGross(line) * pct) / 100);
  }

  /**
   * Angka yang tampil di kolom Diskon saat mode persen: yang diketik kasir bila
   * ada, kalau tidak diturunkan dari nominal (mis. diskon periodik/default),
   * dibulatkan 2 desimal supaya tidak jadi 9.999999999.
   */
  function linePercent(line: CartLine): number {
    if (line.manualPercent !== null) return line.manualPercent;
    const gross = lineGross(line);
    if (gross <= 0) return 0;
    return Math.round((line.discount / gross) * 10000) / 100;
  }

  /** Kasir mengetik di kolom Diskon — artinya tergantung switch Rp/% di header. */
  function setDiscount(line: CartLine, value: number) {
    if (discountMode === "percent") {
      const pct = Math.min(Math.max(0, value), 100);
      line.manualPercent = pct;
      line.discount = discountFromPercent(line, pct);
    } else {
      // Nominal yang diketik = angka tetap, tidak lagi mengikuti persen.
      line.manualPercent = null;
      line.discount = Math.min(Math.max(0, value), lineGross(line));
    }
    line.manualOverride = true;
    line.periodic = false;
    cart = [...cart];
  }
  const removeLine = (id: string) => (cart = cart.filter((l) => l.product_id !== id));
  function clearCart() {
    cart = [];
    paid = 0;
    paidCash = 0;
    paidQris = 0;
    checkoutRef = null;
  }

  function holdCurrentCart() {
    if (cart.length === 0) return;
    addPending({
      label: selectedCustomer?.name ?? `Pending ${$pendingSales.length + 1}`,
      cart: cart.map((l) => ({ ...l })),
      customerId: selectedCustomer?.id ?? null,
      paymentMethod,
      paid,
      paidCash,
      paidQris,
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
    checkoutRef = null; // keranjang baru = percobaan checkout baru
    selectedCustomer = p.customerId ? customers.find((c) => c.id === p.customerId) ?? null : null;
    customerSearch = "";
    paymentMethod = p.paymentMethod as PaymentMethod;
    paid = p.paid;
    paidCash = p.paidCash ?? 0;
    paidQris = p.paidQris ?? 0;
    removePending(id);
    showPendingList = false;
  }

  /** Judul di-set eksplisit tiap kali, supaya judul modal sebelumnya tidak nyangkut. */
  function showWarning(message: string, title: string | null = null) {
    warningTitle = title;
    warningModal = message;
  }

  function closeWarning() {
    warningModal = null;
    warningTitle = null;
  }

  async function doCheckout() {
    if (cart.length === 0) return showWarning("Keranjang kosong.");
    if (paid < total) return showWarning("Pembayaran kurang dari total.");
    if (!activeShift) return showWarning("Buka shift terlebih dahulu sebelum bertransaksi.");
    busy = true;
    // Kunci percobaan ini dibuat sekali dan dipakai lagi kalau kasir menekan
    // Bayar ulang setelah gagal — itu yang membuat percobaan ulang aman: Server
    // Pusat mengembalikan transaksi yang sudah tersimpan, bukan mencatat dobel.
    // Dikosongkan lagi oleh clearCart() begitu transaksi ini benar-benar selesai.
    checkoutRef ??= crypto.randomUUID();
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
        client_ref: checkoutRef,
        ...(paymentMethod === "Kombinasi" ? { paid_cash: paidCash, paid_qris: paidQris } : {}),
      };
      const tersimpan = total;
      const tx = await api.checkout(sale);
      lastReceipt = tx;
      showPrintConfirm = true;
      showToast(`Transaksi ${tx.invoice_no} tersimpan.`, "success");
      markTransactionsDirty();
      clearCart();
      selectedCustomer = null;
      customerSearch = "";
      tanggal = todayIso();
      paymentMethod = "Tunai";
      // Totalnya beda = yang dibalas Server Pusat adalah transaksi dari
      // percobaan SEBELUMNYA (ternyata sampai, cuma jawabannya hilang) yang
      // isinya sudah tidak sama dengan keranjang tadi. Kasir harus tahu, jangan
      // sampai dia mengira keranjang terakhir itu yang tercatat.
      if (Math.abs(tx.total - tersimpan) > 0.5) {
        showWarning(
          `Percobaan sebelumnya ternyata SUDAH tersimpan sebagai ${tx.invoice_no} ` +
            `(total ${formatIDR(tx.total)}). Supaya tidak tercatat dobel, keranjang tadi ` +
            `tidak disimpan lagi — periksa struk ini dan sesuaikan lewat Riwayat kalau perlu.`,
          "Transaksi Sudah Tersimpan",
        );
      }
    } catch (e) {
      // Kunci checkout SENGAJA tidak dikosongkan di sini: menekan Bayar lagi
      // memakai kunci yang sama, jadi kalau transaksi tadi sebenarnya sudah
      // tersimpan, percobaan kedua mengembalikannya alih-alih mencatat dobel.
      if ($isRemoteClient) {
        showWarning(
          `${errorMessage(e)}\n\nTransaksi mungkin sudah tersimpan di PC pusat, mungkin juga belum. ` +
            `Tekan "Bayar" lagi untuk memastikan — aman, tidak akan tercatat dobel.`,
          "Jawaban Tidak Sampai",
        );
      } else {
        toastError(e);
      }
    } finally {
      busy = false;
    }
  }

  async function doPrintReceipt() {
    if (!lastReceipt || !receiptCfg) return;
    try {
      // Perintah buka laci disatukan ke job cetak yang sama (bukan kirim
      // terpisah) supaya laci membuka begitu struk selesai keluar.
      const bytes = buildReceiptEscPos(lastReceipt, receiptCfg);
      await api.printEscposTo(
        receiptCfg.printer,
        saleNeedsDrawer(lastReceipt) ? withDrawerKick(bytes, receiptCfg.cashDrawer) : bytes,
      );
      showToast("Struk dikirim ke printer.", "success");
    } catch (e) {
      toastError(e);
    } finally {
      showPrintConfirm = false;
    }
  }

  /** Buka laci tanpa transaksi (tukar uang, ambil kembalian) — F8. */
  async function openDrawer() {
    if (!receiptCfg) return;
    if (receiptCfg.cashDrawer === "off") {
      return showToast("Laci kasir dimatikan di Pengaturan → Struk & Printer.", "info");
    }
    try {
      await api.printEscposTo(receiptCfg.printer, buildDrawerKick(receiptCfg.cashDrawer));
      showToast("Laci kasir dibuka.", "success");
    } catch (e) {
      toastError(e);
    }
  }

  // F1 fokus ke field Jumlah (Tunai sudah default, tidak perlu shortcut sendiri).
  // Rumpun F2-F4 metode pembayaran (selain Tunai), F5-F7 alur keranjang, F9 aksi
  // akhir (konsisten dengan Opname/Item Masuk-Keluar), F10-F12 rumpun uang cepat —
  // tiap rumpun dikelompokkan, tidak dipencar (poin 1).
  function onGlobalKey(e: KeyboardEvent) {
    if (tabId && $activeTabId !== tabId) return;
    if (!activeShift) return;
    if (e.key === "F1") {
      e.preventDefault();
      scanQtyEl?.focus();
      scanQtyEl?.select();
    } else if (e.key === "F2") {
      e.preventDefault();
      selectPaymentMethod("QRIS");
    } else if (e.key === "F3") {
      e.preventDefault();
      selectPaymentMethod("Kombinasi");
    } else if (e.key === "F4") {
      e.preventDefault();
      selectPaymentMethod("Kartu");
    } else if (e.key === "F5") {
      e.preventDefault();
      openSearchPopup(search.trim());
    } else if (e.key === "F6") {
      e.preventDefault();
      if (cart.length > 0) holdCurrentCart();
    } else if (e.key === "F7") {
      e.preventDefault();
      if (cart.length > 0) clearCart();
    } else if (e.key === "F8") {
      e.preventDefault();
      openDrawer();
    } else if (e.key === "F9") {
      e.preventDefault();
      if (!busy) doCheckout();
    } else if (e.key === "F10") {
      e.preventDefault();
      paid = total;
    } else if (e.key === "F11") {
      e.preventDefault();
      paid = 50000;
    } else if (e.key === "F12") {
      e.preventDefault();
      paid = 100000;
    }
  }
  onMount(() => window.addEventListener("keydown", onGlobalKey));
  onDestroy(() => window.removeEventListener("keydown", onGlobalKey));
</script>

<div class="pos-page">
<!-- Informasi header transaksi -->
<div class="pos-header">
  <div class="pos-meta-grid">
  <div class="pos-meta">
    <span class="meta-label">No. Struk</span>
    <span class="meta-val mono">{lastReceipt ? lastReceipt.invoice_no : "— (auto)"}</span>
  </div>
  <div class="pos-meta">
    <span class="meta-label">Kasir</span>
    <span class="meta-val">{$currentUser?.name ?? $currentUser?.username ?? "—"}</span>
  </div>
  <div class="pos-meta">
    <span class="meta-label">Tanggal</span>
    {#if isAdmin}
      <input class="mono tanggal-input" type="date" max={todayIso()} bind:value={tanggal} title="Admin bisa mundurkan tanggal untuk transaksi yang terlewat" />
    {:else}
      <span class="meta-val mono">{new Date(tanggal).toLocaleDateString("id-ID", { day:"2-digit", month:"short", year:"numeric" })}</span>
    {/if}
  </div>
  <div class="pos-meta">
    <span class="meta-label">Jam</span>
    <span class="meta-val mono">{formatTime(clock.now)}</span>
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
  </div>
  <div class="pos-total-box">
    <span class="mono total-num">{formatIDR(total)}</span>
    <div class="kembalian-row"><span>Kembalian</span><span class="mono">{formatIDR(change)}</span></div>
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
      <input
        class="scan-qty mono"
        type="number"
        min="1"
        style="order:{scanMode === 'scan_first' ? 1 : 2};"
        bind:this={scanQtyEl}
        bind:value={scanQty}
        onkeydown={onQtyKeyBeforeScan}
        title="Jumlah untuk scan berikutnya (F1)"
      />
      <input
        class="scan-input"
        style="order:{scanMode === 'scan_first' ? 2 : 1};"
        bind:this={scanInputEl}
        placeholder="Scan barcode / cari nama lalu Enter…"
        bind:value={search}
        onkeydown={onSearchKey}
        disabled={searchBusy}
      />
      <button class="btn-ghost" style="order:3;" title="Cari nama barang (F5)" onclick={() => openSearchPopup(search.trim())}>🔍</button>
      {#if cart.length > 0}
        <span class="item-count" style="order:4;">{cart.reduce((s, l) => s + l.qty, 0)} item</span>
      {/if}
    </div>
    {#if pendingScanProduct}
      <div class="pending-scan-hint">Jumlah untuk: <b>{pendingScanProduct.name}</b> — isi jumlah lalu Enter (Esc untuk batal)</div>
    {/if}

    <!-- Tabel keranjang -->
    <div class="cart-table-wrap" bind:this={cartWrapEl} tabindex="-1" role="grid" aria-label="Keranjang belanja" onkeydown={onCartKey}>
      <table class="cart-table">
        <thead>
          <tr>
            <th style="width:2rem">No</th>
            <th>Nama Barang</th>
            <th style="width:110px">Barcode</th>
            <th style="width:120px">Jumlah</th>
            <th style="width:90px" class="text-right">Harga</th>
            <th style={isAdmin ? "width:120px" : "width:90px"} class="text-right">
              {#if isAdmin}
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
              {:else}
                Diskon
              {/if}
            </th>
            <th style="width:90px" class="text-right">Total</th>
            <th style="width:2rem"></th>
          </tr>
        </thead>
        <tbody>
          {#each cart as line, i (line.product_id)}
            <tr
              onclick={() => (selectedCartId = line.product_id)}
              style={selectedCartId === line.product_id ? "background:var(--baby-blue-soft);" : ""}
            >
              <td class="mono text-dim">{i + 1}</td>
              <td>
                <div class="cl-name">
                  {line.name}
                  {#if line.discount > 0}<span class="disc-badge">Diskon</span>{/if}
                  {#if line.stock_qty < 3}<span class="stock-badge">tinggal {formatQty(line.stock_qty)}</span>{/if}
                </div>
              </td>
              <!-- Kolom sendiri supaya mudah dipindai mata saat mencocokkan
                   fisik barang; barcode panjang dipotong dengan ellipsis dan
                   nilai penuhnya tetap bisa dilihat lewat tooltip. -->
              <td class="mono cl-barcode" title={line.barcode ?? ""}>
                {line.barcode ?? "—"}
              </td>
              <td>
                <div class="cl-qty">
                  <button onclick={() => setQty(line, line.qty - 1)}>−</button>
                  <input class="qty-input mono" type="number" min="1" value={line.qty} oninput={(e) => setQty(line, +e.currentTarget.value)} />
                  <button onclick={() => setQty(line, line.qty + 1)}>+</button>
                </div>
              </td>
              {#if isAdmin}
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
                    <!-- Nominalnya tetap ditampilkan (itu yang masuk struk), tapi
                         hanya kalau ada diskon — supaya baris tanpa diskon tidak
                         ikut tinggi dan keranjang tetap banyak yang kelihatan. -->
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
              {:else}
                <td class="text-right mono">{formatIDR(line.price)}</td>
                <td class="text-right mono">{line.discount > 0 ? "−" + formatIDR(line.discount) : "—"}</td>
              {/if}
              <td class="text-right mono fw-bold">{formatIDR(line.price * line.qty - line.discount)}</td>
              <td><button class="btn-ghost cl-del" onclick={() => removeLine(line.product_id)}>✕</button></td>
            </tr>
          {:else}
            <tr><td colspan="8" class="text-dim" style="text-align:center; padding:1.5rem 0;">Belum ada item — scan barcode atau cari nama barang.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>

  <!-- Kanan: panel pembayaran (tetap) -->
  <section class="pay-panel card">
    <div class="totals">
      <div class="trow"><span>Jumlah Barang</span><span class="mono">{formatQty(totalQty)}</span></div>
      <div class="trow"><span>Subtotal</span><span class="mono">{formatIDR(subtotal)}</span></div>
      <div class="trow"><span>Diskon</span><span class="mono">−{formatIDR(totalDiscount)}</span></div>
    </div>
    <div class="pay">
      <label>Metode Pembayaran</label>
      <div class="pay-methods">
        {#each payments as m}<button class:active={paymentMethod === m} onclick={() => selectPaymentMethod(m)}>{m}</button>{/each}
      </div>
      {#if paymentMethod === "Kombinasi"}
        <label>QRIS</label>
        <input class="mono" type="text" inputmode="numeric" value={formatMoneyInput(paidQris)} oninput={onPaidQrisInput} />
        <label>Tunai</label>
        <input class="mono" type="text" inputmode="numeric" bind:this={paidInputEl} value={formatMoneyInput(paidCash)} oninput={onPaidCashInput} onkeydown={onPaidKey} />
      {:else}
        <label>Bayar</label>
        <input class="mono" type="text" inputmode="numeric" bind:this={paidInputEl} value={paidDisplay} oninput={onPaidInput} onkeydown={onPaidKey} />
        <div class="quick">
          <button onclick={() => (paid = total)}>Uang Pas</button>
          <button onclick={() => (paid = 50000)}>50rb</button>
          <button onclick={() => (paid = 100000)}>100rb</button>
        </div>
      {/if}
      <button class="btn-primary checkout" disabled={busy || cart.length === 0} onclick={doCheckout} title="Bayar & Simpan (F9)">Bayar &amp; Simpan (F9)</button>
    </div>
  </section>
</div>

<ShortcutBar items={[
  { key: "F1", label: "Fokus Jumlah", action: () => { scanQtyEl?.focus(); scanQtyEl?.select(); } },
  { key: "F2", label: "Pilih QRIS", action: () => selectPaymentMethod("QRIS") },
  { key: "F3", label: "Pilih Kombinasi", action: () => selectPaymentMethod("Kombinasi") },
  { key: "F4", label: "Pilih Kartu", action: () => selectPaymentMethod("Kartu") },
  { key: "F5", label: "Cari Barang", action: () => openSearchPopup(search.trim()) },
  { key: "F6", label: "Pending", action: holdCurrentCart, disabled: cart.length === 0 },
  { key: "F7", label: "Kosongkan", action: clearCart, disabled: cart.length === 0 },
  { key: "F8", label: "Buka Laci", action: openDrawer, disabled: receiptCfg?.cashDrawer === "off" },
  { key: "F9", label: "Bayar & Simpan", action: doCheckout, disabled: busy || cart.length === 0 },
  { key: "F10", label: "Uang Pas", action: () => (paid = total) },
  { key: "F11", label: "50rb", action: () => (paid = 50000) },
  { key: "F12", label: "100rb", action: () => (paid = 100000) },
]}>
  {#snippet children()}
    {#if $pendingSales.length > 0}
      <button class="shortcut-item" onclick={() => (showPendingList = true)}>📋 Lihat Pending ({$pendingSales.length})</button>
    {/if}
  {/snippet}
</ShortcutBar>
{/if}
</div>

{#if showPrintConfirm && lastReceipt}
  <div class="modal-backdrop" onclick={() => (showPrintConfirm = false)} role="presentation">
    <div class="modal print-confirm" onclick={(e) => e.stopPropagation()} onkeydown={onPrintConfirmKey} role="presentation">
      <div class="stock-alert-icon">✅</div>
      <h2>Transaksi Tersimpan</h2>
      <p class="text-dim mono" style="margin:0.3rem 0 1rem;">{lastReceipt.invoice_no} · {formatIDR(lastReceipt.total)}</p>
      <div class="row" style="gap:0.5rem;">
        <button class="btn-ghost" style="flex:1;" bind:this={printNoBtnEl} onclick={() => (showPrintConfirm = false)}>Tidak</button>
        <button class="btn-primary" style="flex:1;" bind:this={printYesBtnEl} onclick={doPrintReceipt}>🖨️ Cetak Struk</button>
      </div>
    </div>
  </div>
{/if}

{#if warningModal}
  <div class="modal-backdrop" onclick={closeWarning} role="presentation">
    <div class="modal stock-alert" onclick={(e) => e.stopPropagation()} role="presentation">
      <div class="stock-alert-icon">⚠️</div>
      <h2>{warningTitle ?? "Belum Bisa Checkout"}</h2>
      <p class="text-dim" style="margin:0.3rem 0 1rem; white-space:pre-line;">{warningModal}</p>
      <button class="btn-primary" style="width:100%;" onclick={closeWarning}>Tutup</button>
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
        onkeydown={onPopupKey}
        autofocus
      />
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
    align-items: stretch;
    gap: 1rem;
    background: var(--white);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.5rem 0.8rem;
    margin-bottom: 0.5rem;
    flex-shrink: 0;
  }
  /* No.Struk+Kasir, Tanggal+Jam berpasangan; Pelanggan penuh 2 kolom (poin 4-6) */
  .pos-meta-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.25rem 1.2rem;
    flex: 1;
    min-width: 0;
    align-content: center;
  }
  .pos-meta { display:flex; flex-direction:column; gap:0.02rem; }
  .meta-label { font-size:0.62rem; color:var(--text-dim); text-transform:uppercase; letter-spacing:0.05em; }
  .meta-val { font-size:0.8rem; font-weight:600; }
  .tanggal-input { width:135px; padding:0.2rem 0.4rem; font-size:0.78rem; }
  .pos-customer { grid-column: 1 / 3; min-width:0; }
  .pos-customer input { width:100%; max-width:320px; padding:0.25rem 0.4rem; font-size:0.78rem; }

  /* Total besar + Kembalian kecil di kanan header, gantikan posisi lama di
     panel bayar (poin A2/A5) — sesuai mockup yang dikirim. */
  .pos-total-box {
    flex-shrink: 0;
    min-width: 260px;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    justify-content: center;
    gap: 0.25rem;
    background: var(--baby-blue-bg);
    border-radius: var(--radius);
    padding: 0.4rem 1rem;
  }
  .total-num { font-size: 2.7rem; font-weight: 800; line-height: 1; text-align: right; }
  .kembalian-row {
    display: flex; justify-content: space-between; align-items: baseline;
    font-size: 0.82rem; color: var(--text-dim);
    border-top: 1px solid var(--border); padding-top: 0.25rem;
  }
  .kembalian-row .mono { font-weight: 700; color: var(--text); font-size: 0.95rem; }

  /* Layout utama: sisa tinggi setelah header, hanya cart-table-wrap yang scroll */
  .pos { display:grid; grid-template-columns:1fr 400px; gap:0.9rem; flex:1; min-height:0; }

  /* Panel kiri */
  .main-panel { display:flex; flex-direction:column; gap:0.5rem; min-height:0; overflow:hidden; }

  /* Bar shortcut keyboard di bawah — komponen ShortcutBar.svelte reusable. */

  /* Popup konfirmasi cetak struk setelah transaksi tersimpan */
  .print-confirm { max-width: 340px; text-align: center; }

  /* Baris scan */
  .scan-row { display:flex; align-items:center; gap:0.6rem; }
  .scan-input { flex:1; font-size:1.25rem; padding:0.85rem 1rem; }
  .scan-qty { width:64px; text-align:center; font-size:1.1rem; padding:0.85rem 0.3rem; flex-shrink:0; }
  .item-count {
    white-space:nowrap; font-size:0.8rem; font-weight:700;
    background:var(--primary); color:#fff;
    padding:0.25rem 0.7rem; border-radius:999px;
  }
  .pending-scan-hint {
    font-size:0.8rem; color:var(--primary-dark); font-weight:600;
    background:var(--baby-blue-bg); border:1px solid var(--border);
    border-radius:var(--radius); padding:0.35rem 0.7rem; margin-top:0.4rem;
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
  .sr-row.active { background: var(--baby-blue-soft); }
  .sr-name { font-weight: 600; }
  .sr-meta, .sr-stock { font-size: 0.78rem; }
  .sr-empty { padding: 0.7rem 0.8rem; font-size: 0.85rem; }

  /* Tabel keranjang — sengaja dipangkas ~5 baris kelihatan (scroll untuk sisanya),
     supaya header di atas (No.Struk/Kasir/Tanggal/Jam/Pelanggan + Total besar)
     bisa lebih lega (poin 3). */
  .cart-table-wrap { max-height:260px; overflow-y:auto; border:1px solid var(--border); border-radius:var(--radius); background:var(--white); }
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
  /* Kolom barcode: sengaja kecil & redup — untuk mencocokkan fisik barang,
     bukan informasi utama. Tidak boleh membungkus supaya tinggi baris tetap. */
  .cl-barcode {
    font-size:0.78rem;
    color:var(--text-dim);
    white-space:nowrap;
    overflow:hidden;
    text-overflow:ellipsis;
    max-width:110px;
  }
  .disc-head { display:inline-flex; align-items:center; gap:0.35rem; }
  .disc-toggle {
    padding:0.05rem 0.32rem;
    font-size:0.7rem;
    font-weight:700;
    line-height:1.4;
    min-width:1.9rem;
  }
  .disc-toggle-on { background:var(--primary); color:#fff; border-color:var(--primary); }
  .cl-disc-pct { display:flex; align-items:center; gap:0.15rem; }
  .pct-sign { font-size:0.78rem; }
  .cl-disc-hint { font-size:0.7rem; text-align:right; margin-top:0.1rem; }
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
