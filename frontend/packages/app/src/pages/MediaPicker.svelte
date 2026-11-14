<script lang="ts">
  import { onMount } from "svelte";
  import { rpc, type PreviewFile, type ScanRecord } from "../api";

  export let record: ScanRecord;
  export let csrf: string;
  export let onchange: (path: string, value: unknown) => void;

  let files: PreviewFile[] = [];
  let previews: Record<string, string> = {};
  let busy = false;
  let error = "";
  let uploadInput: HTMLInputElement | null = null;
  let selected = "";

  onMount(load);

  async function load() {
    busy = true;
    error = "";
    try {
      const result = await rpc<{ files: PreviewFile[] }>(
        "preview.list",
        { path: record.path },
        csrf,
      );
      files = result.files;
      selected = String(record.value ?? "");
      const nextPreviews: Record<string, string> = {};
      for (const file of files
        .filter((candidate) => /\.(png|jpe?g|webp|gif|svg)$/i.test(candidate.name))
        .slice(0, 12)) {
        try {
          const preview = await rpc<{ mime: string; data: string }>(
            "preview.read",
            { path: file.path, option: record.path },
            csrf,
          );
          nextPreviews[file.path] = `data:${preview.mime};base64,${preview.data}`;
        } catch {
          // Keep the file without a preview.
        }
      }
      previews = nextPreviews;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot list media";
    } finally {
      busy = false;
    }
  }

  function choose(file: PreviewFile) {
    selected = file.value;
    onchange(record.path, file.value);
  }

  async function upload() {
    const file = uploadInput?.files?.[0];
    if (!file) {
      error = "Choose a file first";
      return;
    }
    if (file.size > 32 * 1024 * 1024) {
      error = "The file is larger than 32 MiB";
      return;
    }
    busy = true;
    error = "";
    try {
      const buffer = await file.arrayBuffer();
      const result = await rpc<PreviewFile>(
        "preview.upload",
        { path: record.path, name: file.name, data: toBase64(buffer) },
        csrf,
      );
      if (uploadInput) uploadInput.value = "";
      selected = result.value;
      onchange(record.path, result.value);
      await load();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot upload the file";
    } finally {
      busy = false;
    }
  }

  async function remove(file: PreviewFile) {
    if (!confirm(`Delete ${file.name}?`)) return;
    busy = true;
    error = "";
    try {
      await rpc(
        "preview.delete",
        { path: record.path, name: file.name },
        csrf,
      );
      await load();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot delete the file";
    } finally {
      busy = false;
    }
  }

  function toBase64(buffer: ArrayBuffer): string {
    const bytes = new Uint8Array(buffer);
    let binary = "";
    const chunk = 0x8000;
    for (let index = 0; index < bytes.length; index += chunk) {
      binary += String.fromCharCode(...bytes.subarray(index, index + chunk));
    }
    return btoa(binary);
  }
</script>

<div class="media-picker">
  <div class="toolbar">
    <span class="muted small">{files.length} file(s)</span>
    <span class="spacer"></span>
    <label class="button-like">
      Choose file
      <input
        type="file"
        bind:this={uploadInput}
        onchange={upload}
        disabled={busy}
        hidden
      />
    </label>
    <button class="ghost" onclick={load} disabled={busy}>
      {busy ? "Loading..." : "Refresh"}
    </button>
  </div>
  {#if error}
    <p class="error">{error}</p>
  {/if}
  <div class="media-grid">
    {#each files as file (file.path)}
      <div class="media-entry">
        <button
          class="media-item"
          class:selected={selected === file.value}
          type="button"
          onclick={() => choose(file)}
        >
          {#if previews[file.path]}
            <img src={previews[file.path]} alt={file.name} />
          {:else}
            <span class="media-placeholder">file</span>
          {/if}
          <span class="media-name">{file.name}</span>
        </button>
        <button
          class="ghost tiny media-delete"
          type="button"
          disabled={busy}
          onclick={() => remove(file)}
        >
          Delete
        </button>
      </div>
    {/each}
  </div>
  {#if files.length === 0 && !busy}
    <p class="muted small">
      This directory has no accepted files yet. Upload one to stage it.
    </p>
  {/if}
</div>
