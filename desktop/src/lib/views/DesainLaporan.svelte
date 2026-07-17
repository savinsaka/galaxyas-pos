<script lang="ts">
  import { onMount } from "svelte";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import BlockDesigner from "$lib/components/BlockDesigner.svelte";
  import {
    REPORT_TYPES,
    defaultConfig,
    loadReportDesign,
    saveReportDesign,
    type ReportDesignConfig,
  } from "$lib/reportDesign";

  let activeType = $state(REPORT_TYPES[0].key);
  let config = $state<ReportDesignConfig>(defaultConfig(REPORT_TYPES[0].blocks));
  let loading = $state(false);
  let saving = $state(false);

  const activeDef = $derived(REPORT_TYPES.find((t) => t.key === activeType)!);

  async function load() {
    loading = true;
    try {
      config = await loadReportDesign(activeType, activeDef.blocks);
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }
  onMount(load);
  $effect(() => {
    activeType;
    load();
  });

  async function save() {
    saving = true;
    try {
      await saveReportDesign(activeType, config);
      showToast("Desain laporan tersimpan — berlaku saat laporan ini dicetak berikutnya.", "success");
    } catch (e) {
      toastError(e);
    } finally {
      saving = false;
    }
  }

  function resetDefault() {
    config = defaultConfig(activeDef.blocks);
  }
</script>

<div class="page-head"><h1>Desain Laporan</h1></div>

{#if $currentUser?.role !== "admin"}
  <div class="card text-dim" style="text-align:center; padding:2rem;">Hanya admin yang bisa mengubah desain laporan.</div>
{:else}
  <p class="text-dim" style="margin-top:0; max-width:640px;">
    Pilih jenis laporan, lalu seret ⠿⠿ untuk mengurutkan bagian dan klik 👁️/🚫 untuk memilih bagian mana yang
    ikut tercetak. Perubahan berlaku begitu disimpan — laporan yang tidak punya banyak bagian (Laporan Kasir
    Detail, Detail/Per Hari Item) tidak ada di sini karena isinya cuma satu blok.
  </p>

  <div class="card" style="max-width:720px;">
    <label>Jenis Laporan</label>
    <select bind:value={activeType}>
      {#each REPORT_TYPES as t}<option value={t.key}>{t.label}</option>{/each}
    </select>

    {#if loading}
      <div class="text-dim" style="padding:1rem 0;">Memuat…</div>
    {:else}
      <div style="margin-top:1rem;">
        <BlockDesigner blocks={activeDef.blocks} {config} />
      </div>
    {/if}

    <div class="row" style="justify-content:flex-end; margin-top:1rem; gap:0.5rem;">
      <button class="btn-ghost" onclick={resetDefault} disabled={loading}>↺ Reset ke Default</button>
      <button class="btn-primary" onclick={save} disabled={loading || saving}>💾 Simpan Desain</button>
    </div>
  </div>
{/if}
