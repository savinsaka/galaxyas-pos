<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { showToast, toastError } from "$lib/toast";
  import { parseReceiptConfig, RECEIPT_SHOW_KEYS, maxFontSizeForPaper } from "$lib/receipt";
  import { buildDrawerKick, buildReceiptEscPos } from "$lib/escpos";
  import { THEMES, saveTheme } from "$lib/theme";
  import type {
    LanServerStatus, MobileDevice, PairingQr, RelayStatus, TransactionDetail,
  } from "$lib/types";
  import { currentServer, isRemoteClient } from "$lib/stores/activeServer";
  import Receipt from "$lib/components/Receipt.svelte";

  type PTab = "toko" | "server" | "lan" | "struk" | "tema" | "kasir" | "lanjutan";
  let { section }: { section: PTab; tabId?: string } = $props();
  const SECTION_TITLES: Record<PTab, string> = {
    toko: "Informasi Toko",
    server: "Server Sinkronisasi",
    lan: "Server Pusat",
    struk: "Struk & Printer",
    tema: "Tema Tampilan",
    kasir: "Preferensi Kasir",
    lanjutan: "Lanjutan",
  };
  let activeTheme = $state("baby-blue");
  let savingTheme = $state(false);

  let lanStatus = $state<LanServerStatus | null>(null);
  let lanBusy = $state(false);

  // Akses Online (relay) — supaya HP kasir bisa dipakai dari luar wifi toko.
  let relay = $state<RelayStatus | null>(null);
  let relayBusy = $state(false);
  let relayForm = $state({ url: "", store_id: "", agent_key: "" });
  let devices = $state<MobileDevice[]>([]);
  let relayTimer: ReturnType<typeof setInterval> | null = null;
  let qr = $state<PairingQr | null>(null);
  let qrBusy = $state(false);

  let settings = $state<Record<string, string>>({});
  let printers = $state<string[]>([]);
  let savingKey = $state<string | null>(null);
  let savingReceipt = $state(false);
  let savingServer = $state(false);
  let preview = $state(false);
  let spacingChanged = $state(false);
  let spacingTimer: ReturnType<typeof setTimeout> | null = null;
  const fontSizeMax = $derived(maxFontSizeForPaper(settings.receipt_paper === "58" ? "58" : "80"));

  const RESET_PHRASE = "HAPUS SEMUA DATA";
  let resetConfirmText = $state("");
  let resetting = $state(false);

  const tokoFields = [
    { key: "store_name", label: "Nama Toko", hint: "Tampil di header & struk" },
    { key: "store_address", label: "Alamat", hint: "Jl. Contoh No.1, Kota" },
    { key: "store_phone", label: "Telepon", hint: "0812-xxxx-xxxx" },
    { key: "store_tax_id", label: "NPWP", hint: "Opsional" },
    { key: "store_instagram", label: "Instagram", hint: "@namatoko" },
    { key: "store_tiktok", label: "TikTok", hint: "@namatoko" },
    { key: "store_whatsapp", label: "WhatsApp", hint: "0812-xxxx-xxxx" },
    { key: "tax_percent", label: "Pajak (%)", hint: "0 jika tidak memakai pajak" },
  ];
  const serverFields = [
    { key: "store_id", label: "ID Toko", hint: "Identitas untuk sinkronisasi" },
    { key: "server_url", label: "URL Server Sync", hint: "http://localhost:8000" },
  ];
  // Server GALAXYAS Mobile (bridge tombol "Pull" di Item Masuk) — TERPISAH
  // dari Server Sinkronisasi di atas (beda server, beda database, beda auth).
  const mobileFields = [
    { key: "mobile_server_url", label: "URL Server Mobile", hint: "https://mobile-api.jjapps.net" },
    { key: "mobile_store_api_key", label: "API Key Toko", hint: "Diminta ke Bos (dari app mobile)" },
  ];
  let savingMobile = $state(false);
  const receiptKeys = [
    "receipt_paper", "receipt_printer", "receipt_font_size",
    "receipt_line_height", "receipt_margin", "receipt_header", "receipt_footer",
    "receipt_cash_drawer",
    ...RECEIPT_SHOW_KEYS.map((k) => k.setting),
  ];

  const sampleDetail: TransactionDetail = {
    id: "sample", invoice_no: "INV-CONTOH-0001", cashier_id: "admin",
    subtotal: 25000, discount: 1000, total: 24000, paid: 50000, change: 26000,
    payment_method: "Tunai", created_at: new Date().toISOString(),
    customer_id: null, shift_id: null,
    items: [
      { product_id: "1", name: "Indomie Goreng", price: 3500, qty: 2, discount: 0, line_total: 7000 },
      { product_id: "2", name: "Aqua 600ml", price: 4000, qty: 1, discount: 1000, line_total: 3000 },
    ],
  };

  async function loadLanStatus() {
    if ($isRemoteClient) return;
    try {
      lanStatus = await api.lanServerStatus();
    } catch (e) {
      toastError(e);
    }
  }

  async function toggleLanServer() {
    lanBusy = true;
    try {
      lanStatus = await api.setLanServerEnabled(!lanStatus?.enabled);
      showToast(lanStatus.enabled ? "Server Pusat aktif." : "Server Pusat dimatikan.", "success");
    } catch (e) {
      toastError(e);
    } finally {
      lanBusy = false;
    }
  }

  async function regenerateToken() {
    lanBusy = true;
    try {
      lanStatus = await api.regenerateLanToken();
      qr = null; // QR lama memuat kode yang sudah mati
      showToast("Kode pairing baru dibuat.", "success");
    } catch (e) {
      toastError(e);
    } finally {
      lanBusy = false;
    }
  }

  /// QR memuat kode pairing, jadi hanya dibuat saat diminta — bukan dipajang
  /// terus-menerus di layar kasir yang dilihat banyak orang.
  async function showQr() {
    qrBusy = true;
    try {
      qr = await api.pairingQr();
    } catch (e) {
      toastError(e);
    } finally {
      qrBusy = false;
    }
  }

  async function loadRelay() {
    if ($isRemoteClient) return;
    try {
      relay = await api.relayStatus();
      relayForm.url = relay.url;
      relayForm.store_id = relay.store_id;
      devices = await api.listMobileDevices();
    } catch (e) {
      toastError(e);
    }
  }

  async function saveRelay() {
    relayBusy = true;
    try {
      relay = await api.saveRelaySettings({ ...relayForm });
      relayForm.agent_key = ""; // tidak ditampilkan lagi setelah tersimpan
      showToast("Pengaturan Akses Online disimpan.", "success");
    } catch (e) {
      toastError(e);
    } finally {
      relayBusy = false;
    }
  }

  async function toggleRelay() {
    relayBusy = true;
    try {
      relay = await api.setRelayEnabled(!relay?.enabled);
      qr = null; // isi QR ikut berubah (alamat relay masuk/hilang)
      showToast(
        relay.enabled ? "Akses Online dinyalakan." : "Akses Online dimatikan.",
        "success",
      );
    } catch (e) {
      toastError(e);
      relay = await api.relayStatus();
    } finally {
      relayBusy = false;
    }
  }

  async function revokeDevice(d: MobileDevice) {
    if (!confirm(`Cabut akses "${d.name}"? HP itu harus pairing ulang untuk bisa dipakai lagi.`))
      return;
    try {
      await api.revokeMobileDevice(d.id);
      devices = await api.listMobileDevices();
      showToast("Akses perangkat dicabut.", "success");
    } catch (e) {
      toastError(e);
    }
  }

  function fmtWaktu(iso: string | null): string {
    if (!iso) return "belum pernah";
    return new Date(iso).toLocaleString("id-ID");
  }

  async function disconnectServer() {
    if (!confirm("Putuskan koneksi dari Server Pusat dan kembali ke mode lokal?")) return;
    lanBusy = true;
    try {
      await api.selectServer("local");
      window.location.reload();
    } catch (e) {
      toastError(e);
      lanBusy = false;
    }
  }

  async function load() {
    try {
      settings = await api.getSettings();
      printers = await api.listPrinters();
      await loadLanStatus();
      await loadRelay();
      settings.receipt_paper ??= "80";
      settings.receipt_font_size ??= "12";
      clampFontSize();
      settings.receipt_line_height ??= "1.35";
      settings.receipt_margin ??= "3";
      settings.receipt_header ??= "";
      settings.receipt_footer ??= "";
      settings.receipt_cash_drawer ??= "pin2";
      settings.store_name ??= "";
      settings.store_address ??= "";
      settings.store_phone ??= "";
      settings.store_tax_id ??= "";
      settings.store_instagram ??= "";
      settings.store_tiktok ??= "";
      settings.store_whatsapp ??= "";
      settings.store_id ??= "";
      settings.server_url ??= "";
      settings.mobile_server_url ??= "";
      settings.mobile_store_api_key ??= "";
      settings.tax_percent ??= "0";
      settings.theme ??= "baby-blue";
      activeTheme = settings.theme;
      settings.kasir_scan_mode ??= "scan_first";
      for (const { setting } of RECEIPT_SHOW_KEYS) settings[setting] ??= "1";
    } catch (e) { toastError(e); }
  }
  onMount(() => {
    load();
    // Status agent hidup di background thread; polling ringan supaya indikator
    // "Terhubung" ikut berubah saat internet toko putus/nyambung lagi.
    relayTimer = setInterval(() => {
      if (section === "lan" && !$isRemoteClient && !relayBusy) {
        api.relayStatus().then((s) => (relay = s)).catch(() => {});
      }
    }, 5000);
    return () => {
      if (relayTimer) clearInterval(relayTimer);
    };
  });

  let savingScanMode = $state(false);
  async function pickScanMode(mode: "scan_first" | "jumlah_first") {
    savingScanMode = true;
    try {
      await api.updateSetting("kasir_scan_mode", mode);
      settings.kasir_scan_mode = mode;
      showToast("Preferensi kasir diterapkan.", "success");
    } catch (e) {
      toastError(e);
    } finally {
      savingScanMode = false;
    }
  }

  async function pickTheme(key: string) {
    activeTheme = key;
    savingTheme = true;
    try {
      await saveTheme(key);
      settings.theme = key;
      showToast("Tema diterapkan.", "success");
    } catch (e) {
      toastError(e);
    } finally {
      savingTheme = false;
    }
  }

  async function saveToko(key: string) {
    savingKey = key;
    try {
      await api.updateSetting(key, String(settings[key] ?? ""));
      showToast("Tersimpan.", "success");
    } catch (e) { toastError(e); } finally { savingKey = null; }
  }

  async function saveServer() {
    savingServer = true;
    try {
      for (const f of serverFields) await api.updateSetting(f.key, String(settings[f.key] ?? ""));
      showToast("Pengaturan server tersimpan.", "success");
    } catch (e) { toastError(e); } finally { savingServer = false; }
  }

  async function saveMobile() {
    savingMobile = true;
    try {
      for (const f of mobileFields) await api.updateSetting(f.key, String(settings[f.key] ?? ""));
      showToast("Pengaturan server mobile tersimpan.", "success");
    } catch (e) { toastError(e); } finally { savingMobile = false; }
  }

  async function saveReceipt() {
    savingReceipt = true;
    try {
      for (const k of receiptKeys) await api.updateSetting(k, String(settings[k] ?? ""));
      showToast("Pengaturan struk tersimpan.", "success");
      spacingChanged = false;
    } catch (e) { toastError(e); } finally { savingReceipt = false; }
  }

  async function testPrint() {
    await saveReceipt();
    try {
      await api.printEscposTo(settings.receipt_printer || null, buildReceiptEscPos(sampleDetail, parseReceiptConfig(settings)));
      showToast("Test print dikirim.", "success");
    } catch (e) { toastError(e); }
  }

  async function testDrawer() {
    await saveReceipt();
    const cfg = parseReceiptConfig(settings);
    if (cfg.cashDrawer === "off") return showToast("Laci kasir sedang dimatikan.", "info");
    try {
      await api.printEscposTo(settings.receipt_printer || null, buildDrawerKick(cfg.cashDrawer));
      showToast("Perintah buka laci dikirim.", "success");
    } catch (e) { toastError(e); }
  }

  async function openPreview() {
    await saveReceipt();
    preview = true;
  }

  function onSpacingInput() {
    spacingChanged = true;
    if (spacingTimer) clearTimeout(spacingTimer);
    spacingTimer = setTimeout(() => {
      showToast("Spasi/margin diubah — klik Simpan Struk untuk menerapkan.", "info");
    }, 800);
  }

  /** Kertas 58mm punya lebar lebih sempit — batasi font agar baris qty/harga tidak pecah. */
  function clampFontSize() {
    if (Number(settings.receipt_font_size) > fontSizeMax) {
      settings.receipt_font_size = String(fontSizeMax);
    }
  }

  function onPaperChange() {
    clampFontSize();
    onSpacingInput();
  }

  function onFontSizeInput() {
    clampFontSize();
    onSpacingInput();
  }

  async function doResetData() {
    if (resetConfirmText.trim() !== RESET_PHRASE) return;
    if (!confirm("Ini akan menghapus SEMUA data barang, stok, transaksi, diskon, dan merek secara permanen. Lanjutkan?")) return;
    resetting = true;
    try {
      await api.resetData(resetConfirmText.trim());
      showToast("Semua data berhasil dihapus.", "success");
      resetConfirmText = "";
    } catch (e) {
      toastError(e);
    } finally {
      resetting = false;
    }
  }
