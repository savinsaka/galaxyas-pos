<script lang="ts">
  let {
    title,
    rows,
  }: { title?: string; rows: { label: string; value: string; bold?: boolean; danger?: boolean }[] } = $props();
</script>

<div class="card" style="padding:0; overflow:hidden;">
  {#if title}<div style="padding:0.7rem 0.9rem;"><b>{title}</b></div>{/if}
  <table>
    <tbody>
      {#each rows as r (r.label)}
        <tr>
          <td class:fw-bold={r.bold}>{r.label}</td>
          <td class="text-right mono" class:fw-bold={r.bold} style={r.danger ? "color:var(--danger)" : undefined}>{r.value}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .fw-bold { font-weight: 700; }
</style>
