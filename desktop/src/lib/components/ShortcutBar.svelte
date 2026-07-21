<script lang="ts">
  import type { Snippet } from "svelte";

  export interface ShortcutBarItem {
    key: string;
    label: string;
    action: () => void;
    disabled?: boolean;
  }

  let { items, children }: { items: ShortcutBarItem[]; children?: Snippet } = $props();
</script>

<div class="shortcut-bar no-print">
  {#each items as it (it.key)}
    <button class="shortcut-item" disabled={it.disabled} onclick={it.action}><kbd>{it.key}</kbd> {it.label}</button>
  {/each}
  {@render children?.()}
</div>

<style>
  .shortcut-bar {
    flex-shrink: 0;
    display: flex; flex-wrap: wrap; gap: 0.6rem;
    margin-top: 0.6rem; padding: 0.3rem 0.2rem;
  }
  /* :global karena tombol tambahan lewat snippet `children` ditulis di komponen
     pemanggil (mis. badge "Lihat Pending" di KasirPOS) tapi harus ikut gaya ini. */
  :global(.shortcut-bar .shortcut-item) {
    display: flex; align-items: center; gap: 0.3rem;
    background: transparent; border: 1px solid transparent; border-radius: 6px;
    padding: 0.25rem 0.5rem; font-size: 0.78rem; color: var(--text-dim);
  }
  :global(.shortcut-bar .shortcut-item:hover:not(:disabled)) { background: var(--baby-blue-bg); border-color: var(--border); color: var(--text); }
  :global(.shortcut-bar .shortcut-item:disabled) { opacity: 0.45; cursor: default; }
  :global(.shortcut-bar kbd) {
    display: inline-block; min-width: 1.6rem; text-align: center;
    font-family: inherit; font-weight: 700; font-size: 0.72rem;
    background: var(--baby-blue-bg); border: 1px solid var(--border);
    border-radius: 4px; padding: 0.08rem 0.3rem;
  }
</style>
