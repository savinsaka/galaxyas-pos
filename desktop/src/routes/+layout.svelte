<script lang="ts">
  import "../app.css";
  import { toasts } from "$lib/toast";
  let { children } = $props();
</script>

{@render children()}

<div class="toast-wrap">
  {#each $toasts as t (t.id)}
    <div class="toast {t.kind}">{t.message}</div>
  {/each}
</div>
