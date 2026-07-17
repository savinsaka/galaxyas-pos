<script lang="ts">
  import type { Brand } from "$lib/types";

  let {
    allBrands,
    selected = $bindable(),
    placeholder = "Cari merek…",
  }: { allBrands: Brand[]; selected: Set<string>; placeholder?: string } = $props();

  let query = $state("");

  const suggestions = $derived(
    query.trim()
      ? allBrands
          .filter((b) => b.name.toLowerCase().includes(query.trim().toLowerCase()) && !selected.has(b.name))
          .slice(0, 8)
      : [],
  );

  function add(name: string) {
    const next = new Set(selected);
    next.add(name);
    selected = next;
    query = "";
  }
  function remove(name: string) {
    const next = new Set(selected);
    next.delete(name);
    selected = next;
  }
</script>

<div class="brand-select">
  <div class="chips">
    {#each [...selected] as name (name)}
      <span class="chip">
        {name}
        <button type="button" onclick={() => remove(name)} aria-label="Hapus {name}">✕</button>
      </span>
    {/each}
    <div class="chip-input-wrap">
      <input {placeholder} bind:value={query} />
      {#if suggestions.length > 0}
        <div class="suggest-drop">
          {#each suggestions as b (b.id)}
            <button type="button" class="suggest-row" onclick={() => add(b.name)}>{b.name}</button>
          {/each}
        </div>
      {/if}
    </div>
  </div>
  {#if selected.size}
    <button type="button" class="btn-ghost reset-btn" onclick={() => (selected = new Set())}>✕ Reset Filter</button>
  {/if}
</div>

<style>
  .brand-select { display: flex; align-items: flex-start; gap: 0.5rem; flex-wrap: wrap; }
  .chips { display: flex; align-items: center; gap: 0.4rem; flex-wrap: wrap; flex: 1; min-width: 200px; }
  .chip {
    display: inline-flex; align-items: center; gap: 0.3rem;
    background: var(--primary); color: #fff; font-size: 0.82rem; font-weight: 600;
    padding: 0.25rem 0.35rem 0.25rem 0.65rem; border-radius: 999px; white-space: nowrap;
  }
  .chip button {
    background: rgba(255, 255, 255, 0.25); border: none; color: #fff;
    width: 1.1rem; height: 1.1rem; border-radius: 50%; padding: 0;
    display: flex; align-items: center; justify-content: center; font-size: 0.65rem; line-height: 1;
  }
  .chip button:hover { background: rgba(255, 255, 255, 0.45); }
  .chip-input-wrap { position: relative; min-width: 160px; }
  .chip-input-wrap input { width: 100%; min-width: 160px; }
  .suggest-drop {
    position: absolute; left: 0; right: 0; top: 100%; z-index: 100; margin-top: 0.2rem;
    background: var(--white); border: 1px solid var(--border);
    border-radius: var(--radius); box-shadow: var(--shadow);
    max-height: 220px; overflow-y: auto;
  }
  .suggest-row {
    display: block; width: 100%; text-align: left; border: none; border-radius: 0;
    border-bottom: 1px solid var(--border); padding: 0.4rem 0.7rem; font-size: 0.85rem; font-weight: 500;
  }
  .suggest-row:last-child { border-bottom: none; }
  .reset-btn { align-self: center; }
</style>
