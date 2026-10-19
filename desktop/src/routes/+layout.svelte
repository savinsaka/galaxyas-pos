<script lang="ts">
  import "../app.css";
  import { toasts } from "$lib/toast";
  import UpdateBanner from "$lib/components/UpdateBanner.svelte";
  import UpdateGate from "$lib/components/UpdateGate.svelte";
  import ShiftCloseGate from "$lib/components/ShiftCloseGate.svelte";
  import DialogHost from "$lib/components/DialogHost.svelte";
  let { children } = $props();
</script>

<UpdateBanner />
<ShiftCloseGate />

{@render children()}

<!-- Popup konfirmasi app (pengganti confirm() bawaan) — sengaja setelah
     children supaya selalu di atas popup halaman. -->
<DialogHost />

<!-- Wajib update: paling akhir supaya layar kuncinya menutupi semuanya. -->
<UpdateGate />

<div class="toast-wrap">
  {#each $toasts as t (t.id)}
    <div class="toast {t.kind}">{t.message}</div>
  {/each}
</div>
