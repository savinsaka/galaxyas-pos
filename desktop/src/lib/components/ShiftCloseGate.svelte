<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api } from "$lib/api";
  import { toastError, showToast } from "$lib/toast";
  import { activeShiftStore } from "$lib/stores/shift";

  let showGate = $state(false);
  let step = $state<"form" | "confirm-quit">("form");
  let closingCash = $state(0);
  let closeNote = $state("");
  let busy = $state(false);

  onMount(() => {
    // Snapshot awal shift aktif saat app boot — supaya guard tahu statusnya
    // walau tab Kasir/Shift belum pernah dibuka sesi ini. Ini IPC call biasa
    // di titik mount, BUKAN di dalam handler onCloseRequested (lihat di bawah).
    api.getActiveShift().then((s) => activeShiftStore.set(s)).catch(() => {});

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
        if (get(activeShiftStore)) {
          event.preventDefault();
          closingCash = 0;
          closeNote = "";
          step = "form";
          showGate = true;
        }
      })
      .then((fn) => (unlisten = fn));
    return () => unlisten?.();
  });

  async function doCloseShift() {
    const shift = get(activeShiftStore);
    if (!shift) return;
    busy = true;
    try {
      await api.closeShift({ id: shift.id, closing_cash: closingCash, note: closeNote || null });
      activeShiftStore.set(null);
      showToast("Shift ditutup.", "success");
      step = "confirm-quit";
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
        <h2>🔒 Tutup Shift Dulu</h2>
        <p class="text-dim" style="margin-top:0; font-size:0.83rem;">
          Ada shift yang masih berjalan. Tutup shift dulu sebelum menutup aplikasi.
        </p>
        <label>Uang Fisik di Laci Sekarang (Rp)</label>
        <input type="number" min="0" bind:value={closingCash} />
        <label style="margin-top:0.6rem;">Catatan</label>
        <input bind:value={closeNote} placeholder="opsional" />
        <div class="row" style="justify-content:flex-end; margin-top:1rem; gap:0.5rem;">
          <button disabled={busy} onclick={keepUsing}>Batal, Lanjut Pakai</button>
          <button class="btn-danger" disabled={busy} onclick={doCloseShift}>🔒 Tutup Shift</button>
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
  .shift-gate-modal { max-width: 380px; }
</style>
