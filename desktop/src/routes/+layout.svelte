<script lang="ts">
  import "../app.css";
  import { toasts } from "$lib/toast";
  import UpdateBanner from "$lib/components/UpdateBanner.svelte";
  import ShiftCloseGate from "$lib/components/ShiftCloseGate.svelte";
  let { children } = $props();
</script>

<UpdateBanner />
<ShiftCloseGate />

{@render children()}

<div class="toast-wrap">
  {#each $toasts as t (t.id)}
    <div class="toast {t.kind}">{t.message}</div>
  {/each}
</div>
