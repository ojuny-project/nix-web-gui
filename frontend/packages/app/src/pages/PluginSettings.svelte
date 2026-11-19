<script lang="ts">
  import { onMount } from "svelte";
  import type { ControlValue } from "@nix-web-gui/ui";
  import {
    rpc,
    type PluginSchemaField,
    type ScanRecord,
  } from "../api";

  export let plugin: ScanRecord;
  export let allRecords: ScanRecord[];
  export let values: Record<string, ControlValue>;
  export let csrf: string;
  export let onchange: (path: string, value: ControlValue) => void;
  export let onstructurechange: () => Promise<void>;

  let fields: PluginSchemaField[] = [];
  let error = "";
  let busy = false;

  $: pluginId = plugin.path.split(".").at(-1) ?? "";
  $: settingsPath = `${plugin.path}.settings`;

  onMount(load);

  async function load() {
    busy = true;
    error = "";
    try {
      fields = await rpc<PluginSchemaField[]>(
        "plugins.schema",
        { id: pluginId },
        csrf,
      );
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot read plugin schema";
    } finally {
      busy = false;
    }
  }

  function fieldRecord(field: PluginSchemaField): ScanRecord | null {
    return (
      allRecords.find(
        (record) => record.path === `${settingsPath}.${field.key}`,
      ) ?? null
    );
  }

  function valueOf(field: PluginSchemaField): ControlValue {
    const record = fieldRecord(field);
    return record ? values[record.path] : null;
  }

  function change(field: PluginSchemaField, value: ControlValue) {
    const record = fieldRecord(field);
    if (record) onchange(record.path, value);
  }

  async function addField(field: PluginSchemaField) {
    busy = true;
    error = "";
    try {
      const hasSettings = allRecords.some(
        (record) => record.path === settingsPath,
      );
      if (!hasSettings) {
        await rpc(
          "structure.addField",
          { parent: plugin.path, field: "settings", literal: "{ }" },
          csrf,
        );
        await onstructurechange();
      }
      await rpc(
        "structure.addField",
        { parent: settingsPath, field: field.key, literal: field.default },
        csrf,
      );
      await onstructurechange();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot add the setting";
    } finally {
      busy = false;
    }
  }
</script>

{#if fields.length || error}
  <div class="plugin-settings">
    {#if error}
      <p class="error">{error}</p>
    {/if}
    {#each fields as field (field.key)}
      {@const record = fieldRecord(field)}
      <div class="plugin-setting">
        <label>
          <span>{field.label || field.key}</span>
          {#if !record}
            <button
              type="button"
              class="ghost"
              disabled={busy}
              onclick={() => addField(field)}
            >
              Add
            </button>
          {:else if field.kind === "bool"}
            <input
              type="checkbox"
              checked={Boolean(valueOf(field))}
              onchange={(event) =>
                change(field, event.currentTarget.checked)}
            />
          {:else if field.kind === "int" || field.kind === "float"}
            <input
              type="number"
              value={Number(valueOf(field) ?? 0)}
              min={field.minimum ?? undefined}
              max={field.maximum ?? undefined}
              oninput={(event) =>
                change(field, Number(event.currentTarget.value))}
            />
          {:else if field.kind === "dropdown"}
            <select
              value={String(valueOf(field) ?? "")}
              onchange={(event) =>
                change(field, event.currentTarget.value)}
            >
              {#each field.choices as choice (choice)}
                <option value={choice}>{choice}</option>
              {/each}
            </select>
          {:else if field.kind === "color"}
            <input
              type="color"
              value={String(valueOf(field) ?? "#000000")}
              oninput={(event) =>
                change(field, event.currentTarget.value)}
            />
          {:else if field.kind === "raw"}
            <textarea
              rows="3"
              value={String(valueOf(field) ?? "")}
              oninput={(event) =>
                change(field, event.currentTarget.value)}
            ></textarea>
          {:else}
            <input
              type="text"
              value={String(valueOf(field) ?? "")}
              oninput={(event) =>
                change(field, event.currentTarget.value)}
            />
          {/if}
        </label>
        {#if field.description}
          <p class="muted small">{field.description}</p>
        {/if}
      </div>
    {/each}
  </div>
{/if}
