<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatQty, formatDateTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import {
    parseReceiptConfig,
    paperWidthMm,
    type ReceiptConfig,
  } from "$lib/receipt";
  import { buildStockDocEscPos } from "$lib/escpos";
  import { printReceiptElement } from "$lib/print";
  import type { StockMovementBatchDetail } from "$lib/types";

  let {
    detail,
    onClose,
  }: {
    detail: StockMovementBatchDetail;
    onClose: () => void;
  } = $props();

  const verb = detail.kind === "in" ? "ITEM MASUK" : "ITEM KELUAR";

  let cfg = $state<ReceiptConfig>({
    storeName: "GALAXYAS POS",
    address: "",
    phone: "",
    taxId: "",
    instagram: "",
    tiktok: "",
    whatsapp: "",
    header: "",
    footer: "",
    paper: "80",
    fontSize: 12,
    lineHeight: 1.35,
    margin: 3,
    printer: null,
    show: {
      storeName: true, address: true, phone: true, taxId: true, social: true, header: true,
      invoiceNo: true, date: true, items: true, subtotal: true, discount: true,
      total: true, paymentMethod: true, change: true, footer: true,
    },
  });

  onMount(async () => {
    try {
      const s = await api.getSettings();
      cfg = parseReceiptConfig(s);
    } catch (e) {
      toastError(e);
    }
  });

  const widthMm = $derived(paperWidthMm(cfg.paper));
  const totalQty = $derived(detail.items.reduce((s, it) => s + it.qty, 0));

  function printDialog() {
    printReceiptElement("stockdoc", widthMm, `${verb} ${detail.no}`);
  }

  async function printToPrinter() {
    try {
      await api.printEscposTo(cfg.printer, buildStockDocEscPos(detail, cfg));
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
        id="stockdoc"
        class="receipt"
        style="width:{widthMm}mm; font-size:{cfg.fontSize}px; line-height:{cfg.lineHeight}; padding:{cfg.margin}mm;"
      >
        {#if cfg.show.storeName && cfg.storeName.trim()}<div class="r-center r-bold">{cfg.storeName}</div>{/if}
        <div class="r-center r-bold">{verb}</div>
        <div class="r-center r-dim">{detail.no}</div>
        <div class="r-center r-dim">{formatDateTime(detail.created_at)}</div>

        <div class="r-sep"></div>
        {#each detail.items as it}
          <div class="r-item">
            <div>{it.product_name}</div>
            <div class="r-item-line"><span>Qty: {formatQty(it.qty)}</span></div>
            {#if it.note}<div class="r-item-line r-dim"><span>Ket: {it.note}</span></div>{/if}
          </div>
        {/each}
        <div class="r-sep"></div>
        <div class="r-line"><span>Total Item</span><span>{detail.items.length}</span></div>
        <div class="r-line"><span>Total Qty</span><span>{formatQty(totalQty)}</span></div>
        {#if detail.note}<div class="r-line"><span>Catatan</span><span>{detail.note}</span></div>{/if}
        {#if detail.user_id}<div class="r-line"><span>Oleh</span><span>{detail.user_id}</span></div>{/if}
        <div class="r-sep"></div>

        {#if cfg.show.footer}
          {#each cfg.footer.split("\n").map((x) => x.trim()).filter(Boolean) as f}
            <div class="r-center r-dim">{f}</div>
          {/each}
        {/if}
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
    align-items: flex-start;
    justify-content: center;
    background: #e9eef3;
    padding: 1rem;
    border-radius: 8px;
  }
  .receipt {
    flex: none;
    box-sizing: border-box;
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
</style>
