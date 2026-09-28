<script lang="ts">
  import { tick } from "svelte";
  import { dialogQueue } from "$lib/dialog";

  // Tampilkan satu per satu — permintaan berikutnya menunggu giliran.
  const current = $derived($dialogQueue[0] ?? null);

  let okBtnEl = $state<HTMLButtonElement>();
  let cancelBtnEl = $state<HTMLButtonElement>();
  let lastFocusEl: HTMLElement | null = null;

  // Fokus default ke OK (sama seperti confirm() bawaan) supaya Enter langsung
  // menyetujui; fokus dikembalikan ke elemen semula begitu popup tertutup.
  let shownId = 0;
  $effect(() => {
    const id = current?.id ?? 0;
    if (id && id !== shownId) {
      if (!shownId) lastFocusEl = document.activeElement as HTMLElement | null;
      tick().then(() => okBtnEl?.focus());
    } else if (!id && shownId) {
      const el = lastFocusEl;
      lastFocusEl = null;
      tick().then(() => el?.focus?.());
    }
    shownId = id;
  });

  function answer(ok: boolean) {
    current?.resolve(ok);
  }

  function onKey(e: KeyboardEvent) {
    // Jangan sampai tombol yang ditekan di popup ini ikut diproses shortcut
    // halaman di belakangnya (Esc menutup tab, F-key kasir, dsb.).
    e.stopPropagation();
    if (e.key === "Escape") {
      e.preventDefault();
      answer(false);
    } else if ((e.key === "ArrowLeft" || e.key === "ArrowRight") && current?.kind === "confirm") {
      e.preventDefault();
      (e.key === "ArrowLeft" ? cancelBtnEl : okBtnEl)?.focus();
    }
  }
</script>

{#if current}
  <div class="modal-backdrop dialog-backdrop" onclick={() => answer(false)} role="presentation">
    <div
      class="modal app-dialog"
      onclick={(e) => e.stopPropagation()}
      onkeydown={onKey}
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="app-dialog-title"
      tabindex="-1"
    >
      <div class="dialog-icon">{current.icon ?? (current.kind === "alert" ? "ℹ️" : current.danger ? "⚠️" : "❓")}</div>
      <h2 id="app-dialog-title">{current.title ?? (current.kind === "alert" ? "Informasi" : "Konfirmasi")}</h2>
      <p class="dialog-msg">{current.message}</p>
      <div class="dialog-actions">
        {#if current.kind === "confirm"}
          <button bind:this={cancelBtnEl} onclick={() => answer(false)}>{current.cancelText ?? "Batal"}</button>
        {/if}
        <button
          class={current.danger ? "btn-danger-solid" : "btn-primary"}
          bind:this={okBtnEl}
          onclick={() => answer(true)}
        >
          {current.okText ?? "OK"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Di atas semua popup halaman (z-index 900) — popup konfirmasi bisa muncul
     dari dalam popup lain. */
  .dialog-backdrop { z-index: 2000; }
  .app-dialog { max-width: 400px; text-align: center; }
  .dialog-icon { font-size: 2.2rem; margin-bottom: 0.3rem; }
  .app-dialog h2 { margin: 0; }
  .dialog-msg {
    margin: 0.5rem 0 1.1rem;
    color: var(--text-dim);
    white-space: pre-line;
    overflow-wrap: anywhere;
    max-height: 50vh;
    overflow-y: auto;
  }
  .dialog-actions { display: flex; gap: 0.5rem; }
  .dialog-actions button { flex: 1; padding: 0.55rem 0.8rem; }
  .btn-danger-solid {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
    font-weight: 600;
  }
  .btn-danger-solid:hover { filter: brightness(0.93); }
  /* Tombol yang sedang fokus harus jelas — kasir memilih pakai panah + Enter. */
  .dialog-actions button:focus-visible { outline: 2px solid var(--text); outline-offset: 2px; }
</style>
