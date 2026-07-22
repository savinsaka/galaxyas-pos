<script lang="ts">
  import type { Snippet } from "svelte";
  import { printElement, extractReportForEscPos } from "$lib/print";
  import { buildReportEscPos } from "$lib/escpos";
  import { api } from "$lib/api";
  import { parseReceiptConfig } from "$lib/receipt";
  import { showToast, toastError } from "$lib/toast";

  let {
    title,
    subtitle,
    onClose,
    children,
  }: { title: string; subtitle?: string; onClose: () => void; children: Snippet } = $props();

  let strukBusy = $state(false);

  const printedAt = () =>
    new Date().toLocaleString("id-ID", { day: "2-digit", month: "long", year: "numeric", hour: "2-digit", minute: "2-digit" });

  function printDialog() {
    printElement("printable-report-content", title);
  }

  /** Cetak langsung ke printer thermal (ESC/POS) — bukan lewat dialog print, supaya tidak buram. */
  async function printStruk() {
    strukBusy = true;
    try {
      const cfg = parseReceiptConfig(await api.getSettings());
      const doc = extractReportForEscPos("printable-report-content", title, subtitle, `Dicetak: ${printedAt()}`);
      if (!doc) {
        toastError("Tidak ada data untuk dicetak.");
        return;
      }
      await api.printEscposTo(cfg.printer, buildReportEscPos(doc, cfg));
      showToast(cfg.printer ? `Dikirim ke printer: ${cfg.printer}` : "Dikirim ke printer default.", "success");
    } catch (e) {
      toastError(e);
    } finally {
      strukBusy = false;
    }
  }
</script>

<div class="modal-backdrop" onclick={onClose} role="presentation">
  <div class="modal report-modal" onclick={(e) => e.stopPropagation()} role="presentation">
    <button class="icon-close no-print" onclick={onClose} aria-label="Tutup" title="Tutup">✕</button>
    <div class="report-modal-head no-print">
      <h2>{title}</h2>
      <div class="row">
        <button onclick={printDialog}>🖨️ Print</button>
        <button disabled={strukBusy} onclick={printStruk} title="Cetak langsung ke printer thermal (ESC/POS)">
          {strukBusy ? "Mencetak…" : "🧾 Cetak Struk"}
        </button>
        <button class="btn-ghost" onclick={onClose}>Tutup</button>
      </div>
    </div>
    <div class="report-modal-body" id="printable-report-content">
      <div class="print-header">
        <h1>{title}</h1>
        {#if subtitle}<div class="print-subtitle">{subtitle}</div>{/if}
        <div class="print-meta">Dicetak: {printedAt()}</div>
        <hr />
      </div>
      {@render children()}
    </div>
  </div>
</div>

<style>
  .report-modal {
    width: 90vw;
    max-width: 1100px;
    max-height: 88vh;
    display: flex;
    flex-direction: column;
    position: relative;
  }
  .icon-close {
    position: absolute;
    top: 0.6rem;
    right: 0.6rem;
    width: 2rem;
    height: 2rem;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    font-size: 1rem;
    line-height: 1;
    background: var(--baby-blue-bg);
    border: 1px solid var(--border);
    color: var(--text);
    z-index: 1;
  }
  .icon-close:hover {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
  }
  .report-modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.8rem;
    padding-right: 2.2rem;
    flex-shrink: 0;
  }
  .report-modal-head h2 { margin: 0; }
  .report-modal-body {
    overflow-y: auto;
    padding-right: 0.3rem;
  }
  /* Header khusus cetak: disembunyikan di layar, cuma tampil saat window.print(). */
  .print-header { display: none; }
</style>
