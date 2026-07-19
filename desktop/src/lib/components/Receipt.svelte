<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR, formatQty, formatDateTime } from "$lib/format";
  import { showToast, toastError } from "$lib/toast";
  import {
    parseReceiptConfig,
    paperWidthMm,
    type ReceiptConfig,
  } from "$lib/receipt";
  import { buildReceiptEscPos } from "$lib/escpos";
  import { printReceiptElement } from "$lib/print";
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
    address: "",
    phone: "",
    taxId: "",
    instagram: "",
    tiktok: "",
    whatsapp: "",
    header: "",
    footer: footer ?? "",
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
  const socialLine = $derived(
    [
      cfg.instagram.trim() && `IG: ${cfg.instagram.trim()}`,
      cfg.tiktok.trim() && `TikTok: ${cfg.tiktok.trim()}`,
      cfg.whatsapp.trim() && `WA: ${cfg.whatsapp.trim()}`,
    ]
      .filter(Boolean)
      .join(" · "),
  );

  function printDialog() {
    printReceiptElement("receipt", widthMm, `Struk ${detail.invoice_no}`);
  }

  async function printToPrinter() {
    try {
      await api.printEscposTo(cfg.printer, buildReceiptEscPos(detail, cfg));
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
        {#if cfg.show.storeName}<div class="r-center r-bold">{cfg.storeName}</div>{/if}
        {#if cfg.show.address}
          {#each cfg.address.split("\n").map((x) => x.trim()).filter(Boolean) as a}<div class="r-center r-dim">{a}</div>{/each}
        {/if}
        {#if cfg.show.phone && cfg.phone.trim()}<div class="r-center r-dim">{cfg.phone.trim()}</div>{/if}
        {#if cfg.show.taxId && cfg.taxId.trim()}<div class="r-center r-dim">NPWP: {cfg.taxId.trim()}</div>{/if}
        {#if cfg.show.social && socialLine}<div class="r-center r-dim">{socialLine}</div>{/if}
        {#if cfg.show.header}
          {#each headerLines as h}<div class="r-center r-dim">{h}</div>{/each}
        {/if}
        {#if cfg.show.date}<div class="r-center r-dim">{formatDateTime(detail.created_at)}</div>{/if}
        {#if cfg.show.invoiceNo}<div class="r-center r-dim">{detail.invoice_no}</div>{/if}
        {#if cfg.show.items}
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
        {/if}
        {#if cfg.show.subtotal || cfg.show.discount || cfg.show.total || cfg.show.paymentMethod || cfg.show.change}
          <div class="r-sep"></div>
          {#if cfg.show.subtotal}<div class="r-line"><span>Subtotal</span><span>{formatIDR(detail.subtotal)}</span></div>{/if}
          {#if cfg.show.discount}<div class="r-line"><span>Diskon</span><span>−{formatIDR(detail.discount)}</span></div>{/if}
          {#if cfg.show.total}<div class="r-line r-bold"><span>TOTAL</span><span>{formatIDR(detail.total)}</span></div>{/if}
          {#if cfg.show.paymentMethod}<div class="r-line"><span>{detail.payment_method}</span><span>{formatIDR(detail.paid)}</span></div>{/if}
          {#if cfg.show.change}<div class="r-line"><span>Kembali</span><span>{formatIDR(detail.change)}</span></div>{/if}
          <div class="r-sep"></div>
        {/if}
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
  .r-line { display: flex; justify-content: space-between; align-items: baseline; width: 100%; gap: 0.6em; }
  .r-item-line > span:first-child,
  .r-line > span:first-child { flex: 1 1 auto; min-width: 0; overflow-wrap: break-word; }
  .r-item-line > span:last-child,
  .r-line > span:last-child { flex: 0 0 auto; white-space: nowrap; }

  /* Semua logika print kini dilakukan via JS (clone ke body).
     Tidak ada CSS @media print di sini agar tidak konflik dengan laporan. */
</style>
