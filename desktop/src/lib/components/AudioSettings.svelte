<script lang="ts">
  // Pengaturan Audio Voice Call: pilih mikrofon & speaker, tes keduanya, dan
  // pemrosesan suara. Berlaku langsung, termasuk ke panggilan yang sedang jalan.
  import { onDestroy, onMount } from "svelte";
  import { showToast, toastError } from "$lib/toast";
  import { audioPrefs, loadAudioPrefs, saveAudioPrefs, micConstraints, applySink, type AudioPrefs } from "$lib/stores/audioPrefs";
  import { applyAudioToCall } from "$lib/stores/call";

  type Sinkable = { setSinkId?: (id: string) => Promise<void> };

  let mics = $state<MediaDeviceInfo[]>([]);
  let speakers = $state<MediaDeviceInfo[]>([]);
  let needPermission = $state(false);
  let level = $state(0);
  let testingMic = $state(false);

  let testStream: MediaStream | null = null;
  let testCtx: AudioContext | null = null;
  let raf = 0;

  // Speaker tidak bisa dipilih kalau WebView tidak mendukung setSinkId.
  const canPickSpeaker = typeof HTMLMediaElement !== "undefined" && "setSinkId" in HTMLMediaElement.prototype;

  async function refreshDevices() {
    try {
      const all = await navigator.mediaDevices.enumerateDevices();
      mics = all.filter((d) => d.kind === "audioinput" && d.deviceId !== "communications");
      speakers = all.filter((d) => d.kind === "audiooutput" && d.deviceId !== "communications");
      // Nama perangkat baru terlihat setelah izin mikrofon diberikan.
      needPermission = mics.length > 0 && mics.every((d) => !d.label);
    } catch (e) {
      toastError(e);
    }
  }

  async function askPermission() {
    try {
      const s = await navigator.mediaDevices.getUserMedia({ audio: true });
      s.getTracks().forEach((t) => t.stop());
      await refreshDevices();
    } catch {
      showToast("Izin mikrofon ditolak atau mikrofon tidak ada.", "error", 5000);
    }
  }

  function label(d: MediaDeviceInfo, i: number, kind: string): string {
    if (d.deviceId === "default") return `Default Windows${d.label ? ` — ${d.label.replace(/^Default - /, "")}` : ""}`;
    return d.label || `${kind} ${i + 1}`;
  }

  async function update(patch: Partial<AudioPrefs>) {
    try {
      await saveAudioPrefs(patch);
      await applyAudioToCall();
      if (testingMic && ("micId" in patch || "echoCancellation" in patch || "noiseSuppression" in patch || "autoGainControl" in patch)) {
        stopMicTest();
        await startMicTest();
      }
    } catch (e) {
      toastError(e);
    }
  }

  function onSelect(key: "micId" | "speakerId", e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value;
    update({ [key]: v === "default" ? "" : v });
  }

  async function startMicTest() {
    try {
      testStream = await navigator.mediaDevices.getUserMedia(micConstraints($audioPrefs));
    } catch {
      showToast("Mikrofon pilihan tidak bisa dibuka.", "error");
      return;
    }
    testCtx = new AudioContext();
    const analyser = testCtx.createAnalyser();
    analyser.fftSize = 1024;
    testCtx.createMediaStreamSource(testStream).connect(analyser);
    const buf = new Float32Array(analyser.fftSize);
    testingMic = true;
    const tick = () => {
      analyser.getFloatTimeDomainData(buf);
      let sum = 0;
      for (const v of buf) sum += v * v;
      // RMS → 0..1 (dikali supaya suara bicara biasa mengisi ±setengah bar).
      level = Math.min(1, Math.sqrt(sum / buf.length) * 6);
      raf = requestAnimationFrame(tick);
    };
    tick();
  }

  function stopMicTest() {
    cancelAnimationFrame(raf);
    testStream?.getTracks().forEach((t) => t.stop());
    testStream = null;
    testCtx?.close().catch(() => {});
    testCtx = null;
    testingMic = false;
    level = 0;
  }

  async function testSpeaker() {
    const ac = new AudioContext();
    await applySink(ac as unknown as Sinkable, $audioPrefs.speakerId);
    const t = ac.currentTime + 0.05;
    for (const [i, f] of [523.3, 659.3, 784].entries()) {
      const osc = ac.createOscillator();
      const gain = ac.createGain();
      osc.frequency.value = f;
      gain.gain.setValueAtTime(0.0001, t + i * 0.25);
      gain.gain.exponentialRampToValueAtTime(0.25, t + i * 0.25 + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.0001, t + i * 0.25 + 0.22);
      osc.connect(gain).connect(ac.destination);
      osc.start(t + i * 0.25);
      osc.stop(t + i * 0.25 + 0.25);
    }
    setTimeout(() => ac.close().catch(() => {}), 1200);
  }

  onMount(async () => {
    await loadAudioPrefs();
    await refreshDevices();
    navigator.mediaDevices.addEventListener("devicechange", refreshDevices);
  });
  onDestroy(() => {
    stopMicTest();
    navigator.mediaDevices.removeEventListener("devicechange", refreshDevices);
  });
