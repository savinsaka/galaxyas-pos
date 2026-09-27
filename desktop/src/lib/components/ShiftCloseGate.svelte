<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api } from "$lib/api";
  import { toastError, showToast } from "$lib/toast";
  import { activeShiftStore } from "$lib/stores/shift";
  import { closingSopStore, loadClosingSop, sopActive } from "$lib/stores/closingSop";

  let showGate = $state(false);
  let step = $state<"form" | "confirm-quit">("form");
  // Tiga mode gate (dikunci saat gate dibuka):
  //  - SOP off + ada shift → form Tutup Shift biasa, lalu konfirmasi keluar.
  //  - SOP on  + ada shift → SATU popup: ceklis SOP + nominal tutup shift,
  //    satu tombol yang menutup shift lalu langsung menutup aplikasi.
  //  - SOP on  + tanpa shift → hanya ceklis SOP.
  let gateShift = $state(false);
  let gateSop = $state(false);
  // Snapshot daftar SOP saat gate dibuka supaya perubahan pengaturan di
  // tengah jalan tidak mengacak ceklis.
  let sopItems = $state<string[]>([]);
  let sopChecked = $state<boolean[]>([]);
  let sopDone = $derived(sopChecked.length > 0 && sopChecked.every(Boolean));
  let closingCash = $state(0);
  let closeNote = $state("");
  let busy = $state(false);

  onMount(() => {
    // Snapshot awal shift aktif saat app boot — supaya guard tahu statusnya
    // walau tab Kasir/Shift belum pernah dibuka sesi ini. Ini IPC call biasa
    // di titik mount, BUKAN di dalam handler onCloseRequested (lihat di bawah).
    api.getActiveShift().then((s) => activeShiftStore.set(s)).catch(() => {});
    loadClosingSop().catch(() => {});

    let unlisten: (() => void) | undefined;
    const win = getCurrentWindow();
    win
      .onCloseRequested((event) => {
        // PENTING: JANGAN await invoke() apa pun di sini. Kalau ini nunggu
        // Tauri command, event loop webview2 bisa freeze menunggu satu sama
        // lain (persis kasus open_print_window yang WAJIB async di
        // commands.rs) — akibatnya aplikasi sama sekali tidak bisa ditutup,
        // bahkan setelah shift sudah ditutup dari sisi lain. Makanya status
        // shift dibaca SINKRON dari store lokal (activeShiftStore), bukan
        // panggil api.getActiveShift() lagi di titik ini.
        const hasShift = !!get(activeShiftStore);
        const sop = get(closingSopStore);
        if (hasShift || sopActive(sop)) {
          event.preventDefault();
          closingCash = 0;
          closeNote = "";
          gateShift = hasShift;
          gateSop = sopActive(sop);
          sopItems = [...sop.items];
          sopChecked = sop.items.map(() => false);
          step = "form";
          showGate = true;
        }
      })
      .then((fn) => (unlisten = fn));
    return () => unlisten?.();
  });

  async function doCloseShift() {
    if (gateSop && !sopDone) return;
    const shift = get(activeShiftStore);
    busy = true;
    try {
      if (shift) {
        await api.closeShift({ id: shift.id, closing_cash: closingCash, note: closeNote || null });
        activeShiftStore.set(null);
        showToast("Shift ditutup.", "success");
      }
      // Dengan Closing SOP semuanya sudah dikonfirmasi lewat ceklis — langsung keluar.
      if (gateSop) await quitNow();
      else step = "confirm-quit";
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  function keepUsing() {
    showGate = false;
  }

  async function quitNow() {
    try {
      await getCurrentWindow().destroy();
    } catch (e) {
      toastError(e);
    }
  }
</script>

{#if showGate}
  <div class="modal-backdrop" role="presentation">
    <div class="modal shift-gate-modal" role="presentation">
      {#if step === "form"}
        {#if gateSop}
          <h2>📋 Closing SOP</h2>
          <p class="text-dim" style="margin-top:0; font-size:0.83rem;">
            Ceklis semua tugas penutupan{gateShift ? " dan isi uang laci" : ""} sebelum menutup aplikasi.
          </p>
          <div class="sop-list">
            {#each sopItems as item, i (i)}
              <label class="sop-item" class:done={sopChecked[i]}>
                <input type="checkbox" bind:checked={sopChecked[i]} />
                <span class="sop-text">{item}</span>
              </label>
            {/each}
          </div>
          <p class="text-dim" style="font-size:0.8rem; margin:0.4rem 0 0;">
            {sopChecked.filter(Boolean).length} / {sopItems.length} selesai
          </p>
        {:else}
          <h2>🔒 Tutup Shift Dulu</h2>
          <p class="text-dim" style="margin-top:0; font-size:0.83rem;">
            Ada shift yang masih berjalan. Tutup shift dulu sebelum menutup aplikasi.
          </p>
        {/if}
        {#if gateShift}
          {#if gateSop}<h3 class="shift-sub">🔒 Tutup Shift</h3>{/if}
          <label>Uang Fisik di Laci Sekarang (Rp)</label>
          <input type="number" min="0" bind:value={closingCash} />
          <label style="margin-top:0.6rem;">Catatan</label>
          <input bind:value={closeNote} placeholder="opsional" />
        {/if}
        <div class="row" style="justify-content:flex-end; margin-top:1rem; gap:0.5rem;">
          <button disabled={busy} onclick={keepUsing}>Batal, Lanjut Pakai</button>
          {#if !gateSop}
            <button class="btn-danger" disabled={busy} onclick={doCloseShift}>🔒 Tutup Shift</button>
          {:else}
            <button class="btn-primary" disabled={busy || !sopDone} onclick={doCloseShift}>
              {gateShift ? "🔒 Tutup Shift & Aplikasi" : "✅ Tutup Aplikasi"}
            </button>
          {/if}
        </div>
      {:else}
        <h2>✅ Shift Ditutup</h2>
        <p class="text-dim" style="margin-top:0; font-size:0.83rem;">Tutup aplikasi sekarang?</p>
        <div class="row" style="justify-content:flex-end; margin-top:1rem; gap:0.5rem;">
          <button onclick={keepUsing}>Tidak, Lanjut Pakai</button>
          <button class="btn-primary" onclick={quitNow}>Ya, Tutup Aplikasi</button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .shift-gate-modal { max-width: 420px; }
  .shift-sub { font-size: 0.95rem; margin: 1rem 0 0.4rem; padding-top: 0.8rem; border-top: 1px solid var(--border); }
  .sop-list { display: flex; flex-direction: column; gap: 0.4rem; max-height: 50vh; overflow-y: auto; }
  .sop-item {
    display: flex; align-items: center; gap: 0.65rem; margin: 0;
    padding: 0.55rem 0.75rem; border: 1px solid var(--border); border-radius: var(--radius);
    font-size: 0.9rem; font-weight: 600; color: var(--text); cursor: pointer;
    transition: background 0.12s, border-color 0.12s;
  }
  .sop-item:hover { border-color: var(--primary); }
  /* Global `input { width:100% }` ikut kena checkbox — kembalikan ke ukuran kotak. */
  .sop-item input[type="checkbox"] {
    width: 18px; height: 18px; flex: none; margin: 0; padding: 0;
    accent-color: var(--success); cursor: pointer; box-shadow: none;
  }
  .sop-text { flex: 1; text-align: left; line-height: 1.3; }
  .sop-item.done { border-color: var(--success); background: color-mix(in srgb, var(--success) 10%, var(--white)); }
  .sop-item.done .sop-text { text-decoration: line-through; color: var(--text-dim); }
</style>
