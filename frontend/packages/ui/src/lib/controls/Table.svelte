<script lang="ts">
  import type { TableRow } from "../types";

  export let id: string;
  export let columns: string[];
  export let columnTypes: string[] = [];
  export let list = false;
  export let keyEditable = false;
  export let rows: TableRow[] = [];
  export let disabled = false;
  export let onchange: (value: { rows: TableRow[] }) => void;

  function updateKey(index: number, key: string) {
    onchange({
      rows: rows.map((row, current) =>
        current === index ? { ...row, key } : row,
      ),
    });
  }

  function updateCell(index: number, column: string, value: string) {
    onchange({
      rows: rows.map((row, current) =>
        current === index
          ? { ...row, fields: { ...row.fields, [column]: value } }
          : row,
      ),
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

  function removeRow(index: number) {
    onchange({ rows: rows.filter((_, current) => current !== index) });
  }
</script>

<div class="table-field" data-table={id}>
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
      {#each rows as row, index (index)}
        <tr>
          {#if !list}
            <td>
              {#if keyEditable || row.key === ""}
                <input
                  value={row.key}
                  {disabled}
                  placeholder="key"
                  oninput={(event) => updateKey(index, event.currentTarget.value)}
                />
              {:else}
                <code>{row.key}</code>
              {/if}
            </td>
          {/if}
          {#each columns as column, columnIndex (column)}
            <td>
              {#if columnTypes[columnIndex] === "bool"}
                <input
                  type="checkbox"
                  checked={row.fields[column] === "true"}
                  {disabled}
                  onchange={(event) =>
                    updateCell(
                      index,
                      column,
                      event.currentTarget.checked ? "true" : "false",
                    )}
                />
              {:else}
                <input
                  value={row.fields[column] ?? ""}
                  {disabled}
                  oninput={(event) =>
                    updateCell(index, column, event.currentTarget.value)}
                />
              {/if}
            </td>
          {/each}
          <td>
            <button
              type="button"
              class="ghost"
              {disabled}
              onclick={() => removeRow(index)}
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
