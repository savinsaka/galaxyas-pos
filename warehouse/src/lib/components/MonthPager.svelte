<script lang="ts">
  import { monthLabel, shiftMonthBounds } from "$lib/monthPager";

  let {
    from = $bindable(""),
    to = $bindable(""),
    onchange,
  }: { from?: string; to?: string; onchange: () => void } = $props();

  function go(delta: number) {
    [from, to] = shiftMonthBounds(from, delta);
    onchange();
  }
</script>

<div class="month-pager">
  <div class="mp-nav">
    <button class="btn-ghost" title="Bulan sebelumnya" onclick={() => go(-1)}>‹</button>
    <span class="mp-label mono">{monthLabel(from)}</span>
    <button class="btn-ghost" title="Bulan berikutnya" onclick={() => go(1)}>›</button>
  </div>
  <div class="mp-range">
    <label>Dari</label>
    <input type="date" bind:value={from} onchange={onchange} />
    <label>Sampai</label>
    <input type="date" bind:value={to} onchange={onchange} />
  </div>
</div>

<style>
  .month-pager {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }
  .mp-nav {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .mp-nav button {
    padding: 0.3rem 0.65rem;
    font-size: 1rem;
    font-weight: 700;
  }
  .mp-label {
    min-width: 9rem;
    text-align: center;
    font-weight: 650;
    text-transform: capitalize;
  }
  .mp-range {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .mp-range label {
    font-size: 0.8rem;
    color: var(--text-dim);
    margin: 0;
  }
  .mp-range input {
    width: 140px;
  }
</style>
