<script lang="ts">
  import { onMount } from "svelte";
  import { showToast, toastError } from "$lib/toast";
  import { currentUser } from "$lib/stores/auth";
  import CanvasEditor from "$lib/components/CanvasEditor.svelte";
  import { REPORT_KINDS, type ReportKind } from "$lib/report/kinds";
  import { defaultTemplate } from "$lib/report/defaults";
  import {
    listTemplates,
    readTemplate,
    saveTemplate,
    deleteTemplate,
    duplicateTemplate,
    renameTemplate,
    ensureDefault,
    getActiveTemplateName,
    setActiveTemplateName,
  } from "$lib/report/storage";
  import type { ReportTemplate } from "$lib/report/template";

  const CATEGORY_LABEL: Record<string, string> = {
    kasir: "Kasir (Struk & Dokumen)",
    penjualan: "Penjualan",
    item: "Item",
    persediaan: "Persediaan",
    umum: "Umum",
  };

  // Kelompokkan jenis laporan per kategori untuk panel kiri.
  const GROUPS = Object.entries(
    REPORT_KINDS.reduce<Record<string, ReportKind[]>>((acc, k) => {
      (acc[k.category] ??= []).push(k);
      return acc;
    }, {}),
  ).map(([cat, kinds]) => ({ cat, label: CATEGORY_LABEL[cat] ?? cat, kinds }));

  let kindId = $state(REPORT_KINDS[0].id);
  let names = $state<string[]>([]);
  let activeName = $state<string | null>(null);
  let loading = $state(false);
  let editing = $state<ReportTemplate | null>(null);
  let dialog = $state<{ mode: "new" | "dup" | "rename"; target: string; value: string } | null>(null);

  const kind = $derived(REPORT_KINDS.find((k) => k.id === kindId)!);

  async function refresh() {
    loading = true;
    try {
      await ensureDefault(kindId);
      names = await listTemplates(kindId);
      activeName = await getActiveTemplateName(kindId);
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }
  onMount(refresh);
  $effect(() => {
    kindId;
    refresh();
  });

  async function openEditor(name: string) {
    try {
      editing = await readTemplate(kindId, name);
    } catch (e) {
      toastError(e);
    }
  }

  async function onSave(t: ReportTemplate) {
    try {
      await saveTemplate(t);
      editing = t;
      showToast(`Template "${t.name}" tersimpan.`, "success");
      await refresh();
    } catch (e) {
      toastError(e);
    }
  }

  async function makeActive(name: string) {
    try {
      await setActiveTemplateName(kindId, name);
      activeName = name;
      showToast(`"${name}" jadi template aktif untuk ${kind.label}.`, "success");
    } catch (e) {
      toastError(e);
    }
  }

  async function removeOne(name: string) {
    if (!confirm(`Hapus template "${name}"? Tidak bisa dibatalkan.`)) return;
    try {
      await deleteTemplate(kindId, name);
      await refresh();
      showToast("Template dihapus.", "success");
    } catch (e) {
      toastError(e);
    }
  }

  function askName(mode: "new" | "dup" | "rename", target = "") {
    dialog = {
      mode,
      target,
      value: mode === "rename" ? target : mode === "dup" ? `Salinan ${target}` : "Template Baru",
    };
  }

  async function confirmDialog() {
    if (!dialog) return;
    const name = dialog.value.trim();
    if (!name) return;
    try {
      if (dialog.mode === "new") {
        const base = defaultTemplate(kindId);
        const fresh: ReportTemplate = base
          ? { ...structuredClone(base), name }
          : { greport: 2, kind: kindId, name, paper: { preset: "a4", widthMm: 210, heightMm: 297, marginMm: 15, fontPt: 10.5 }, elements: [] };
        await saveTemplate(fresh);
        await refresh();
        editing = fresh;
      } else if (dialog.mode === "dup") {
        await duplicateTemplate(kindId, dialog.target, name);
        await refresh();
      } else {
        await renameTemplate(kindId, dialog.target, name);
        if (activeName === dialog.target) await setActiveTemplateName(kindId, name);
        await refresh();
      }
      dialog = null;
    } catch (e) {
      toastError(e);
    }
  }
</script>

{#if $currentUser?.role !== "admin"}
  <div class="card text-dim" style="text-align:center; padding:2rem;">Hanya admin yang bisa mengubah template laporan.</div>
{:else if editing}
  <div class="editor-shell">
    <div class="crumb">
      <button class="btn-ghost" onclick={() => (editing = null)}>← Daftar Template</button>
      <span class="text-dim">{kind.label} › {editing.name}</span>
    </div>
    <CanvasEditor template={editing} {onSave} onClose={() => (editing = null)} />
  </div>
{:else}
  <div class="page-head"><h1>Editor Laporan</h1></div>
  <div class="launcher">
    <!-- Panel kiri: jenis laporan -->
    <div class="card pane types">
      <div class="pane-title">Jenis Laporan</div>
      {#each GROUPS as g}
        <div class="grp-label">{g.label}</div>
        {#each g.kinds as k}
          <button class="type-row" class:sel={k.id === kindId} onclick={() => (kindId = k.id)}>
            {k.label}
          </button>
        {/each}
      {/each}
    </div>

    <!-- Panel kanan: template dari jenis terpilih -->
    <div class="card pane templates">
      <div class="pane-head">
        <div class="pane-title">Template — {kind.label}</div>
        <button class="btn-primary" onclick={() => askName("new")}>➕ Baru</button>
      </div>
      {#if loading}
        <div class="text-dim" style="padding:1rem;">Memuat…</div>
      {:else if names.length === 0}
        <div class="text-dim" style="padding:1rem;">Belum ada template. Klik “Baru” untuk membuat.</div>
      {:else}
        {#each names as n}
          <div class="tpl-row">
            <span class="tpl-name">{n}{#if n === activeName}<span class="active-badge">✓ aktif</span>{/if}</span>
            <div class="tpl-actions">
              <button class="btn-ghost mini" onclick={() => openEditor(n)}>✏️ Edit</button>
              <button class="btn-ghost mini" onclick={() => askName("dup", n)}>⧉ Duplikat</button>
              <button class="btn-ghost mini" onclick={() => askName("rename", n)}>✎ Rename</button>
              <button class="btn-ghost mini" disabled={n === activeName} onclick={() => makeActive(n)}>⭐ Aktifkan</button>
              <button class="btn-ghost mini danger" onclick={() => removeOne(n)}>🗑️</button>
            </div>
          </div>
        {/each}
      {/if}
    </div>
  </div>

  {#if dialog}
    <div class="modal-backdrop" role="presentation" onclick={() => (dialog = null)}>
      <div class="card name-dialog" role="presentation" onclick={(e) => e.stopPropagation()}>
        <div class="pane-title">
          {dialog.mode === "new" ? "Nama template baru" : dialog.mode === "dup" ? "Nama salinan" : "Nama baru"}
        </div>
        <!-- svelte-ignore a11y_autofocus -->
        <input bind:value={dialog.value} autofocus onkeydown={(e) => e.key === "Enter" && confirmDialog()} />
        <div class="row" style="justify-content:flex-end; gap:0.5rem;">
          <button class="btn-ghost" onclick={() => (dialog = null)}>Batal</button>
          <button class="btn-primary" onclick={confirmDialog}>OK</button>
        </div>
      </div>
    </div>
  {/if}
{/if}

<style>
  .launcher { display: grid; grid-template-columns: 280px 1fr; gap: 1rem; align-items: start; }
  @media (max-width: 900px) { .launcher { grid-template-columns: 1fr; } }
  .pane-title { font-weight: 700; }
  .grp-label { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.03em; color: var(--text-dim); margin: 0.7rem 0 0.3rem; }
  .type-row { display: block; width: 100%; text-align: left; padding: 0.45rem 0.6rem; border: 1px solid transparent; border-radius: var(--radius); background: none; cursor: pointer; font-size: 0.9rem; }
  .type-row:hover { background: var(--baby-blue-bg); }
  .type-row.sel { background: var(--baby-blue-bg); border-color: var(--baby-blue, #4a86e8); font-weight: 600; }

  .pane-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.6rem; }
  .tpl-row { display: flex; justify-content: space-between; align-items: center; gap: 0.6rem; padding: 0.5rem 0.6rem; border: 1px solid var(--border); border-radius: var(--radius); margin-bottom: 0.4rem; flex-wrap: wrap; }
  .tpl-name { font-weight: 600; }
  .active-badge { margin-left: 0.5rem; font-size: 0.72rem; color: #2e7d32; font-weight: 600; }
  .tpl-actions { display: flex; gap: 0.3rem; flex-wrap: wrap; }
  .btn-ghost.mini { font-size: 0.75rem; padding: 0.25rem 0.5rem; }
  .btn-ghost.danger { color: #b42318; }

  .editor-shell { display: flex; flex-direction: column; gap: 0.5rem; }
  .crumb { display: flex; align-items: center; gap: 0.8rem; }

  .modal-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.35); display: flex; align-items: center; justify-content: center; z-index: 50; }
  .name-dialog { min-width: 320px; display: flex; flex-direction: column; gap: 0.7rem; }
</style>
