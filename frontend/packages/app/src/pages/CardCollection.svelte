<script lang="ts">
  import {
    Control,
    type ControlDescriptor,
    type ControlValue,
  } from "@nix-web-gui/ui";
  import MediaPicker from "./MediaPicker.svelte";
  import MonitorCanvas from "./MonitorCanvas.svelte";
  import PluginSettings from "./PluginSettings.svelte";
  import EnvEditor from "./EnvEditor.svelte";
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
  export let fieldHelp:
    | ((parent: ScanRecord, record: ScanRecord) => string | null)
    | undefined = undefined;
  export let onstructurechange: () => Promise<void> = async () => {};

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

  let gridDraft: Record<
    string,
    { col: number; row: number; w: number; h: number }
  > = {};
  let gridMove: {
    path: string;
    colPath: string;
    rowPath: string;
    startX: number;
    startY: number;
    originCol: number;
    originRow: number;
    cellWidth: number;
    rowHeight: number;
  } | null = null;
  let gridResize: {
    path: string;
    wPath: string;
    hPath: string;
    startX: number;
    startY: number;
    originW: number;
    originH: number;
    cellWidth: number;
    rowHeight: number;
  } | null = null;

  function cellField(record: ScanRecord, field: string): ScanRecord | null {
    return (
      allRecords.find(
        (candidate) => candidate.path === `${record.path}.${field}`,
      ) ?? null
    );
  }

  function cellNumbers(record: ScanRecord) {
    const draft = gridDraft[record.path];
    return {
      col: draft?.col ?? fieldValue(record, "col") ?? 0,
      row: draft?.row ?? fieldValue(record, "row") ?? 0,
      w: draft?.w ?? fieldValue(record, "w") ?? 1,
      h: draft?.h ?? fieldValue(record, "h") ?? 1,
    };
  }

  function gridMetrics(element: HTMLElement) {
    const canvas = element.closest(".card-collection") as HTMLElement | null;
    const columns = gridColumns ?? 1;
    return {
      cellWidth: (canvas?.clientWidth ?? columns * 72) / columns,
      rowHeight: 72,
    };
  }

  function startGridMove(event: PointerEvent, record: ScanRecord) {
    const col = cellField(record, "col");
    const row = cellField(record, "row");
    if (!col || !row) return;
    event.preventDefault();
    event.stopPropagation();
    const numbers = cellNumbers(record);
    const metrics = gridMetrics(event.currentTarget as HTMLElement);
    gridMove = {
      path: record.path,
      colPath: col.path,
      rowPath: row.path,
      startX: event.clientX,
      startY: event.clientY,
      originCol: numbers.col,
      originRow: numbers.row,
      ...metrics,
    };
    try {
      (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    } catch {
      // Synthetic pointer events in tests have no active pointer.
    }
  }

  function moveGrid(event: PointerEvent) {
    if (!gridMove) return;
    const col = Math.max(
      0,
      gridMove.originCol +
        Math.round((event.clientX - gridMove.startX) / gridMove.cellWidth),
    );
    const row = Math.max(
      0,
      gridMove.originRow +
        Math.round((event.clientY - gridMove.startY) / gridMove.rowHeight),
    );
    const current = gridDraft[gridMove.path] ?? {
      ...cellNumbers(allRecords.find((item) => item.path === gridMove?.path)!),
    };
    gridDraft = {
      ...gridDraft,
      [gridMove.path]: { ...current, col, row },
    };
  }

  function endGridMove(event: PointerEvent) {
    if (!gridMove) return;
    const draft = gridDraft[gridMove.path];
    const colPath = gridMove.colPath;
    const rowPath = gridMove.rowPath;
    gridMove = null;
    if (draft) {
      onchange(colPath, draft.col);
      onchange(rowPath, draft.row);
    }
    try {
      (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    } catch {
      // Synthetic pointer events in tests have no active pointer.
    }
  }

  function startGridResize(event: PointerEvent, record: ScanRecord) {
    const w = cellField(record, "w");
    const h = cellField(record, "h");
    if (!w || !h) return;
    event.preventDefault();
    event.stopPropagation();
    const numbers = cellNumbers(record);
    const metrics = gridMetrics(event.currentTarget as HTMLElement);
    gridResize = {
      path: record.path,
      wPath: w.path,
      hPath: h.path,
      startX: event.clientX,
      startY: event.clientY,
      originW: numbers.w,
      originH: numbers.h,
      ...metrics,
    };
    try {
      (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    } catch {
      // Synthetic pointer events in tests have no active pointer.
    }
  }

  function resizeGrid(event: PointerEvent) {
    if (!gridResize) return;
    const record = allRecords.find((item) => item.path === gridResize?.path);
    if (!record) return;
    const current = gridDraft[record.path] ?? cellNumbers(record);
    const w = Math.max(
      1,
      gridResize.originW +
        Math.round((event.clientX - gridResize.startX) / gridResize.cellWidth),
    );
    const h = Math.max(
      1,
      gridResize.originH +
        Math.round((event.clientY - gridResize.startY) / gridResize.rowHeight),
    );
    gridDraft = {
      ...gridDraft,
      [record.path]: { ...current, w, h },
    };
  }

  function endGridResize(event: PointerEvent) {
    if (!gridResize) return;
    const draft = gridDraft[gridResize.path];
    const wPath = gridResize.wPath;
    const hPath = gridResize.hPath;
    gridResize = null;
    if (draft) {
      onchange(wPath, draft.w);
      onchange(hPath, draft.h);
    }
    try {
      (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    } catch {
      // Synthetic pointer events in tests have no active pointer.
    }
  }

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
    const { col, row, w, h } = cellNumbers(record);
    return `grid-column: ${col + 1} / span ${w}; grid-row: ${row + 1} / span ${h};`;
  }

  function bodyRecords(child: ScanRecord, records: ScanRecord[]): ScanRecord[] {
    const rows = records.filter((record) => {
      if (!isVisible(record)) return false;
      if (parent.widget === "plugins" && record.path.startsWith(`${child.path}.settings`)) {
        return false;
      }
      const environment = `${child.path}.${child.containerEnvironment ?? "environment"}`;
      if (parent.widget === "docker" && record.path.startsWith(`${environment}.`)) {
        return false;
      }
      return !(record.hidden && record.children?.length);
    });
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
  {#if parent.widget === "monitors"}
    <MonitorCanvas
      {parent}
      {allRecords}
      {values}
      {onchange}
    />
  {/if}
  {#if canAdd}
    <div class="toolbar">
      <span class="spacer"></span>
      <button class="ghost" onclick={onadd}>Add row</button>
    </div>
  {/if}

  {#each rows as row (row.child.path)}
    <section class="row-card" style={cellStyle(row.child)}>
      <header class="row-card-head">
        {#if gridColumns}
          <button
            class="grid-handle"
            type="button"
            aria-label="Move card"
            onpointerdown={(event) => startGridMove(event, row.child)}
            onpointermove={moveGrid}
            onpointerup={endGridMove}
          >
            ⠿
          </button>
        {/if}
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
            {#if fieldHelp?.(parent, record)}
              <p class="muted small">{fieldHelp(parent, record)}</p>
            {/if}
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
        {#if parent.widget === "docker"}
          <EnvEditor
            container={row.child}
            {allRecords}
            {values}
            {csrf}
            {onchange}
            {onstructurechange}
          />
        {/if}
        {#if parent.widget === "plugins"}
          <PluginSettings
            plugin={row.child}
            {allRecords}
            {values}
            {csrf}
            {onchange}
            {onstructurechange}
          />
        {/if}
        {#if bodyRecords(row.child, row.records).length === 0}
          <p class="muted small">No fields in this row.</p>
        {/if}
      </div>
      {#if gridColumns}
        <button
          class="grid-resize"
          type="button"
          aria-label="Resize card"
          onpointerdown={(event) => startGridResize(event, row.child)}
          onpointermove={resizeGrid}
          onpointerup={endGridResize}
        ></button>
      {/if}
    </section>
  {/each}
</div>
