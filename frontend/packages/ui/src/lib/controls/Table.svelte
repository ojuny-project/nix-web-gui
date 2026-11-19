<script lang="ts">
  import type { TableRow } from "../types";

  export let id: string;
  export let columns: string[];
  export let columnTypes: string[] = [];
  export let choices: string[] = [];
  export let list = false;
  export let keyEditable = false;
  export let firstValueEditable = false;
  export let rows: TableRow[] = [];
  export let disabled = false;
  export let onchange: (value: { rows: TableRow[] }) => void;
  export let onIconPick:
    | ((current: string) => Promise<string | null>)
    | undefined = undefined;
  export let onIconPreview:
    | ((value: string) => Promise<string | null>)
    | undefined = undefined;
  export let onIconDelete:
    | ((value: string) => Promise<void> | void)
    | undefined = undefined;
  let picking = false;
  let deleting: Record<string, boolean> = {};
  let iconPreviews: Record<string, string> = {};
  let loadingIcons = new Set<string>();
  $: iconValues = columns.flatMap((column, index) =>
    columnTypes[index] === "icon" || columnTypes[index] === "image"
      ? rows.map((row) => row.fields[column] ?? "").filter(Boolean)
      : [],
  );
  $: for (const value of iconValues) {
    void loadIconPreview(value);
  }

  async function loadIconPreview(value: string) {
    if (!onIconPreview || iconPreviews[value] || loadingIcons.has(value)) return;
    loadingIcons.add(value);
    try {
      const preview = await onIconPreview(value);
      if (preview) {
        iconPreviews = { ...iconPreviews, [value]: preview };
      }
    } catch {
      // Keep the text value when the icon cannot be read.
    } finally {
      loadingIcons.delete(value);
    }
  }

  async function pickIcon(row: TableRow, column: string) {
    if (!onIconPick) return;
    picking = true;
    try {
      const selected = await onIconPick(row.fields[column] ?? "");
      if (selected != null) updateCell(row, column, selected, true);
    } finally {
      picking = false;
    }
  }

  async function removeIcon(row: TableRow, column: string) {
    if (!onIconDelete) return;
    const value = row.fields[column] ?? "";
    if (!value) return;
    deleting = { ...deleting, [value]: true };
    try {
      await onIconDelete(value);
      updateCell(row, column, "", true);
    } catch {
      // Keep the value when the file could not be removed.
    } finally {
      const next = { ...deleting };
      delete next[value];
      deleting = next;
    }
  }

  let showHidden = false;
  $: visibleRows = showHidden ? rows : rows.filter((row) => !row.hidden);
  $: hiddenCount = rows.filter((row) => row.hidden).length;

  function indexOf(row: TableRow): number {
    return rows.indexOf(row);
  }

  function updateKey(row: TableRow, key: string) {
    const index = indexOf(row);
    onchange({
      rows: rows.map((current, position) =>
        position === index ? { ...current, key } : current,
      ),
    });
  }

  function updateCell(
    row: TableRow,
    column: string,
    value: string,
    clearRaw = false,
  ) {
    const index = indexOf(row);
    onchange({
      rows: rows.map((current, position) => {
        if (position !== index) return current;
        const rawExpressions = { ...(current.rawExpressions ?? {}) };
        if (clearRaw) delete rawExpressions[column];
        const next: TableRow = {
          ...current,
          fields: { ...current.fields, [column]: value },
        };
        if (Object.keys(rawExpressions).length > 0) {
          next.rawExpressions = rawExpressions;
        } else {
          delete next.rawExpressions;
        }
        return next;
      }),
    });
  }

  function supportsRaw(columnIndex: number): boolean {
    const type = (columnTypes[columnIndex] ?? "").toLowerCase();
    return (
      type === "" ||
      type === "str" ||
      type === "string" ||
      type === "text" ||
      type === "icon" ||
      type === "image"
    );
  }

  function isRaw(row: TableRow, column: string): boolean {
    return row.rawExpressions?.[column] === true;
  }

  function setRaw(row: TableRow, column: string, raw: boolean) {
    const index = indexOf(row);
    onchange({
      rows: rows.map((current, position) => {
        if (position !== index) return current;
        const rawExpressions = { ...(current.rawExpressions ?? {}) };
        if (raw) rawExpressions[column] = true;
        else delete rawExpressions[column];
        const next: TableRow = { ...current };
        if (Object.keys(rawExpressions).length > 0) {
          next.rawExpressions = rawExpressions;
        } else {
          delete next.rawExpressions;
        }
        return next;
      }),
    });
  }

  function addRow() {
    const fields = Object.fromEntries(
      columns.map((column, index) => [
        column,
        columnTypes[index] === "bool" ? "false" : "",
      ]),
    );
    onchange({ rows: [...rows, { key: "", fields }] });
  }

  function removeRow(row: TableRow) {
    onchange({ rows: rows.filter((current) => current !== row) });
  }

  function move(row: TableRow, offset: number) {
    const index = indexOf(row);
    const target = index + offset;
    if (target < 0 || target >= rows.length) return;
    const next = [...rows];
    [next[index], next[target]] = [next[target], next[index]];
    onchange({ rows: next });
  }

  function firstCellStatic(columnIndex: number): boolean {
    return list && columnIndex === 0 && !firstValueEditable;
  }
