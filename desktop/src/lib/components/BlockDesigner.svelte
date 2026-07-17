<script lang="ts">
  import { dndzone } from "svelte-dnd-action";
  import type { ReportBlockDef, ReportDesignConfig } from "$lib/reportDesign";

  let {
    blocks,
    config,
  }: { blocks: ReportBlockDef[]; config: ReportDesignConfig } = $props();

  interface Row {
    id: string;
    label: string;
    hidden: boolean;
  }

  let rows = $state<Row[]>(toRows(config));

  // Sinkron ulang kalau config dari luar berubah (mis. setelah "Reset ke Default").
  let lastConfigRef = config;
  $effect(() => {
    if (config !== lastConfigRef) {
      lastConfigRef = config;
      rows = toRows(config);
    }
  });

  function toRows(cfg: ReportDesignConfig): Row[] {
    const byId = new Map(blocks.map((b) => [b.id, b.label]));
    return cfg.order
      .filter((id) => byId.has(id))
      .map((id) => ({ id, label: byId.get(id)!, hidden: cfg.hidden.includes(id) }));
  }

  function syncConfig() {
    config.order = rows.map((r) => r.id);
    config.hidden = rows.filter((r) => r.hidden).map((r) => r.id);
  }

  function onConsider(e: CustomEvent<{ items: Row[] }>) {
    rows = e.detail.items;
  }
  function onFinalize(e: CustomEvent<{ items: Row[] }>) {
    rows = e.detail.items;
    syncConfig();
  }
  function toggleHidden(id: string) {
    rows = rows.map((r) => (r.id === id ? { ...r, hidden: !r.hidden } : r));
    syncConfig();
  }
</script>

<div class="block-list" use:dndzone={{ items: rows, flipDurationMs: 150 }} onconsider={onConsider} onfinalize={onFinalize}>
  {#each rows as row (row.id)}
    <div class="block-row" class:hidden-row={row.hidden}>
      <span class="drag-handle" title="Seret untuk urutkan">⠿⠿</span>
      <span class="block-label">{row.label}</span>
      <button
        class="btn-ghost vis-toggle"
        title={row.hidden ? "Tampilkan di cetak" : "Sembunyikan dari cetak"}
        onclick={() => toggleHidden(row.id)}
      >
        {row.hidden ? "🚫 Disembunyikan" : "👁️ Tampil"}
      </button>
    </div>
  {/each}
</div>

<style>
  .block-list {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .block-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    background: var(--white);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.55rem 0.7rem;
  }
  .block-row.hidden-row {
    opacity: 0.55;
    background: var(--baby-blue-bg);
  }
  .drag-handle {
    cursor: grab;
    color: var(--text-dim);
    font-size: 0.9rem;
    user-select: none;
  }
  .block-label {
    flex: 1;
    font-weight: 600;
    font-size: 0.9rem;
  }
  .vis-toggle {
    font-size: 0.78rem;
    padding: 0.3rem 0.6rem;
    white-space: nowrap;
  }
</style>
