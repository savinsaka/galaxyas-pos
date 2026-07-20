<script lang="ts">
  import BatchStockEntry from "$lib/components/BatchStockEntry.svelte";
  import PullFromMobile from "$lib/components/PullFromMobile.svelte";
  let { tabId }: { tabId?: string } = $props();
</script>

<PullFromMobile />
<BatchStockEntry kind="in" title="Tambah Item Masuk" {tabId} />
