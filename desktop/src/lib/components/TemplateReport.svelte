<script lang="ts">
  import { renderToDom } from "$lib/report/render";
  import type { ReportContext } from "$lib/report/kinds";
  import type { ReportTemplate } from "$lib/report/template";

  let { template, ctx }: { template: ReportTemplate; ctx: ReportContext } = $props();

  let host = $state<HTMLDivElement>();
  // Render ulang tiap template/ctx berubah (mis. ganti granularity/rentang).
  $effect(() => {
    if (host) host.replaceChildren(renderToDom(template, ctx));
  });
</script>

<div bind:this={host} class="tpl-report"></div>

<style>
  /* Elemen absolut dari renderToDom butuh induk relatif berukuran; #print-root
     sudah relatif & selebar area isi kertas. Biarkan mengalir apa adanya. */
  .tpl-report :global(#print-root) { margin: 0 auto; }
  .tpl-report :global(table) { border-collapse: collapse; width: 100%; }
  .tpl-report :global(th),
  .tpl-report :global(td) { border: 1px solid #999; padding: 2px 5px; }
  .tpl-report :global(thead th) { background: #eee; }
</style>
