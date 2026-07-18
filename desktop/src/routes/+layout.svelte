<script lang="ts">
  import "../app.css";
  import { toasts } from "$lib/toast";
  import UpdateBanner from "$lib/components/UpdateBanner.svelte";
  let { children } = $props();
</script>

<UpdateBanner />

{@render children()}

<div class="toast-wrap">
  {#each $toasts as t (t.id)}
    <div class="toast {t.kind}">{t.message}</div>
  {/each}
</div>
