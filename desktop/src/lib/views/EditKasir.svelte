<script lang="ts">
  // Edit Kasir = layar Kasir yang sama persis, hanya dalam mode edit: data
  // struk (No. Struk, Kasir, Tanggal, Jam) dikunci ke transaksi asli dan
  // tombol bayar menyimpan perubahan ke transaksi itu. Semua logikanya ada di
  // KasirPOS.svelte supaya dua layar ini tidak pernah beda tampilan lagi.
  import KasirPOS from "./KasirPOS.svelte";

  let { transactionId, tabId }: { transactionId: string; tabId?: string } = $props();
</script>

<KasirPOS {tabId} editTransactionId={transactionId} />
