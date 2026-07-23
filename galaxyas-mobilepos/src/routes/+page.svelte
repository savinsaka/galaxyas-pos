<script lang="ts">
  // Shell sementara Phase 0: membuktikan webview + backend Rust jalan di
  // Android. Shell asli (bottom nav + gate onboarding) menyusul di Phase 1.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  let backendStatus = $state("memeriksa backend…");

  onMount(async () => {
    document.getElementById("boot-splash")?.remove();
    try {
      const settings = await invoke<Record<string, string>>("get_settings");
      backendStatus = `backend OK (${Object.keys(settings).length} setting tersimpan)`;
    } catch (e) {
      backendStatus = `backend error: ${e}`;
    }
  });
</script>

<main>
  <h1>GALAX<b>YAS</b> POS</h1>
  <p class="sub">Mobile · Phase 0 scaffold</p>
  <p class="status">{backendStatus}</p>
</main>

<style>
  main {
    min-height: 100dvh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    background: #f2f9ff;
    font-family: -apple-system, "Segoe UI", Roboto, Arial, sans-serif;
    padding: env(safe-area-inset-top) env(safe-area-inset-right)
      env(safe-area-inset-bottom) env(safe-area-inset-left);
  }
  h1 {
    margin: 0;
    font-size: 1.8rem;
    font-weight: 850;
    letter-spacing: 0.04em;
    color: #2f6aa8;
  }
  h1 b {
    color: #4a90d9;
  }
  .sub {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    letter-spacing: 0.18em;
    color: #7fb2dd;
    text-transform: uppercase;
  }
  .status {
    margin-top: 1rem;
    font-size: 0.9rem;
    color: #456;
  }
</style>
