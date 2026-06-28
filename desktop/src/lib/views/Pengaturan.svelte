<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import { buildReceiptText, type ReceiptConfig } from "$lib/receipt";
  import type { TransactionDetail } from "$lib/types";
  import Receipt from "$lib/components/Receipt.svelte";

  let settings = $state<Record<string, string>>({});
  let printers = $state<string[]>([]);
  let savingKey = $state<string | null>(null);
  let savingReceipt = $state(false);
  let preview = $state(false);

  const tokoFields = [
    { key: "store_name", label: "Nama Toko", hint: "Tampil di struk & header" },
    { key: "store_id", label: "ID Toko", hint: "Identitas untuk sinkronisasi" },
    { key: "server_url", label: "URL Server Sync", hint: "http://localhost:8000" },
    { key: "tax_percent", label: "Pajak (%)", hint: "0 jika tidak memakai pajak" },
  ];

  const receiptKeys = [
    "receipt_paper",
    "receipt_printer",
    "receipt_font_size",
    "receipt_line_height",
    "receipt_margin",
    "receipt_header",
    "receipt_footer",
  ];

  const sampleDetail: TransactionDetail = {
    id: "sample",
    invoice_no: "INV-CONTOH-0001",
    cashier_id: "admin",
    subtotal: 25000,
    discount: 1000,
    total: 24000,
    paid: 50000,
    change: 26000,
    payment_method: "Tunai",
    created_at: new Date().toISOString(),
    items: [
      { product_id: "1", name: "Indomie Goreng", price: 3500, qty: 2, discount: 0, line_total: 7000 },
      { product_id: "2", name: "Aqua 600ml", price: 4000, qty: 1, discount: 1000, line_total: 3000 },
    ],
  };

  async function load() {
    try {
      settings = await api.getSettings();
      printers = await api.listPrinters();
      // default values agar input terkontrol
      settings.receipt_paper ??= "80";
      settings.receipt_font_size ??= "12";
      settings.receipt_line_height ??= "1.35";
      settings.receipt_margin ??= "3";
      settings.receipt_header ??= "";
      settings.receipt_footer ??= "";
    } catch (e) {
      toastError(e);
    }
  }
  onMount(load);

  async function saveToko(key: string) {
    savingKey = key;
    try {
      await api.updateSetting(key, settings[key] ?? "");
      showToast("Tersimpan.", "success");
    } catch (e) {
      toastError(e);
    } finally {
      savingKey = null;
    }
  }

  async function saveReceipt() {
    savingReceipt = true;
    try {
      for (const k of receiptKeys) await api.updateSetting(k, settings[k] ?? "");
      showToast("Pengaturan struk tersimpan.", "success");
    } catch (e) {
      toastError(e);
    } finally {
      savingReceipt = false;
    }
  }

  function cfgFromForm(): ReceiptConfig {
    return {
      storeName: settings.store_name || "GALAXYAS POS",
      header: settings.receipt_header || "",
      footer: settings.receipt_footer || "",
      paper: settings.receipt_paper === "58" ? "58" : "80",
      fontSize: Number(settings.receipt_font_size) || 12,
      lineHeight: Number(settings.receipt_line_height) || 1.35,
      margin: Number(settings.receipt_margin) || 3,
      printer: settings.receipt_printer || null,
    };
  }

  async function testPrint() {
    await saveReceipt();
    try {
      await api.printTextTo(settings.receipt_printer || null, buildReceiptText(sampleDetail, cfgFromForm()));
      showToast("Test print dikirim.", "success");
    } catch (e) {
      toastError(e);
    }
  }

  async function openPreview() {
    await saveReceipt();
    preview = true;
  }
</script>

<div class="page-head"><h1>Pengaturan</h1></div>

<div class="grid-2" style="align-items:start;">
  <div class="card">
    <h2>Toko & Server</h2>
    {#each tokoFields as f}
      <div style="margin-bottom:0.9rem;">
        <label>{f.label}</label>
        <div class="row">
          <input bind:value={settings[f.key]} placeholder={f.hint} />
          <button disabled={savingKey === f.key} onclick={() => saveToko(f.key)}>Simpan</button>
        </div>
      </div>
    {/each}
  </div>

  <div class="card">
    <h2>Struk & Printer</h2>

    <div class="grid-2">
      <div>
        <label>Printer</label>
        <div class="row">
          <select bind:value={settings.receipt_printer}>
            <option value="">(Printer default)</option>
            {#each printers as p}<option value={p}>{p}</option>{/each}
          </select>
          <button title="Muat ulang daftar printer" onclick={() => api.listPrinters().then((p) => (printers = p))}>↻</button>
        </div>
      </div>
      <div>
        <label>Ukuran Kertas</label>
        <select bind:value={settings.receipt_paper}>
          <option value="58">58 mm</option>
          <option value="80">80 mm</option>
        </select>
      </div>
      <div>
        <label>Ukuran Font (px)</label>
        <input type="number" min="8" max="20" bind:value={settings.receipt_font_size} />
      </div>
      <div>
        <label>Spasi Baris</label>
        <input type="number" min="1" max="2.5" step="0.05" bind:value={settings.receipt_line_height} />
      </div>
      <div>
        <label>Margin (mm)</label>
        <input type="number" min="0" max="10" bind:value={settings.receipt_margin} />
      </div>
    </div>

    <label style="margin-top:0.7rem;">Header Tambahan (alamat, telp — satu baris per item)</label>
    <textarea rows="3" bind:value={settings.receipt_header} placeholder={"Jl. Contoh No.1\nTelp 0812-xxxx"}></textarea>

    <label style="margin-top:0.7rem;">Footer Struk</label>
    <textarea rows="2" bind:value={settings.receipt_footer} placeholder="Terima kasih telah berbelanja!"></textarea>

    {#if printers.length === 0}
      <p class="text-dim" style="font-size:0.76rem; margin-top:0.5rem;">
        Daftar printer kosong (hanya terbaca di Windows). Kamu tetap bisa cetak via dialog.
      </p>
    {/if}

    <div class="row" style="justify-content:flex-end; margin-top:1rem;">
      <button onclick={openPreview}>👁️ Preview</button>
      <button onclick={testPrint}>🖨️ Test Print</button>
      <button class="btn-primary" disabled={savingReceipt} onclick={saveReceipt}>Simpan Pengaturan Struk</button>
    </div>
  </div>
</div>

{#if preview}
  <Receipt detail={sampleDetail} onClose={() => (preview = false)} />
{/if}