</script>

<div class="page-head"><h1>{SECTION_TITLES[section]}</h1></div>

<!-- ── Informasi Toko ── -->
{#if section === "toko"}
  <div class="card" style="max-width:520px;">
    <h2>Informasi Toko</h2>
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
{/if}

<!-- ── Tab: Server Sinkronisasi ── -->
{#if section === "server"}
  <div class="card" style="max-width:520px;">
    <h2>Server Sinkronisasi</h2>
    <p class="text-dim" style="margin-top:0; font-size:0.83rem;">
      Hanya diperlukan jika menggunakan fitur sinkronisasi data ke server pusat.
      Biarkan kosong jika hanya dipakai secara lokal.
    </p>
    {#each serverFields as f}
      <div style="margin-bottom:0.9rem;">
        <label>{f.label}</label>
        <input bind:value={settings[f.key]} placeholder={f.hint} />
      </div>
    {/each}
    <div class="row" style="justify-content:flex-end;">
      <button class="btn-primary" disabled={savingServer} onclick={saveServer}>
        Simpan Pengaturan Server
      </button>
    </div>
  </div>

  <div class="card" style="max-width:520px; margin-top:0.8rem;">
    <h2>Server Mobile (Bridge Pull)</h2>
    <p class="text-dim" style="margin-top:0; font-size:0.83rem;">
      Server terpisah untuk app mobile (order/pengirim/absen/gaji/dst). Diisi
      sekali agar tombol "Pull" di menu Item Masuk bisa menarik pengiriman
      dari Pengirim. API Key didapat dari Bos lewat app mobile.
    </p>
    {#each mobileFields as f}
      <div style="margin-bottom:0.9rem;">
        <label>{f.label}</label>
        <input bind:value={settings[f.key]} placeholder={f.hint} />
      </div>
    {/each}
    <div class="row" style="justify-content:flex-end;">
      <button class="btn-primary" disabled={savingMobile} onclick={saveMobile}>
        Simpan Pengaturan Mobile
      </button>
    </div>
  </div>
{/if}

<!-- ── Tab: Server Pusat (LAN multi-kasir) ── -->
{#if section === "lan"}
  <div class="card" style="max-width:560px;">
    {#if $isRemoteClient}
      <h2>Server Pusat</h2>
      <p class="text-dim" style="margin-top:0; font-size:0.83rem;">
        PC ini terhubung sebagai kasir client ke Server Pusat berikut. Semua data
        (barang, stok, transaksi, akun) dibagikan langsung dari PC tersebut.
      </p>
      <div class="row" style="gap:1.5rem; margin:0.8rem 0;">
        <div><div class="text-dim">Nama</div><strong>{$currentServer?.name}</strong></div>
        <div><div class="text-dim">Alamat</div><strong class="mono">{$currentServer?.host}:{$currentServer?.port}</strong></div>
      </div>
      <button class="btn-danger" disabled={lanBusy} onclick={disconnectServer}>
        Putuskan Koneksi
      </button>
    {:else}
      <h2>Server Pusat</h2>
      <p class="text-dim" style="margin-top:0; font-size:0.83rem;">
        Jadikan PC ini "Server Pusat" supaya kasir lain di jaringan wifi yang sama bisa
        konek dan berbagi data barang, stok, transaksi, dan akun secara langsung — tanpa
        cloud. PC kasir lain memilih "+ Tambah Server" di layar awal, lalu masukkan IP dan
        Kode Pairing di bawah ini.
      </p>
      <label class="row" style="gap:0.6rem; margin:0.9rem 0;">
        <input
          type="checkbox"
          style="width:auto;"
          checked={lanStatus?.enabled ?? false}
          disabled={lanBusy || !lanStatus}
          onchange={toggleLanServer}
        />
        Jadikan PC ini Server Pusat
      </label>
      {#if lanStatus?.enabled}
        <div class="card" style="background:var(--baby-blue-bg); margin-top:0.6rem;">
          <div style="margin-bottom:0.7rem;">
            <div class="text-dim" style="font-size:0.78rem;">Alamat IP (untuk dimasukkan di PC kasir lain)</div>
            <strong class="mono" style="font-size:1.1rem;">
              {lanStatus.local_ip ?? "(IP tidak terdeteksi — cek koneksi wifi)"}:{lanStatus.port}
            </strong>
          </div>
          <div style="margin-bottom:0.7rem;">
            <div class="text-dim" style="font-size:0.78rem;">Kode Pairing</div>
            <strong class="mono" style="font-size:1.3rem; letter-spacing:0.1em;">{lanStatus.token}</strong>
          </div>
          <button disabled={lanBusy} onclick={regenerateToken}>🔄 Buat Kode Baru</button>
        </div>

        <!-- QR pairing: Store ID relay 32 karakter hex tidak mungkin diketik
             benar di HP, jadi semua isian dijadikan satu QR. -->
        <div class="card" style="margin-top:0.8rem;">
          <div style="font-weight:600; margin-bottom:0.3rem;">Pairing Cepat (QR)</div>
          <p class="text-dim" style="margin:0 0 0.6rem; font-size:0.8rem;">
            Di HP: <em>+ Tambah Server</em> → <em>Scan QR</em>. Alamat LAN, alamat
            relay, dan kode pairing terisi sendiri.
          </p>
          {#if qr}
            <div style="background:#fff; padding:10px; display:inline-block; border-radius:6px;">
              <!-- eslint-disable-next-line svelte/no-at-html-tags -->
              {@html qr.svg}
            </div>
            <div class="text-dim" style="font-size:0.78rem; margin-top:0.5rem;">
              {qr.payload.name} ·
              {qr.payload.host ? `${qr.payload.host}:${qr.payload.port}` : "IP LAN tidak terdeteksi"}
              {#if qr.payload.relay}· online: {qr.payload.relay}{:else}· online: belum aktif{/if}
            </div>
            <button style="margin-top:0.5rem;" onclick={() => (qr = null)}>Sembunyikan</button>
          {:else}
            <button disabled={qrBusy} onclick={showQr}>
              {qrBusy ? "Membuat…" : "📱 Tampilkan QR"}
            </button>
          {/if}
        </div>
      {/if}
    {/if}
  </div>

  {#if !$isRemoteClient}
    <!-- Akses Online: HP kasir dari luar wifi toko, lewat relay di VPS. -->
    <div class="card" style="max-width:560px; margin-top:1rem;">
      <h2>Akses Online (HP dari luar toko)</h2>
      <p class="text-dim" style="margin-top:0; font-size:0.83rem;">
        Supaya app HP bisa dipakai dari mana saja, bukan cuma di wifi toko. PC ini
        menyambung keluar ke relay, jadi tidak perlu setting router. <strong>PC ini harus
        tetap menyala</strong> — kalau mati, HP langsung diberi tahu dan tidak ada
        transaksi yang tertahan.
      </p>

      <label>URL Relay</label>
      <input bind:value={relayForm.url} placeholder="relay.jjapps.net" disabled={relayBusy} />
      <label>Store ID</label>
      <input bind:value={relayForm.store_id} placeholder="dari skrip add_store.py" disabled={relayBusy} />
      <label>Agent Key</label>
      <input
        type="password"
        bind:value={relayForm.agent_key}
        placeholder={relay?.enabled || relay?.store_id ? "(tersimpan — isi hanya bila ingin ganti)" : "dari skrip add_store.py"}
        disabled={relayBusy}
      />
      <div class="row" style="margin-top:0.7rem;">
        <button disabled={relayBusy} onclick={saveRelay}>Simpan</button>
        <button disabled={relayBusy} onclick={toggleRelay}>
          {relay?.enabled ? "Matikan Akses Online" : "Nyalakan Akses Online"}
        </button>
      </div>

      {#if relay?.enabled}
        <div class="card" style="background:var(--baby-blue-bg); margin-top:0.8rem;">
          <div class="row" style="gap:0.5rem;">
            <span style="font-size:1.1rem;">{relay.connected ? "🟢" : "🔴"}</span>
            <strong>{relay.connected ? "Terhubung ke relay" : "Tidak terhubung"}</strong>
          </div>
          {#if relay.connected && relay.connected_since}
            <div class="text-dim" style="font-size:0.78rem; margin-top:0.3rem;">
              Sejak {fmtWaktu(relay.connected_since)}
            </div>
          {/if}
          {#if !relay.connected && relay.last_error}
            <div class="text-dim" style="font-size:0.78rem; margin-top:0.3rem;">
              {relay.last_error} — mencoba menyambung ulang otomatis.
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <div class="card" style="max-width:560px; margin-top:1rem;">
      <h2>Perangkat Terhubung</h2>
      <p class="text-dim" style="margin-top:0; font-size:0.83rem;">
        HP yang sudah pairing dengan Server Pusat ini. Tiap HP punya kunci sendiri —
        mencabut satu HP tidak mengganggu yang lain. Ganti Kode Pairing di atas hanya
        mencegah HP <em>baru</em> mendaftar, tidak memutus yang sudah terdaftar.
      </p>
      {#if devices.length === 0}
        <p class="text-dim" style="font-size:0.83rem;">Belum ada HP yang terdaftar.</p>
      {:else}
        <table>
          <thead>
            <tr><th>Nama</th><th>Terakhir dipakai</th><th></th></tr>
          </thead>
          <tbody>
            {#each devices as d (d.id)}
              <tr>
                <td>{d.name}</td>
                <td class="text-dim">{fmtWaktu(d.last_seen_at)}</td>
                <td style="text-align:right;">
                  <button class="btn-danger" onclick={() => revokeDevice(d)}>Cabut</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
      <button style="margin-top:0.6rem;" onclick={loadRelay}>↻ Muat Ulang</button>
    </div>
  {/if}
{/if}

<!-- ── Tab: Struk & Printer ── -->
{#if section === "struk"}
  <div class="card">
    <h2>Struk &amp; Printer</h2>
    <div class="struk-grid">
      <div>
        <label>Printer</label>
        <div class="row">
          <select bind:value={settings.receipt_printer}>
            <option value="">(Printer default)</option>
            {#each printers as p}<option value={p}>{p}</option>{/each}
          </select>
          <button title="Muat ulang" onclick={() => api.listPrinters().then((p) => (printers = p))}>↻</button>
        </div>
      </div>
      <div>
        <label>Ukuran Kertas</label>
        <select bind:value={settings.receipt_paper} onchange={onPaperChange}>
          <option value="58">58 mm</option>
          <option value="80">80 mm</option>
        </select>
      </div>
      <div>
        <label>Ukuran Font (px)</label>
        <input type="number" min="8" max={fontSizeMax} bind:value={settings.receipt_font_size} oninput={onFontSizeInput} />
        {#if fontSizeMax < 20}<div class="text-dim" style="font-size:0.76rem; margin-top:0.25rem;">Maks {fontSizeMax}px untuk kertas 58mm agar baris qty/harga tidak pecah.</div>{/if}
      </div>
      <div>
        <label>Spasi Baris</label>
        <input type="number" min="1" max="2.5" step="0.05" bind:value={settings.receipt_line_height} oninput={onSpacingInput} />
      </div>
      <div>
        <label>Margin (mm)</label>
        <input type="number" min="0" max="10" bind:value={settings.receipt_margin} oninput={onSpacingInput} />
      </div>
      <div>
        <label>Laci Kasir (Cash Drawer)</label>
        <select bind:value={settings.receipt_cash_drawer}>
          <option value="pin2">Pin 2 (umum)</option>
          <option value="pin5">Pin 5</option>
          <option value="both">Pin 2 &amp; 5 (kalau ragu)</option>
          <option value="off">Mati</option>
        </select>
        <div class="text-dim" style="font-size:0.76rem; margin-top:0.25rem;">
          Laci membuka otomatis hanya untuk pembayaran Tunai &amp; Kombinasi. Manual: F8 di Kasir.
        </div>
      </div>
    </div>
    <label style="margin-top:0.9rem;">Header Tambahan</label>
    <textarea rows="3" bind:value={settings.receipt_header} placeholder={"Jl. Contoh No.1\nTelp 0812-xxxx"}></textarea>
    <label style="margin-top:0.7rem;">Footer Struk</label>
    <textarea rows="2" bind:value={settings.receipt_footer} placeholder="Terima kasih telah berbelanja!"></textarea>

    <label style="margin-top:0.9rem;">🧩 Blok Struk (Designer)</label>
    <p class="text-dim" style="margin-top:0; font-size:0.78rem;">
      Pilih blok yang ditampilkan pada struk cetak (Preview / Test Print / saat kasir mencetak).
    </p>
    <div class="block-grid">
      {#each RECEIPT_SHOW_KEYS as k}
        <label class="row block-cb">
          <input
            type="checkbox"
            style="width:auto;"
            checked={settings[k.setting] !== "0"}
            onchange={(e) => { settings[k.setting] = e.currentTarget.checked ? "1" : "0"; spacingChanged = true; }}
          />
          {k.label}
        </label>
      {/each}
    </div>
    {#if printers.length === 0}
      <p class="text-dim" style="font-size:0.76rem; margin-top:0.5rem;">
        Daftar printer kosong — hanya terbaca di Windows. Cetak via dialog tetap bisa digunakan.
      </p>
    {/if}
    <div class="row" style="justify-content:flex-end; margin-top:1rem; flex-wrap:wrap; gap:0.4rem;">
      <button onclick={openPreview}>👁️ Preview</button>
      <button onclick={testPrint}>🖨️ Test Print</button>
      <button onclick={testDrawer}>💰 Tes Buka Laci</button>
      <button class="btn-primary" class:btn-warning={spacingChanged} disabled={savingReceipt} onclick={saveReceipt}>
        {spacingChanged ? "⚠ Simpan Struk*" : "Simpan Struk"}
      </button>
    </div>
  </div>
{/if}

<!-- ── Tab: Tema ── -->
{#if section === "tema"}
  <div class="card" style="max-width:640px;">
    <h2>Tema Tampilan</h2>
    <p class="text-dim" style="margin-top:0; font-size:0.83rem;">
      Pilih skema warna aplikasi, termasuk mode gelap. Berlaku langsung dan tersimpan untuk semua kasir di toko ini.
    </p>
    <div class="theme-grid">
      {#each THEMES as t}
        <button class="theme-card" class:active={activeTheme === t.key} disabled={savingTheme} onclick={() => pickTheme(t.key)}>
          <span class="theme-swatch" style="background:{t.swatch};"></span>
          <span class="theme-name">{t.label}</span>
          {#if activeTheme === t.key}<span class="theme-check">✓</span>{/if}
        </button>
      {/each}
    </div>
  </div>
{/if}

<!-- ── Tab: Preferensi Kasir ── -->
{#if section === "kasir"}
  <div class="card" style="max-width:640px;">
    <h2>Preferensi Kasir</h2>
    <p class="text-dim" style="margin-top:0; font-size:0.83rem;">
      Atur alur scan barang di layar Kasir POS. Berlaku langsung untuk semua kasir di toko ini.
    </p>
    <div class="scanmode-grid">
      <button
        class="scanmode-card"
        class:active={(settings.kasir_scan_mode ?? "scan_first") === "scan_first"}
        disabled={savingScanMode}
        onclick={() => pickScanMode("scan_first")}
      >
        <span class="scanmode-name">Scan First <span class="text-dim" style="font-weight:400;">(default)</span></span>
        <span class="text-dim" style="font-size:0.82rem;">
          Isi Jumlah dulu (opsional), lalu scan barcode / Enter di search — barang langsung masuk daftar keranjang.
        </span>
        {#if (settings.kasir_scan_mode ?? "scan_first") === "scan_first"}<span class="theme-check">✓</span>{/if}
      </button>
      <button
        class="scanmode-card"
        class:active={settings.kasir_scan_mode === "jumlah_first"}
        disabled={savingScanMode}
        onclick={() => pickScanMode("jumlah_first")}
      >
        <span class="scanmode-name">Jumlah First</span>
        <span class="text-dim" style="font-size:0.82rem;">
          Scan barcode dulu — barang TIDAK langsung masuk, cursor pindah ke Jumlah. Isi jumlah lalu Enter baru masuk daftar keranjang.
        </span>
        {#if settings.kasir_scan_mode === "jumlah_first"}<span class="theme-check">✓</span>{/if}
      </button>
    </div>
  </div>
{/if}

<!-- ── Tab: Lanjutan (Zona Berbahaya) ── -->
{#if section === "lanjutan"}
  <div class="card danger-zone" style="max-width:560px;">
    <h2>⚠️ Reset Data</h2>
    <p class="text-dim" style="margin-top:0; font-size:0.83rem;">
      Menghapus <b>seluruh</b> data barang, stok, transaksi, diskon periodik, dan merek secara
      permanen. Akun pengguna &amp; pengaturan toko <b>tidak</b> ikut terhapus. Tindakan ini
      <b>tidak bisa dibatalkan</b> — pastikan sudah export/backup data bila perlu.
    </p>
    <label>Ketik <span class="mono" style="font-weight:700;">{RESET_PHRASE}</span> untuk mengaktifkan tombol hapus</label>
    <input bind:value={resetConfirmText} placeholder={RESET_PHRASE} />
    <div class="row" style="justify-content:flex-end; margin-top:0.9rem;">
      <button
        class="btn-danger"
        disabled={resetting || resetConfirmText.trim() !== RESET_PHRASE}
        onclick={doResetData}
      >
        {resetting ? "Menghapus…" : "🗑️ Hapus Semua Data"}
      </button>
    </div>
  </div>
{/if}

{#if preview}
  <Receipt detail={sampleDetail} onClose={() => (preview = false)} />
{/if}

<style>
  .danger-zone {
    border-color: var(--danger);
    background: color-mix(in srgb, var(--danger) 6%, var(--white));
  }
  .struk-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px,1fr)); gap: 0.7rem; }
  .block-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px,1fr)); gap: 0.4rem; margin-top: 0.4rem; }
  .block-cb { margin: 0; gap: 0.35rem; font-weight: 400; font-size: 0.85rem; }
  .btn-warning { background: var(--warning); border-color: var(--warning); color: #fff; }
  .btn-warning:hover { background: #c98a2a; border-color: #c98a2a; }

  .theme-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px,1fr)); gap: 0.7rem; margin-top: 0.8rem; }
  .theme-card {
    position: relative;
    display: flex; flex-direction: column; align-items: center; gap: 0.5rem;
    padding: 1rem 0.8rem;
    border: 2px solid var(--border);
    border-radius: var(--radius);
    background: var(--panel);
  }
  .theme-card.active { border-color: var(--primary); }
  .theme-swatch { width: 40px; height: 40px; border-radius: 50%; border: 1px solid rgba(0,0,0,0.1); }
  .theme-name { font-size: 0.85rem; font-weight: 600; text-align: center; }
  .theme-check {
    position: absolute; top: 0.4rem; right: 0.5rem;
    color: var(--primary); font-weight: 700;
  }

  .scanmode-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px,1fr)); gap: 0.8rem; margin-top: 0.8rem; }
  .scanmode-card {
    position: relative;
    display: flex; flex-direction: column; align-items: flex-start; gap: 0.35rem;
    text-align: left;
    padding: 1rem;
    border: 2px solid var(--border);
    border-radius: var(--radius);
    background: var(--panel);
  }
  .scanmode-card.active { border-color: var(--primary); }
  .scanmode-name { font-size: 0.92rem; font-weight: 700; }
</style>
