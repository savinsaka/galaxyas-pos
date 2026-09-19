<script lang="ts">
  // Placeholder Fase 1. Fase 2: layar keranjang ala Kasir POS — scan barang,
  // pilih toko tujuan, "Kirim" -> catat pengiriman + kurangi stok gudang
  // (Item Keluar) + antre kirim ke server bridge toko tujuan.
  let { tabId }: { tabId?: string } = $props();
</script>

<div class="page-head"><h1>Kirim ke Toko</h1></div>
<div class="card" style="padding:1.2rem;">
  <p class="text-dim">
    🚚 Layar pengiriman sedang dibangun (Fase 2). Nanti bekerja seperti Kasir POS:
    scan barang → pilih toko tujuan → <b>Kirim</b>, dan stok gudang otomatis berkurang.
  </p>
</div>
