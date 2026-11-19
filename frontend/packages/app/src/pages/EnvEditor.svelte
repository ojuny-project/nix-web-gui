<script lang="ts">
  import type { ControlValue } from "@nix-web-gui/ui";
  import { rpc, type ScanRecord } from "../api";

  export let container: ScanRecord;
  export let allRecords: ScanRecord[];
  export let values: Record<string, ControlValue>;
  export let csrf: string;
  export let onchange: (path: string, value: ControlValue) => void;
  export let onstructurechange: () => Promise<void>;

  let name = "";
  let value = "";
  let busy = false;
  let error = "";

  $: environmentName = container.containerEnvironment ?? "environment";
  $: environmentPath = `${container.path}.${environmentName}`;
  $: variables = allRecords.filter(
    (record) =>
      record.path.startsWith(`${environmentPath}.`) &&
      !record.path.slice(environmentPath.length + 1).includes("."),
  );

  async function ensureEnvironment() {
    if (allRecords.some((record) => record.path === environmentPath)) return;
    await rpc(
      "structure.addField",
      { parent: container.path, field: environmentName, literal: "{ }" },
      csrf,
    );
    await onstructurechange();
  }

  async function addVariable(event: SubmitEvent) {
    event.preventDefault();
    if (!name.trim()) {
      error = "Variable name is required";
      return;
    }
    busy = true;
    error = "";
    try {
      await ensureEnvironment();
      await rpc(
        "structure.addField",
        {
          parent: environmentPath,
          field: name.trim(),
          literal: JSON.stringify(value),
        },
        csrf,
      );
      name = "";
      value = "";
      await onstructurechange();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot add the variable";
    } finally {
      busy = false;
    }
  }

  async function removeVariable(record: ScanRecord) {
    if (!confirm(`Remove ${record.path.split(".").at(-1)}?`)) return;
    busy = true;
    error = "";
    try {
      await rpc(
        "structure.remove",
        { parent: environmentPath, child: record.path },
        csrf,
      );
      await onstructurechange();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot remove the variable";
    } finally {
      busy = false;
    }
  }

  async function renameVariable(record: ScanRecord, next: string) {
    if (!next || next === record.path.split(".").at(-1)) return;
    busy = true;
    error = "";
    try {
      await rpc(
        "structure.rename",
        { parent: environmentPath, child: record.path, key: next },
        csrf,
      );
      await onstructurechange();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot rename the variable";
    } finally {
      busy = false;
    }
  }
</script>

<div class="env-editor">
  {#if error}
    <p class="error">{error}</p>
  {/if}
  {#each variables as variable (variable.path)}
    <div class="env-row">
      <input
        value={variable.path.split(".").at(-1)}
        onchange={(event) =>
          renameVariable(variable, event.currentTarget.value)}
      />
      <input
        value={String(values[variable.path] ?? "")}
        oninput={(event) => onchange(variable.path, event.currentTarget.value)}
      />
      <button
        type="button"
        class="ghost tiny"
        disabled={busy}
        onclick={() => removeVariable(variable)}
      >
        Remove
      </button>
    </div>
  {/each}
  <form class="env-row" onsubmit={addVariable}>
    <input bind:value={name} placeholder="NAME" disabled={busy} />
    <input bind:value={value} placeholder="value" disabled={busy} />
    <button type="submit" class="ghost tiny" disabled={busy}>Add</button>
  </form>
</div>
