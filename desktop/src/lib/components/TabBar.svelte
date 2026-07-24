<script lang="ts">
  import { tabs, activeTabId, setActive, closeTab, moveTab } from "$lib/stores/tabs";

  // HTML5 native drag-and-drop tidak jalan andal di webview Tauri (handler
  // drag-drop file milik Tauri menelan event-nya), jadi susun-ulang tab pakai
  // pointer event manual — ini lewat mulus di webview.
  let barEl = $state<HTMLElement>();
  let dragId = $state<string | null>(null);
  let downId: string | null = null;
  let startX = 0;
  let didDrag = false;

  // Roda mouse vertikal → geser tab horizontal (kayak Chrome).
  function hscroll(node: HTMLElement) {
    const onWheel = (e: WheelEvent) => {
      if (e.deltaY === 0) return;
      e.preventDefault();
      node.scrollLeft += e.deltaY;
    };
    node.addEventListener("wheel", onWheel, { passive: false });
    return { destroy: () => node.removeEventListener("wheel", onWheel) };
  }

  function tabIdAt(x: number, y: number): string | null {
    const el = document.elementFromPoint(x, y) as HTMLElement | null;
    return el?.closest<HTMLElement>(".tab")?.dataset.tabId ?? null;
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement;
    if (target.closest(".tab-close")) return; // biar tombol tutup jalan normal
    const id = target.closest<HTMLElement>(".tab")?.dataset.tabId;
    if (!id) return;
    downId = id;
    startX = e.clientX;
    didDrag = false;
    barEl?.setPointerCapture(e.pointerId);
  }
  function onPointerMove(e: PointerEvent) {
    if (downId == null) return;
    if (!didDrag) {
      if (Math.abs(e.clientX - startX) < 6) return; // ambang: bedakan klik vs seret
      didDrag = true;
      dragId = downId;
    }
    const overId = tabIdAt(e.clientX, e.clientY);
    if (dragId && overId && overId !== dragId) moveTab(dragId, overId);
  }
  function onPointerUp(e: PointerEvent) {
    if (barEl?.hasPointerCapture(e.pointerId)) barEl.releasePointerCapture(e.pointerId);
    // Gerakan kecil = klik biasa → aktifkan tab-nya.
    if (downId != null && !didDrag) setActive(downId);
    downId = null;
    dragId = null;
    didDrag = false;
  }
</script>

{#if $tabs.length}
  <div
    class="tabbar"
    role="tablist"
    bind:this={barEl}
    use:hscroll
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={onPointerUp}
  >
    {#each $tabs as tab (tab.id)}
      <div
        class="tab"
        class:active={tab.id === $activeTabId}
        class:dragging={tab.id === dragId}
        data-tab-id={tab.id}
        role="tab"
        tabindex="0"
      >
        <span>{tab.icon ?? "📄"}</span>
        <span class="tab-title">{tab.title}</span>
        <button class="tab-close" onclick={(e) => { e.stopPropagation(); closeTab(tab.id); }} title="Tutup">✕</button>
      </div>
    {/each}
  </div>
{/if}
