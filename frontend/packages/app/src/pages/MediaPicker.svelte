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
      for (const file of files
        .filter((candidate) => /\.(png|jpe?g|webp|gif|svg)$/i.test(candidate.name))
        .slice(0, 8)) {
        try {
          const preview = await rpc<{ mime: string; data: string }>(
            "preview.read",
            { path: file.path },
            csrf,
          );
          previews = {
            ...previews,
            [file.path]: `data:${preview.mime};base64,${preview.data}`,
          };
        } catch {
          // Keep the file without a preview.
        }
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot list media";
    } finally {
      busy = false;
    }
  }

  function choose(file: PreviewFile) {
    onchange(record.path, file.value);
  }
</script>

<div class="media-picker">
  <div class="toolbar">
    <span class="muted small">{files.length} file(s)</span>
    <span class="spacer"></span>
    <button class="ghost" onclick={load} disabled={busy}>
      {busy ? "Loading..." : "Refresh"}
    </button>
  </div>
  {#if error}
    <p class="error">{error}</p>
  {/if}
  <div class="media-grid">
    {#each files as file (file.path)}
      <button class="media-item" type="button" onclick={() => choose(file)}>
        {#if previews[file.path]}
          <img src={previews[file.path]} alt={file.name} />
        {:else}
          <span class="media-placeholder">file</span>
        {/if}
        <span class="media-name">{file.name}</span>
      </button>
    {/each}
  </div>
</div>
