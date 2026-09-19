<script lang="ts">
  import BatchStockEntry from "$lib/components/BatchStockEntry.svelte";
  let { tabId }: { tabId?: string } = $props();
</script>

<BatchStockEntry kind="out" title="Tambah Item Keluar" {tabId} />
