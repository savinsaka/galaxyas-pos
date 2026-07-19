<script lang="ts">
  import ProductForm from "$lib/components/ProductForm.svelte";
  let { tabId }: { tabId?: string } = $props();
</script>

<div class="page-head"><h1>Tambah Barang</h1></div>
<div class="card" style="max-width:720px;">
  <p class="text-dim" style="margin-top:0;">
    Isi data barang baru. Form akan dikosongkan otomatis setelah disimpan agar bisa input berturut-turut.
  </p>
  <ProductForm submitLabel="Simpan Barang" resetAfterSave={true} {tabId} />
</div>
