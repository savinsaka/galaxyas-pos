<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR, formatQty, formatDateTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import {
    parseReceiptConfig,
    paperWidthMm,
    buildReceiptText,
    type ReceiptConfig,
  } from "$lib/receipt";
  import type { TransactionDetail } from "$lib/types";

  let {
    detail,
    storeName,
    footer,
    onClose,
  }: {
    detail: TransactionDetail;
    storeName?: string;
    footer?: string;
    onClose: () => void;
  } = $props();

  let cfg = $state<ReceiptConfig>({
    storeName: storeName ?? "GALAXYAS POS",
    header: "",
    footer: footer ?? "",
    paper: "80",
    fontSize: 12,
    lineHeight: 1.35,
    margin: 3,
    printer: null,
  });

  onMount(async () => {
    try {
      const s = await api.getSettings();
      const c = parseReceiptConfig(s);
      if (storeName) c.storeName = storeName;
      if (footer !== undefined) c.footer = footer;
      cfg = c;
    } catch (e) {
      toastError(e);
    }
  });

  const widthMm = $derived(paperWidthMm(cfg.paper));
  const headerLines = $derived(cfg.header.split("\n").map((x) => x.trim()).filter(Boolean));

  function printDialog() {
    window.print();
  }

  async function printToPrinter() {
    try {
      await api.printTextTo(cfg.printer, buildReceiptText(detail, cfg));
      showToast(cfg.printer ? `Dikirim ke printer: ${cfg.printer}` : "Dikirim ke printer default.", "success");
    } catch (e) {
      toastError(e);
    }
  }
</script>

<div class="modal-backdrop" onclick={onClose} role="presentation">
  <div class="modal receipt-modal" onclick={(e) => e.stopPropagation()} role="presentation">
    <div class="receipt-scroll">
      <div
        id="receipt"
        class="receipt"
        style="width:{widthMm}mm; font-size:{cfg.fontSize}px; line-height:{cfg.lineHeight}; padding:{cfg.margin}mm;"
      >
        <div class="r-center r-bold">{cfg.storeName}</div>
        {#each headerLines as h}<div class="r-center r-dim">{h}</div>{/each}
        <div class="r-center r-dim">{formatDateTime(detail.created_at)}</div>
        <div class="r-center r-dim">{detail.invoice_no}</div>
        <div class="r-sep"></div>
        {#each detail.items as it}
          <div class="r-item">
            <div>{it.name}</div>
            <div class="r-item-line"><span>{formatQty(it.qty)} x {formatIDR(it.price)}</span><span>{formatIDR(it.price * it.qty)}</span></div>
            {#if it.discount > 0}
              <div class="r-item-line r-dim"><span>Diskon</span><span>−{formatIDR(it.discount)}</span></div>
            {/if}
          </div>
        {/each}
        <div class="r-sep"></div>
        <div class="r-line"><span>Subtotal</span><span>{formatIDR(detail.subtotal)}</span></div>
        <div class="r-line"><span>Diskon</span><span>−{formatIDR(detail.discount)}</span></div>
        <div class="r-line r-bold"><span>TOTAL</span><span>{formatIDR(detail.total)}</span></div>
        <div class="r-line"><span>{detail.payment_method}</span><span>{formatIDR(detail.paid)}</span></div>
        <div class="r-line"><span>Kembali</span><span>{formatIDR(detail.change)}</span></div>
        <div class="r-sep"></div>
        {#each cfg.footer.split("\n").map((x) => x.trim()).filter(Boolean) as f}
          <div class="r-center r-dim">{f}</div>
        {/each}
      </div>
    </div>

    <div class="row" style="margin-top:1rem; justify-content:space-between;">
      <span class="text-dim" style="font-size:0.76rem;">Kertas {cfg.paper}mm{cfg.printer ? ` · ${cfg.printer}` : ""}</span>
      <div class="row">
        <button class="btn-ghost" onclick={onClose}>Tutup</button>
        <button onclick={printDialog}>Cetak (Dialog)</button>
        <button class="btn-primary" onclick={printToPrinter}>Cetak ke Printer</button>
      </div>
    </div>
  </div>
</div>

<style>
  .receipt-scroll {
    max-height: 60vh;
    overflow: auto;
    display: flex;
    justify-content: center;
    background: #e9eef3;
    padding: 1rem;
    border-radius: 8px;
  }
  .receipt {
    background: #fff;
    color: #000;
    font-family: "Courier New", monospace;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
  }
  .r-center { text-align: center; }
  .r-bold { font-weight: 700; }
  .r-dim { color: #555; }
  .r-sep { border-top: 1px dashed #000; margin: 6px 0; }
  .r-item { margin-bottom: 4px; }
  .r-item-line,
  .r-line { display: flex; justify-content: space-between; }

  @media print {
    :global(body *) {
      visibility: hidden;
    }
    #receipt,
    #receipt * {
      visibility: visible;
    }
    #receipt {
      position: fixed;
      left: 0;
      top: 0;
      box-shadow: none;
    }
  }
</style>
