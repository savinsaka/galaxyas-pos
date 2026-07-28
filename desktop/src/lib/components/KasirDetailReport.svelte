<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { formatIDR } from "$lib/format";
  import { toastError } from "$lib/toast";
  import { todayIso, formatPeriodLabel } from "$lib/dateTime";
  import { generateKasirDetailPdf } from "$lib/pdfTemplates";
  import { openPath } from "@tauri-apps/plugin-opener";
  import SummaryTable from "$lib/components/SummaryTable.svelte";
  import { totalsByMethod } from "$lib/payment";
  import type { Transaction } from "$lib/types";

  let from = $state(todayIso());
  let to = $state(todayIso());
  let txs = $state<Transaction[]>([]);
  let pdfBusy = $state(false);

  async function load() {
    try {
      txs = await api.listTransactions(from || null, to || null, 5000);
    } catch (e) {
      toastError(e);
    }
  }

  onMount(load);
  $effect(() => {
    from;
    to;
    load();
  });

  function presetHariIni() {
    from = todayIso();
    to = todayIso();
  }

  const totalTx = $derived(txs.length);
  const totalNet = $derived(txs.reduce((s, t) => s + t.total, 0));
  // Pembayaran kombinasi tidak jadi baris sendiri di laporan — nilainya sudah
  // dipecah ke Tunai & QRIS oleh `totalsByMethod`. Metode di luar daftar baku
  // (mis. transaksi kombinasi lama tanpa rincian) tetap ditampilkan di bawah.
  const METHOD_ORDER = ["Tunai", "QRIS", "Kartu"];
  const byMethod = $derived(totalsByMethod(txs));
  const methods = $derived([
    ...METHOD_ORDER,
    ...[...byMethod.keys()].filter((m) => !METHOD_ORDER.includes(m)),
  ]);
  const ringkasanRows = $derived([
    { label: "Total Transaksi", value: String(totalTx) },
    { label: "Total", value: formatIDR(totalNet), bold: true },
    ...methods.map((m) => ({ label: `Pembayaran ${m}`, value: formatIDR(byMethod.get(m)?.total ?? 0) })),
  ]);

  /** Generate PDF dokumen (layout tetap) untuk laporan ini. */
  async function cetakPdf() {
    pdfBusy = true;
    try {
      const s = await api.getSettings();
      const path = await generateKasirDetailPdf({
        title: "LAPORAN KASIR DETAIL",
        store_name: s.store_name || "GALAXYAS POS",
        periode: formatPeriodLabel(from, to),
        dicetak: `Dicetak: ${new Date().toLocaleString("id-ID")}`,
        ringkasan: JSON.stringify(ringkasanRows.map((r) => [r.label, r.value])),
      });
      await openPath(path);
    } catch (e) {
      toastError(e);
    } finally {
      pdfBusy = false;
    }
  }
</script>

<div class="card no-print" style="margin-bottom:1rem;">
  <div class="row" style="flex-wrap:wrap; gap:0.8rem; align-items:flex-end;">
    <div><label>Dari</label><input type="date" bind:value={from} /></div>
    <div><label>Sampai</label><input type="date" bind:value={to} /></div>
    <button onclick={presetHariIni}>Hari Ini</button>
    <button disabled={pdfBusy} onclick={cetakPdf} title="Cetak sebagai PDF">
      {pdfBusy ? "Membuat PDF…" : "📄 Cetak PDF"}
    </button>
  </div>
</div>

<div class="text-dim" style="margin-bottom:0.6rem;">{formatPeriodLabel(from, to)}</div>

<SummaryTable rows={ringkasanRows} />
