<script lang="ts">
  import {
    Control,
    type ControlDescriptor,
    type ControlValue,
  } from "@nix-web-gui/ui";
  import MediaPicker from "./MediaPicker.svelte";
  import type { ScanRecord } from "../api";

  export let parent: ScanRecord;
  export let allRecords: ScanRecord[];
  export let values: Record<string, ControlValue>;
  export let descriptor: (record: ScanRecord) => ControlDescriptor;
  export let onchange: (path: string, value: ControlValue) => void;
  export let isVisible: (record: ScanRecord) => boolean;
  export let csrf: string;
  export let expandDetails = false;
  export let maxColumns = 1;
  export let gridColumns: number | undefined = undefined;
  export let canAdd = false;
  export let canMove: (record: ScanRecord) => boolean = () => false;
  export let canRename: (record: ScanRecord) => boolean = () => false;
  export let canRemove: (record: ScanRecord) => boolean = () => false;
  export let onadd: () => void = () => {};
  export let onmove: (record: ScanRecord, offset: number) => void = () => {};
  export let onrename: (record: ScanRecord) => void = () => {};
  export let onremove: (record: ScanRecord) => void = () => {};
  export let onpassword: (record: ScanRecord) => void = () => {};
  export let onIconPick: (
    record: ScanRecord,
    current: string,
  ) => Promise<string | null> = async () => null;
  export let onIconPreview: (
    record: ScanRecord,
    value: string,
  ) => Promise<string | null> = async () => null;
  export let isCsv: (record: ScanRecord) => boolean = () => false;
  export let rowTitle: (record: ScanRecord) => string = (record) =>
    record.label ?? record.path.split(".").at(-1) ?? record.path;
  export let sortRows: ((left: ScanRecord, right: ScanRecord) => number) | undefined =
    undefined;

  $: children = [...(parent.children ?? [])].sort((left, right) =>
    sortRows ? sortRows(left, right) : 0,
  );
  $: rows = children.map((child) => ({
    child,
    records: allRecords.filter(
      (record) =>
        record.path === child.path || record.path.startsWith(`${child.path}.`),
    ),
  }));

  function fieldValue(record: ScanRecord, field: string): number | null {
    const child = allRecords.find(
      (candidate) => candidate.path === `${record.path}.${field}`,
    );
    if (!child) return null;
    const value = Number(values[child.path]);
    return Number.isFinite(value) ? value : null;
  }

  function cellStyle(record: ScanRecord): string | undefined {
    if (!gridColumns) return undefined;
    const col = fieldValue(record, "col") ?? 0;
    const row = fieldValue(record, "row") ?? 0;
    const width = fieldValue(record, "w") ?? 1;
    const height = fieldValue(record, "h") ?? 1;
    return `grid-column: ${col + 1} / span ${width}; grid-row: ${row + 1} / span ${height};`;
  }

  function bodyRecords(child: ScanRecord, records: ScanRecord[]): ScanRecord[] {
    const rows = records.filter(
      (record) => isVisible(record) && !(record.hidden && record.children?.length),
    );
    if (child.children?.length) {
      return rows.filter((record) => record.path !== child.path);
    }
    return rows;
  }
</script>

<div
  class="card-collection"
  class:grid-canvas={Boolean(gridColumns)}
  style={
    gridColumns
      ? `grid-template-columns: repeat(${gridColumns}, minmax(0, 1fr));`
      : maxColumns > 1
        ? `grid-template-columns: repeat(${maxColumns}, minmax(0, 1fr));`
        : undefined
  }
>
  <h3 class="group-heading">{parent.label ?? parent.path}</h3>
  {#if canAdd}
    <div class="toolbar">
      <span class="spacer"></span>
      <button class="ghost" onclick={onadd}>Add row</button>
    </div>
  {/if}

  {#each rows as row (row.child.path)}
    <section class="row-card" style={cellStyle(row.child)}>
      <header class="row-card-head">
        <strong>{rowTitle(row.child)}</strong>
        <span class="spacer"></span>
        {#if canMove(row.child)}
          <button
            class="ghost tiny"
            onclick={() => onmove(row.child, -1)}
          >
            Up
          </button>
          <button
            class="ghost tiny"
            onclick={() => onmove(row.child, 1)}
          >
            Down
          </button>
        {/if}
        {#if canRename(row.child)}
          <button class="ghost tiny" onclick={() => onrename(row.child)}>
            Rename
          </button>
        {/if}
        {#if canRemove(row.child)}
          <button class="ghost tiny" onclick={() => onremove(row.child)}>
            Remove
          </button>
        {/if}
      </header>
      <div class="row-card-body">
        {#each bodyRecords(row.child, row.records) as record (record.path)}
          {#if record.children?.length}
            <h4
              class="nested-heading"
              style={`margin-left: ${(record.depth ?? 0) * 10}px`}
            >
              {rowTitle(record)}
            </h4>
          {:else}
            <Control
              descriptor={descriptor(record)}
              value={values[record.path]}
              onchange={onchange}
              {expandDetails}
              onIconPick={(current) => onIconPick(record, current)}
              onIconPreview={(value) => onIconPreview(record, value)}
              csvList={isCsv(record)}
            />
            {#if record.widget === "font"}
              <p
                class="font-preview"
                style={`font-family: "${String(values[record.path] ?? "").replaceAll('"', "")}", sans-serif`}
              >
                The quick brown fox jumps over the lazy dog · 0123456789
              </p>
            {/if}
            {#if record.widget === "wallpaper" || record.widget === "file"}
              <MediaPicker {record} {csrf} onchange={onchange} />
            {/if}
            {#if record.path.endsWith(".hashedPassword")}
              <div class="inline-actions">
                <button class="ghost" onclick={() => onpassword(record)}>
                  Set password
                </button>
              </div>
            {/if}
          {/if}
        {/each}
        {#if bodyRecords(row.child, row.records).length === 0}
          <p class="muted small">No fields in this row.</p>
        {/if}
      </div>
    </section>
  {/each}
</div>