</script>

<div class="card">
  <h2>🎧 Pengaturan Audio</h2>
  {#if needPermission}
    <div class="perm">
      Nama perangkat belum terlihat.
      <button class="btn-ghost small" onclick={askPermission}>Izinkan mikrofon</button>
    </div>
  {/if}

  <label for="as-mic">Mikrofon (input)</label>
  <div class="row-line">
    <select id="as-mic" value={$audioPrefs.micId || "default"} onchange={(e) => onSelect("micId", e)}>
      {#if !mics.some((d) => d.deviceId === "default")}<option value="default">Default Windows</option>{/if}
      {#each mics as d, i (d.deviceId)}
        <option value={d.deviceId}>{label(d, i, "Mikrofon")}</option>
      {/each}
    </select>
    <button class="btn-ghost small" onclick={() => (testingMic ? stopMicTest() : startMicTest())}>
      {testingMic ? "■ Stop" : "🎙️ Tes mic"}
    </button>
  </div>
  <div class="meter" class:on={testingMic} title="Level suara mikrofon">
    <div class="meter-fill" style="width:{Math.round(level * 100)}%"></div>
  </div>
  {#if testingMic}<div class="hint">Bicaralah — bar hijau harus bergerak.</div>{/if}

  <label for="as-spk" style="margin-top:0.8rem;">Speaker / headset (output)</label>
  <div class="row-line">
    <select id="as-spk" value={$audioPrefs.speakerId || "default"} onchange={(e) => onSelect("speakerId", e)} disabled={!canPickSpeaker}>
      {#if !speakers.some((d) => d.deviceId === "default")}<option value="default">Default Windows</option>{/if}
      {#each speakers as d, i (d.deviceId)}
        <option value={d.deviceId}>{label(d, i, "Speaker")}</option>
      {/each}
    </select>
    <button class="btn-ghost small" onclick={testSpeaker}>🔊 Tes speaker</button>
  </div>
  {#if !canPickSpeaker}<div class="hint">Versi WebView ini belum bisa memilih speaker — memakai default Windows.</div>{/if}

  <div class="opts">
    <label class="check">
      <input type="checkbox" checked={$audioPrefs.echoCancellation} onchange={(e) => update({ echoCancellation: e.currentTarget.checked })} />
      Peredam gema (wajib kalau pakai speaker, bukan headset)
    </label>
    <label class="check">
      <input type="checkbox" checked={$audioPrefs.noiseSuppression} onchange={(e) => update({ noiseSuppression: e.currentTarget.checked })} />
      Peredam bising (suara toko/kipas)
    </label>
    <label class="check">
      <input type="checkbox" checked={$audioPrefs.autoGainControl} onchange={(e) => update({ autoGainControl: e.currentTarget.checked })} />
      Atur volume mic otomatis
    </label>
  </div>
  <div class="hint">Tersimpan di PC ini dan langsung berlaku, juga saat sedang menelepon.</div>
</div>

<style>
  .perm {
    display: flex; align-items: center; justify-content: space-between; gap: 0.5rem;
    background: color-mix(in srgb, var(--warning) 16%, var(--white));
    border-radius: 8px; padding: 0.45rem 0.7rem; margin-bottom: 0.7rem; font-size: 0.85rem;
  }
  .row-line { display: flex; gap: 0.5rem; align-items: center; }
  .row-line select { flex: 1; min-width: 0; }
  .small { padding: 0.35rem 0.7rem; font-size: 0.8rem; white-space: nowrap; }
  .meter { height: 8px; border-radius: 999px; background: var(--baby-blue-bg); border: 1px solid var(--border); overflow: hidden; margin-top: 0.45rem; opacity: 0.5; }
  .meter.on { opacity: 1; }
  .meter-fill { height: 100%; background: var(--success); transition: width 0.06s linear; }
  .hint { font-size: 0.75rem; color: var(--text-dim); margin-top: 0.3rem; }
  .opts { margin-top: 0.9rem; display: flex; flex-direction: column; gap: 0.35rem; }
  /* `input { width:100% }` global ikut kena checkbox — kembalikan ke ukuran asli. */
  .check { display: flex; align-items: center; gap: 0.45rem; margin: 0; font-weight: 400; font-size: 0.85rem; }
  .check input { width: auto; flex: none; }
</style>