</script>

<div class="table-field" data-table={id}>
  {#if hiddenCount > 0}
    <label class="inline small">
      <input type="checkbox" bind:checked={showHidden} />
      <span>Show {hiddenCount} hidden row(s)</span>
    </label>
  {/if}
  <table>
    <thead>
      <tr>
        {#if !list}
          <th>Key</th>
        {/if}
        {#each columns as column (column)}
          <th>{column}</th>
        {/each}
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each visibleRows as row (indexOf(row))}
        <tr class:row-hidden={row.hidden}>
          {#if !list}
            <td>
              {#if keyEditable || row.key === ""}
                <input
                  value={row.key}
                  {disabled}
                  placeholder="key"
                  oninput={(event) => updateKey(row, event.currentTarget.value)}
                />
              {:else}
                <code title="Locked row key">{row.label || row.key}</code>
              {/if}
            </td>
          {/if}
          {#each columns as column, columnIndex (column)}
            <td>
              {#if firstCellStatic(columnIndex)}
                <code>{row.fields[column] ?? ""}</code>
              {:else if columnTypes[columnIndex] === "bool"}
                <input
                  type="checkbox"
                  checked={row.fields[column] === "true"}
                  {disabled}
                  onchange={(event) =>
                    updateCell(
                      row,
                      column,
                      event.currentTarget.checked ? "true" : "false",
                    )}
                />
              {:else if columnTypes[columnIndex] === "icon" || columnTypes[columnIndex] === "image"}
                <div class="icon-cell">
                  {#if iconPreviews[row.fields[column] ?? ""]}
                    <img
                      class="icon-thumb"
                      src={iconPreviews[row.fields[column] ?? ""]}
                      alt=""
                    />
                  {/if}
                  <input
                    value={row.fields[column] ?? ""}
                    {disabled}
                    oninput={(event) =>
                      updateCell(row, column, event.currentTarget.value)}
                  />
                  {#if onIconPick}
                    <button
                      type="button"
                      class="ghost tiny"
                      disabled={disabled || picking}
                      onclick={() => pickIcon(row, column)}
                    >
                      Pick
                    </button>
                  {/if}
                  {#if onIconDelete}
                    <button
                      type="button"
                      class="ghost tiny"
                      disabled={disabled ||
                        !row.fields[column] ||
                        deleting[row.fields[column] ?? ""] === true}
                      onclick={() => removeIcon(row, column)}
                    >
                      Delete
                    </button>
                  {/if}
                </div>
                {#if supportsRaw(columnIndex)}
                  <label class="inline small">
                    <input
                      type="checkbox"
                      checked={isRaw(row, column)}
                      {disabled}
                      onchange={(event) =>
                        setRaw(row, column, event.currentTarget.checked)}
                    />
                    <span>Raw expression</span>
                  </label>
                {/if}
              {:else if columnTypes[columnIndex] === "dropdown" && choices.length > 0}
                <select
                  value={row.fields[column] ?? ""}
                  {disabled}
                  onchange={(event) =>
                    updateCell(row, column, event.currentTarget.value)}
                >
                  {#each choices as choice (choice)}
                    <option value={choice}>{choice}</option>
                  {/each}
                </select>
              {:else}
                <input
                  value={row.fields[column] ?? ""}
                  {disabled}
                  oninput={(event) =>
                    updateCell(row, column, event.currentTarget.value)}
                />
                {#if supportsRaw(columnIndex)}
                  <label class="inline small">
                    <input
                      type="checkbox"
                      checked={isRaw(row, column)}
                      {disabled}
                      onchange={(event) =>
                        setRaw(row, column, event.currentTarget.checked)}
                    />
                    <span>Raw expression</span>
                  </label>
                {/if}
              {/if}
            </td>
          {/each}
          <td class="row-actions">
            {#if list && row.label}
              <span class="muted small">{row.label}</span>
            {/if}
            <button
              type="button"
              class="ghost tiny"
              {disabled}
              title="Move up"
              onclick={() => move(row, -1)}
            >
              Up
            </button>
            <button
              type="button"
              class="ghost tiny"
              {disabled}
              title="Move down"
              onclick={() => move(row, 1)}
            >
              Down
            </button>
            <button
              type="button"
              class="ghost tiny"
              {disabled}
              onclick={() => removeRow(row)}
            >
              Remove
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
  <button type="button" class="ghost" {disabled} onclick={addRow}>
    Add row
  </button>
</div>
