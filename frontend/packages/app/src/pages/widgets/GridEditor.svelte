<script lang="ts">
  import {
    Control,
    type ControlDescriptor,
    type ControlValue,
  } from "@nix-web-gui/ui";
  import { rpc, type ScanRecord } from "../../api";

  export let parent: ScanRecord;
  export let allRecords: ScanRecord[];
  export let values: Record<string, ControlValue>;
  export let onchange: (path: string, value: ControlValue) => void;
  export let csrf: string;
  export let registry: Array<{ id: string; label: string }> = [];
  export let sizes: Array<{ w: number; h: number; label: string }> = [];
  export let columns = 6;
  export let onstructurechange: () => Promise<void> = async () => {};

  const GRID_ROW_UNIT = 72;
  const INTERNAL_FIELDS = new Set([
    "col",
    "row",
    "w",
    "h",
    "_order",
    "enabled",
    "group",
    "instanceId",
  ]);
  const GLYPHS: Record<string, string> = {
    user: "◍",
    battery: "▮",
    settings: "⚙",
    lock: "🔒",
    power: "⏻",
    brightnessSlider: "☀",
    volumeSlider: "🔊",
    inputVolumeSlider: "🎙",
    wifi: "≋",
    bluetooth: "ᛒ",
    audioOutput: "🔈",
    audioInput: "🎤",
    builtin_vpn: "🛡",
    builtin_cups: "🖨",
    builtin_tailscale: "🕸",
    builtin_display_profiles: "🖥",
    diskUsage: "💾",
    nightMode: "☾",
    darkMode: "◐",
    colorPicker: "🎨",
    doNotDisturb: "⊘",
    idleInhibitor: "⏱",
    runningApps: "▦",
    quickTiles: "⊞",
    plugin_dankKDEConnect: "📱",
    clock: "🕐",
    weather: "⛅",
    notifications: "🔔",
    calendar: "📅",
    media: "▶",
    sysmon: "📈",
    cpu: "▤",
    memory: "▥",
    network: "🌐",
    disk: "💽",
    wellbeing: "♥",
    conditions: "☁",
    hourly: "⏲",
    sun: "☀",
    moon: "☾",
    humidity: "💧",
    wind: "🌬",
    daily: "📆",
    pressure: "◎",
    precipitation: "🌧",
    visibility: "👁",
    uv: "☀",
  };

  type Cell = { col: number; row: number; w: number; h: number };
  type CardState = {
    record: ScanRecord;
    id: string;
    fields: ScanRecord[];
    col: number;
    row: number;
    w: number;
    h: number;
    order: number;
    enabled: boolean;
    group: string;
    instanceId: string;
  };
  type ResizeAxis = "right" | "bottom" | "corner";
  type MoveDrag = {
    path: string;
    startX: number;
    startY: number;
    originCol: number;
    originRow: number;
    cellWidth: number;
    rowHeight: number;
    moved: boolean;
  };
  type ResizeDrag = {
    path: string;
    axis: ResizeAxis;
    startX: number;
    startY: number;
    originW: number;
    originH: number;
    cellWidth: number;
    rowHeight: number;
    moved: boolean;
  };

  let canvas: HTMLDivElement | null = null;
  let selectedPath: string | null = null;
  let addId = "";
  let addSearch = "";
  let addWidth = sizes[0]?.w ?? 1;
  let addHeight = sizes[0]?.h ?? 1;
  let addSizeLabel = sizes[0]?.label ?? "1x1";
  let busy = false;
  let error = "";
  let draft: Record<string, Partial<Cell>> = {};
  let moveDrag: MoveDrag | null = null;
  let resizeDrag: ResizeDrag | null = null;

  $: cardRegistry =
    registry.length > 0
      ? registry
      : (parent.gridRegistry ?? []).map((id) => ({ id, label: id }));
  $: cardLabels = Object.fromEntries(
    cardRegistry.map((item) => [item.id, item.label]),
  );
  $: cards = (parent.children ?? [])
    .filter(isCardRecord)
    .filter((record) => lastSegment(record.path) !== "edit")
    .map((record) => buildCard(record, allRecords, values, columns))
    .sort((left, right) => left.order - right.order || left.id.localeCompare(right.id));
  $: cardIds = new Set(cards.map((card) => card.id));
  $: groupIds = new Set(
    cards
      .filter((card) => card.id === "quickTiles" && card.instanceId)
      .map((card) => card.instanceId),
  );
  $: groupedPaths = new Set(
    cards
      .filter((card) => card.group && groupIds.has(card.group))
      .map((card) => card.record.path),
  );
  $: quickTilesMembers = (() => {
    const result: Record<string, CardState[]> = {};
    for (const card of cards) {
      if (card.id !== "quickTiles" || !card.instanceId) continue;
      result[card.record.path] = cards.filter(
        (member) => member.group === card.instanceId,
      );
    }
    return result;
  })();
  $: availableCards = cardRegistry.filter((item) => {
    if (cardIds.has(item.id)) return false;
    const term = addSearch.trim().toLowerCase();
    return (
      !term ||
      item.label.toLowerCase().includes(term) ||
      item.id.toLowerCase().includes(term)
    );
  });
  $: if (addId && !availableCards.some((item) => item.id === addId)) {
    addId = "";
  }
  $: if (
    sizes.length > 0 &&
    !sizes.some((size) => size.w === addWidth && size.h === addHeight)
  ) {
    addWidth = sizes[0].w;
    addHeight = sizes[0].h;
    addSizeLabel = sizes[0].label;
  }
  $: layout = cards
    .filter((card) => !groupedPaths.has(card.record.path))
    .map((card) => {
      const pending = draft[card.record.path];
      const cell = normalizeCell(
        {
          col: pending?.col ?? card.col,
          row: pending?.row ?? card.row,
          w: pending?.w ?? card.w,
          h: pending?.h ?? card.h,
        },
        columns,
      );
      return { card, ...cell };
    });
  $: selected = selectedPath
    ? (cards.find((card) => card.record.path === selectedPath) ?? null)
    : null;
  $: selectedGrouped = Boolean(
    selected && selected.group && groupIds.has(selected.group),
  );
  $: selectedRegistryFields = selected
    ? selected.fields.filter(
        (field) => !INTERNAL_FIELDS.has(fieldName(field)),
      )
    : [];
  $: selectedFieldNames = new Set(
    selected ? selected.fields.map(fieldName) : [],
  );
  $: selectedExternalFields = selected
    ? externalFields(parent, allRecords, selected.id).filter(
        (record) => !selectedFieldNames.has(fieldName(record)),
      )
    : [];

  function lastSegment(path: string): string {
    return path.split(".").at(-1) ?? path;
  }

  function fieldName(record: ScanRecord): string {
    return lastSegment(record.path);
  }

  function caption(name: string): string {
    const spaced = name
      .replaceAll("_", " ")
      .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
      .trim();
    return spaced
      ? spaced.charAt(0).toUpperCase() + spaced.slice(1)
      : name;
  }

  function glyphFor(id: string): string {
    return GLYPHS[id] ?? "▦";
  }

  function isCardRecord(record: ScanRecord): boolean {
    return (
      record.widget === "submodule" ||
      (record.value ?? "").trim().startsWith("{")
    );
  }

  function collectFields(
    record: ScanRecord,
    records: ScanRecord[],
  ): ScanRecord[] {
    const children = record.children ?? [];
    if (children.length === 0) {
      const prefix = `${record.path}.`;
      return records.filter(
        (candidate) =>
          candidate.path.startsWith(prefix) &&
          !candidate.path.slice(prefix.length).includes("."),
      );
    }
    const leaves: ScanRecord[] = [];
    for (const child of children) {
      if (child.children?.length) {
        leaves.push(...collectFields(child, records));
      } else {
        leaves.push(child);
      }
    }
    return leaves;
  }

  function readField(
    fields: ScanRecord[],
    name: string,
    current: Record<string, ControlValue>,
  ): ControlValue | undefined {
    const field = fields.find((candidate) => fieldName(candidate) === name);
    return field ? current[field.path] : undefined;
  }

  function readNumber(
    fields: ScanRecord[],
    name: string,
    current: Record<string, ControlValue>,
    fallback: number,
  ): number {
    const value = Number(readField(fields, name, current));
    return Number.isFinite(value) ? value : fallback;
  }

  function readText(
    fields: ScanRecord[],
    name: string,
    current: Record<string, ControlValue>,
  ): string {
    const value = readField(fields, name, current);
    return value == null ? "" : String(value);
  }

  function clamp(value: number, minimum: number, maximum: number): number {
    return Math.min(Math.max(value, minimum), maximum);
  }

  function normalizeCell(cell: Cell, count: number): Cell {
    const w = clamp(Math.round(cell.w) || 1, 1, Math.max(1, count));
    return {
      w,
      h: Math.max(1, Math.round(cell.h) || 1),
      col: clamp(Math.round(cell.col) || 0, 0, Math.max(0, count - w)),
      row: Math.max(0, Math.round(cell.row) || 0),
    };
  }

  function buildCard(
    record: ScanRecord,
    records: ScanRecord[],
    current: Record<string, ControlValue>,
    count: number,
  ): CardState {
    const fields = collectFields(record, records);
    const cell = normalizeCell(
      {
        col: readNumber(fields, "col", current, 0),
        row: readNumber(fields, "row", current, 0),
        w: readNumber(fields, "w", current, 1),
        h: readNumber(fields, "h", current, 1),
      },
      count,
    );
    return {
      record,
      fields,
      id: lastSegment(record.path),
      ...cell,
      order: readNumber(fields, "_order", current, 1000),
      enabled:
        readField(fields, "enabled", current) === undefined
          ? true
          : Boolean(readField(fields, "enabled", current)),
      group: readText(fields, "group", current),
      instanceId: readText(fields, "instanceId", current),
    };
  }

  function externalFields(
    record: ScanRecord,
    records: ScanRecord[],
    id: string,
  ): ScanRecord[] {
    const namespace = record.path.split(".").slice(0, -1).join(".");
    const prefix = `${namespace}.dashOptions.${id}.`;
    return records
      .filter(
        (candidate) =>
          candidate.path.startsWith(prefix) &&
          !candidate.path.slice(prefix.length).includes("."),
      )
      .sort((left, right) =>
        fieldName(left).localeCompare(fieldName(right)),
      );
  }

  function fieldByName(card: CardState, name: string): ScanRecord | null {
    return card.fields.find((field) => fieldName(field) === name) ?? null;
  }

  function inputId(card: CardState, name: string): string {
    const field = fieldByName(card, name);
    const path = field?.path ?? `${card.record.path}.${name}`;
    return `control-${path.replaceAll(".", "-")}`;
  }

  function descriptorFor(record: ScanRecord): ControlDescriptor {
    return {
      path: record.path,
      widget: (record.widget ?? "text") as ControlDescriptor["widget"],
      label: record.label || caption(fieldName(record)),
      description: record.description,
      choices: record.choices,
      min: record.min,
      max: record.max,
      step: record.step,
      digits: record.digits,
      unit: record.unit,
      placeholder: record.placeholder,
      rows: record.rows,
      readonly: record.readonly,
      advanced: record.advanced,
      hidden: record.hidden,
      group: record.group,
      order: record.order,
      groupOrder: record.groupOrder,
      depth: 0,
      table: record.table,
      origin: record.origin,
      visibleWhen: record.visibleWhen,
    };
  }

  function select(card: CardState) {
    selectedPath = card.record.path;
  }

  function selectById(path: string) {
    selectedPath = path;
  }

  function gridMetrics() {
    const width = canvas?.clientWidth ?? columns * GRID_ROW_UNIT;
    return {
      cellWidth: Math.max(1, width / Math.max(1, columns)),
      rowHeight: GRID_ROW_UNIT,
    };
  }

  function capture(event: PointerEvent) {
    const target = event.currentTarget as HTMLElement | null;
    try {
      target?.setPointerCapture(event.pointerId);
    } catch {
      // Synthetic pointer events in tests have no active pointer.
    }
  }

  function release(event: PointerEvent) {
    const target = event.currentTarget as HTMLElement | null;
    try {
      target?.releasePointerCapture(event.pointerId);
    } catch {
      // Synthetic pointer events in tests have no active pointer.
    }
  }

  function beginMove(event: PointerEvent, card: CardState) {
    event.preventDefault();
    event.stopPropagation();
    select(card);
    const metrics = gridMetrics();
    moveDrag = {
      path: card.record.path,
      startX: event.clientX,
      startY: event.clientY,
      originCol: card.col,
      originRow: card.row,
      cellWidth: metrics.cellWidth,
      rowHeight: metrics.rowHeight,
      moved: false,
    };
    capture(event);
  }

  function updateMove(event: PointerEvent, card: CardState) {
    if (!moveDrag || moveDrag.path !== card.record.path) return;
    const col = clamp(
      moveDrag.originCol +
        Math.round((event.clientX - moveDrag.startX) / moveDrag.cellWidth),
      0,
      Math.max(0, columns - card.w),
    );
    const row = Math.max(
      0,
      moveDrag.originRow +
        Math.round((event.clientY - moveDrag.startY) / moveDrag.rowHeight),
    );
    if (col !== moveDrag.originCol || row !== moveDrag.originRow) {
      moveDrag.moved = true;
    }
    draft = { ...draft, [moveDrag.path]: { col, row } };
  }

  function finishMove(event: PointerEvent, card: CardState) {
    if (!moveDrag || moveDrag.path !== card.record.path) return;
    const drag = moveDrag;
    moveDrag = null;
    release(event);
    const cell = draft[drag.path];
    clearDraft(card);
    if (!drag.moved || !cell) return;
    commitNumber(card, "col", cell.col);
    commitNumber(card, "row", cell.row);
  }

  function cancelMove() {
    moveDrag = null;
    draft = {};
  }

  function beginResize(
    event: PointerEvent,
    card: CardState,
    axis: ResizeAxis,
  ) {
    event.preventDefault();
    event.stopPropagation();
    select(card);
    const metrics = gridMetrics();
    resizeDrag = {
      path: card.record.path,
      axis,
      startX: event.clientX,
      startY: event.clientY,
      originW: card.w,
      originH: card.h,
      cellWidth: metrics.cellWidth,
      rowHeight: metrics.rowHeight,
      moved: false,
    };
    capture(event);
  }

  function updateResize(event: PointerEvent, card: CardState) {
    if (!resizeDrag || resizeDrag.path !== card.record.path) return;
    let w = resizeDrag.originW;
    let h = resizeDrag.originH;
    if (resizeDrag.axis !== "bottom") {
      w += Math.round(
        (event.clientX - resizeDrag.startX) / resizeDrag.cellWidth,
      );
    }
    if (resizeDrag.axis !== "right") {
      h += Math.round(
        (event.clientY - resizeDrag.startY) / resizeDrag.rowHeight,
      );
    }
    w = clamp(w, 1, Math.max(1, columns - card.col));
    h = Math.max(1, h);
    if (w !== resizeDrag.originW || h !== resizeDrag.originH) {
      resizeDrag.moved = true;
    }
    draft = { ...draft, [resizeDrag.path]: { w, h } };
  }

  function finishResize(event: PointerEvent, card: CardState) {
    if (!resizeDrag || resizeDrag.path !== card.record.path) return;
    const drag = resizeDrag;
    resizeDrag = null;
    release(event);
    const cell = draft[drag.path];
    clearDraft(card);
    if (!drag.moved || !cell) return;
    commitNumber(card, "w", cell.w);
    commitNumber(card, "h", cell.h);
  }

  function cancelResize() {
    resizeDrag = null;
    draft = {};
  }

  function clearDraft(card: CardState) {
    if (!(card.record.path in draft)) return;
    const next = { ...draft };
    delete next[card.record.path];
    draft = next;
  }

  function commitNumber(
    card: CardState,
    name: "col" | "row" | "w" | "h",
    value: number | undefined,
  ) {
    if (value === undefined) return;
    const field = fieldByName(card, name);
    if (!field || value === card[name]) return;
    onchange(field.path, value);
  }

  function setGeometry(
    card: CardState,
    name: "col" | "row" | "w" | "h",
    value: number,
  ) {
    if (!Number.isFinite(value)) return;
    const next = normalizeCell(
      { col: card.col, row: card.row, w: card.w, h: card.h, [name]: value },
      columns,
    );
    for (const key of ["col", "row", "w", "h"] as const) {
      commitNumber(card, key, next[key]);
    }
  }

  function resizeBy(card: CardState, dw: number, dh: number) {
    setGeometry(card, "w", card.w + dw);
    setGeometry(card, "h", card.h + dh);
  }

  function applySize(card: CardState, size: { w: number; h: number }) {
    setGeometry(card, "w", size.w);
    setGeometry(card, "h", size.h);
  }

  async function toggleEnabled(card: CardState, checked: boolean) {
    const field = fieldByName(card, "enabled");
    if (field) {
      onchange(field.path, checked);
      return;
    }
    busy = true;
    error = "";
    try {
      await rpc(
        "structure.addField",
        {
          parent: card.record.path,
          field: "enabled",
          literal: checked ? "true" : "false",
        },
        csrf,
      );
      await onstructurechange();
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Cannot change the card state";
    } finally {
      busy = false;
    }
  }

  function firstFit(
    taken: Array<{ col: number; row: number; w: number; h: number }>,
    count: number,
    w: number,
    h: number,
  ): { col: number; row: number } {
    const ceiling = taken.reduce(
      (maximum, cell) => Math.max(maximum, cell.row + cell.h),
      0,
    );
    for (let row = 0; row < ceiling; row += 1) {
      for (let col = 0; col + w <= count; col += 1) {
        const overlaps = taken.some(
          (cell) =>
            col < cell.col + cell.w &&
            cell.col < col + w &&
            row < cell.row + cell.h &&
            cell.row < row + h,
        );
        if (!overlaps) return { col, row };
      }
    }
    return { col: 0, row: ceiling };
  }

  async function addCard() {
    if (!addId || busy) return;
    busy = true;
    error = "";
    try {
      const position = firstFit(layout, columns, addWidth, addHeight);
      const defaults: Record<string, string> = {
        _order: String(
          cards.reduce((maximum, card) => Math.max(maximum, card.order), 0) + 10,
        ),
        col: String(position.col),
        row: String(position.row),
        w: String(addWidth),
        h: String(addHeight),
      };
      if (parent.gridEnabled) defaults.enabled = "true";
      const id = addId;
      await rpc(
        "structure.add",
        { path: parent.path, key: id, defaults },
        csrf,
      );
      addId = "";
      selectedPath = `${parent.path}.${id}`;
      await onstructurechange();
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Cannot add the widget";
    } finally {
      busy = false;
    }
  }

  async function removeCard(card: CardState) {
    const label = cardLabels[card.id] ?? caption(card.id);
    if (!confirm(`Remove the ${label} card?`)) return;
    busy = true;
    error = "";
    try {
      await rpc(
        "structure.remove",
        { parent: parent.path, child: card.record.path },
        csrf,
      );
      if (selectedPath === card.record.path) selectedPath = null;
      await onstructurechange();
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Cannot remove the card";
    } finally {
      busy = false;
    }
  }

  function chooseAddSize(size: { w: number; h: number; label: string }) {
    addWidth = size.w;
    addHeight = size.h;
    addSizeLabel = size.label;
  }
</script>

<div class="grid-editor">
  <div class="grid-adder">
    <div class="grid-adder-row">
      <input
        type="search"
        bind:value={addSearch}
        placeholder="Search widgets"
        aria-label="Search grid widgets"
      />
      <select bind:value={addId} aria-label="Widget to add" disabled={busy}>
        <option value="">Add widget…</option>
        {#each availableCards as item (item.id)}
          <option value={item.id}>{item.label}</option>
        {/each}
      </select>
      <button
        type="button"
        disabled={!addId || busy}
        onclick={addCard}
      >
        Add
      </button>
    </div>
    <div class="grid-adder-row">
      <span class="grid-dim">Catalog size</span>
      {#each sizes as size (size.label)}
        <button
          type="button"
          class="ghost tiny"
          class:active={size.w === addWidth && size.h === addHeight}
          onclick={() => chooseAddSize(size)}
        >
          {size.label}
        </button>
      {/each}
      <span class="grid-spacer"></span>
      <span class="grid-dim">
        Profile: {columns} columns{addSizeLabel ? ` · ${addSizeLabel}` : ""}
      </span>
    </div>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div
    class="grid-canvas"
    class:grid-busy={busy}
    bind:this={canvas}
    style={`--grid-columns: ${columns}`}
  >
    {#each layout as item (item.card.record.path)}
      {@const card = item.card}
      <div
        class="grid-card"
        class:grid-card-selected={selectedPath === card.record.path}
        class:grid-card-off={!card.enabled && Boolean(parent.gridEnabled)}
        style={`grid-column: ${item.col + 1} / span ${item.w}; grid-row: ${item.row + 1} / span ${item.h};`}
        role="button"
        tabindex="0"
        onpointerdown={() => select(card)}
        onkeydown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            select(card);
          }
        }}
      >
        <span class="grid-card-size">{item.w}×{item.h}</span>
        <div class="grid-card-body">
          <span class="grid-card-icon" aria-hidden="true"
            >{glyphFor(card.id)}</span
          >
          <strong class="grid-card-title"
            >{cardLabels[card.id] ?? caption(card.id)}</strong
          >
          {#if card.id === "quickTiles"}
            <span class="grid-card-members">
              {#each quickTilesMembers[card.record.path] ?? [] as member (member.record.path)}
                <span
                  class="grid-member"
                  title={cardLabels[member.id] ?? caption(member.id)}
                  >{glyphFor(member.id)}</span
                >
              {:else}
                <span class="grid-dim">No quick tiles</span>
              {/each}
            </span>
          {/if}
        </div>
        <button
          type="button"
          class="grid-handle"
          aria-label="Move card"
          onpointerdown={(event) => beginMove(event, card)}
          onpointermove={(event) => updateMove(event, card)}
          onpointerup={(event) => finishMove(event, card)}
          onpointercancel={cancelMove}
        >⠿</button>
        <button
          type="button"
          class="grid-resize grid-resize-right"
          aria-label="Resize card width"
          onpointerdown={(event) => beginResize(event, card, "right")}
          onpointermove={(event) => updateResize(event, card)}
          onpointerup={(event) => finishResize(event, card)}
          onpointercancel={cancelResize}
        ></button>
        <button
          type="button"
          class="grid-resize grid-resize-bottom"
          aria-label="Resize card height"
          onpointerdown={(event) => beginResize(event, card, "bottom")}
          onpointermove={(event) => updateResize(event, card)}
          onpointerup={(event) => finishResize(event, card)}
          onpointercancel={cancelResize}
        ></button>
        <button
          type="button"
          class="grid-resize grid-resize-corner"
          aria-label="Resize card"
          onpointerdown={(event) => beginResize(event, card, "corner")}
          onpointermove={(event) => updateResize(event, card)}
          onpointerup={(event) => finishResize(event, card)}
          onpointercancel={cancelResize}
        ></button>
      </div>
    {:else}
      <p class="grid-empty">No cards yet. Add a widget to start.</p>
    {/each}
  </div>

  <section class="grid-details">
    {#if selected}
      {@const card = selected}
      <header class="grid-details-head">
        <span class="grid-details-icon" aria-hidden="true"
          >{glyphFor(card.id)}</span
        >
        <div class="grid-details-title">
          <h4>{cardLabels[card.id] ?? caption(card.id)}</h4>
          <p class="grid-dim">
            {card.id}
            {#if selectedGrouped}· grouped under {card.group}{/if}
          </p>
        </div>
        <span class="grid-spacer"></span>
        <button
          type="button"
          class="ghost tiny"
          disabled={busy}
          onclick={() => removeCard(card)}
        >
          Remove card
        </button>
      </header>

      {#if parent.gridEnabled || fieldByName(card, "enabled")}
        <label class="grid-toggle">
          <input
            type="checkbox"
            checked={card.enabled}
            disabled={busy}
            onchange={(event) =>
              toggleEnabled(card, event.currentTarget.checked)}
          />
          <span>Enabled</span>
        </label>
      {/if}

      <div class="grid-details-section">
        <h5>Placement</h5>
        <div class="grid-geometry">
          <label>
            <span>Column</span>
            <input
              id={inputId(card, "col")}
              type="number"
              min="0"
              max={Math.max(0, columns - 1)}
              step="1"
              value={card.col}
              disabled={busy || selectedGrouped}
              oninput={(event) =>
                setGeometry(card, "col", Number(event.currentTarget.value))}
            />
          </label>
          <label>
            <span>Row</span>
            <input
              id={inputId(card, "row")}
              type="number"
              min="0"
              step="1"
              value={card.row}
              disabled={busy || selectedGrouped}
              oninput={(event) =>
                setGeometry(card, "row", Number(event.currentTarget.value))}
            />
          </label>
          <label>
            <span>Width</span>
            <input
              id={inputId(card, "w")}
              type="number"
              min="1"
              max={Math.max(1, columns)}
              step="1"
              value={card.w}
              disabled={busy}
              oninput={(event) =>
                setGeometry(card, "w", Number(event.currentTarget.value))}
            />
          </label>
          <label>
            <span>Height</span>
            <input
              id={inputId(card, "h")}
              type="number"
              min="1"
              step="1"
              value={card.h}
              disabled={busy}
              oninput={(event) =>
                setGeometry(card, "h", Number(event.currentTarget.value))}
            />
          </label>
        </div>
        <div class="grid-size-row">
          <span class="grid-dim"
            >Size: {card.w} × {card.h} of {columns} columns</span
          >
          <span class="grid-spacer"></span>
          <button
            type="button"
            class="ghost tiny"
            aria-label="Shrink width"
            disabled={busy || card.w <= 1}
            onclick={() => resizeBy(card, -1, 0)}>−</button
          >
          <button
            type="button"
            class="ghost tiny"
            aria-label="Grow width"
            disabled={busy || card.w >= columns}
            onclick={() => resizeBy(card, 1, 0)}>+</button
          >
          <button
            type="button"
            class="ghost tiny"
            aria-label="Shrink height"
            disabled={busy || card.h <= 1}
            onclick={() => resizeBy(card, 0, -1)}>−</button
          >
          <button
            type="button"
            class="ghost tiny"
            aria-label="Grow height"
            disabled={busy}
            onclick={() => resizeBy(card, 0, 1)}>+</button
          >
        </div>
        {#if sizes.length}
          <div class="grid-size-row">
            <span class="grid-dim">Catalog size</span>
            {#each sizes as size (size.label)}
              <button
                type="button"
                class="ghost tiny"
                class:active={card.w === size.w && card.h === size.h}
                disabled={busy}
                onclick={() => applySize(card, size)}
              >
                {size.label}
              </button>
            {/each}
          </div>
        {/if}
      </div>

      {#if selectedRegistryFields.length}
        <div class="grid-details-section">
          <h5>Card settings</h5>
          {#each selectedRegistryFields as field (field.path)}
            <Control
              descriptor={descriptorFor(field)}
              value={values[field.path]}
              {onchange}
              expandDetails={Boolean(field.description)}
            />
          {/each}
        </div>
      {/if}

      {#if selectedExternalFields.length}
        <div class="grid-details-section">
          <h5>Card options</h5>
          {#each selectedExternalFields as field (field.path)}
            <Control
              descriptor={descriptorFor(field)}
              value={values[field.path]}
              {onchange}
              expandDetails={Boolean(field.description)}
            />
          {/each}
        </div>
      {/if}

      {#if card.id === "quickTiles"}
        <div class="grid-details-section">
          <h5>Quick tiles</h5>
          <p class="grid-dim small">
            Grouped tiles share this card. Select a tile below to edit it.
          </p>
          <div class="grid-members">
            {#each quickTilesMembers[card.record.path] ?? [] as member (member.record.path)}
              <button
                type="button"
                class="ghost tiny"
                class:active={selectedPath === member.record.path}
                onclick={() => selectById(member.record.path)}
              >
                {glyphFor(member.id)}
                {cardLabels[member.id] ?? caption(member.id)}
              </button>
            {:else}
              <span class="grid-dim">No quick tiles in this group.</span>
            {/each}
          </div>
        </div>
      {:else if selectedGrouped}
        <p class="grid-dim small">
          This tile is placed inside the {card.group} quick tiles card.
        </p>
      {/if}
    {:else}
      <p class="grid-dim">Select a card to edit it.</p>
    {/if}
  </section>
</div>

<style>
  .grid-editor {
    display: grid;
    gap: 0.75rem;
    min-width: 0;
  }

  .grid-adder {
    display: grid;
    gap: 0.4rem;
  }

  .grid-adder-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
  }

  .grid-adder-row input[type="search"] {
    width: min(16rem, 100%);
  }

  .grid-adder-row select {
    width: auto;
    min-width: 11rem;
  }

  .grid-adder button.tiny,
  .grid-size-row button.tiny,
  .grid-members button.tiny {
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
  }

  .grid-dim {
    color: var(--muted);
    font-size: 0.85rem;
  }

  .grid-spacer {
    flex: 1;
  }

  .grid-canvas {
    display: grid;
    grid-template-columns: repeat(var(--grid-columns), minmax(0, 1fr));
    grid-auto-rows: 72px;
    gap: 8px;
    min-height: 144px;
    padding: 8px;
    border-radius: 18px;
    background: color-mix(in srgb, var(--text) 4%, transparent);
  }

  .grid-canvas.grid-busy {
    opacity: 0.75;
  }

  .grid-empty {
    grid-column: 1 / -1;
    align-self: center;
    color: var(--muted);
    text-align: center;
  }

  .grid-card {
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 0.25rem;
    min-width: 0;
    padding: 0.55rem 0.65rem;
    border: 1px solid color-mix(in srgb, var(--text) 12%, transparent);
    border-radius: 14px;
    background: color-mix(in srgb, var(--text) 6%, var(--surface));
    color: var(--text);
    cursor: grab;
    touch-action: none;
    overflow: hidden;
    user-select: none;
  }

  .grid-card:hover {
    background: color-mix(in srgb, var(--text) 10%, var(--surface));
  }

  .grid-card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .grid-card-selected {
    border: 2px solid var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, var(--surface));
  }

  .grid-card-off {
    opacity: 0.4;
  }

  .grid-card-body {
    display: grid;
    justify-items: center;
    gap: 0.15rem;
    min-width: 0;
    text-align: center;
  }

  .grid-card-icon {
    font-size: 1.35rem;
    line-height: 1;
  }

  .grid-card-title {
    max-width: 100%;
    overflow: hidden;
    color: var(--text-strong);
    font-size: 0.9rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .grid-card-size {
    position: absolute;
    top: 5px;
    left: 7px;
    padding: 0.05rem 0.35rem;
    border-radius: 7px;
    background: color-mix(in srgb, var(--text) 82%, transparent);
    color: var(--bg);
    font-size: 0.68rem;
    font-weight: 700;
    line-height: 1.35;
  }

  .grid-card-members {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.2rem;
    min-width: 0;
  }

  .grid-member {
    padding: 0 0.15rem;
    border-radius: 5px;
    background: color-mix(in srgb, var(--text) 10%, transparent);
    font-size: 0.8rem;
  }

  .grid-handle {
    position: absolute;
    top: 4px;
    right: 4px;
    padding: 0 0.3rem;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    cursor: grab;
    touch-action: none;
  }

  .grid-handle:hover {
    background: color-mix(in srgb, var(--text) 12%, transparent);
    color: var(--text);
  }

  .grid-resize {
    position: absolute;
    padding: 0;
    border: 0;
    background: color-mix(in srgb, var(--accent) 55%, transparent);
    touch-action: none;
  }

  .grid-resize:hover {
    background: var(--accent);
  }

  .grid-resize-right {
    top: 12px;
    right: 0;
    bottom: 12px;
    width: 6px;
    border-radius: 3px;
    cursor: ew-resize;
  }

  .grid-resize-bottom {
    right: 12px;
    bottom: 0;
    left: 12px;
    height: 6px;
    border-radius: 3px;
    cursor: ns-resize;
  }

  .grid-resize-corner {
    right: 0;
    bottom: 0;
    width: 12px;
    height: 12px;
    border-radius: 4px 0 12px 0;
    cursor: nwse-resize;
  }

  .grid-details {
    display: grid;
    gap: 0.6rem;
    padding: 0.75rem 0.9rem;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
  }

  .grid-details-head {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .grid-details-icon {
    font-size: 1.5rem;
    line-height: 1;
  }

  .grid-details-title {
    display: grid;
    gap: 0.05rem;
    min-width: 0;
  }

  .grid-details-title h4 {
    margin: 0;
    color: var(--text-strong);
    font-size: 0.95rem;
  }

  .grid-details-title p {
    margin: 0;
  }

  .grid-details-section {
    display: grid;
    gap: 0.35rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border);
  }

  .grid-details-section h5 {
    margin: 0;
    color: var(--accent);
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .grid-geometry {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.4rem;
  }

  .grid-geometry label {
    display: grid;
    gap: 0.2rem;
    color: var(--muted);
    font-size: 0.78rem;
  }

  .grid-geometry input {
    width: 100%;
    padding: 0.3rem 0.45rem;
  }

  .grid-size-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
  }

  .grid-size-row button.active,
  .grid-members button.active {
    border-color: var(--accent);
    color: var(--accent);
  }

  .grid-members {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .grid-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    color: var(--text-strong);
    font-weight: 600;
  }

  .grid-toggle input {
    accent-color: var(--accent);
  }

  .grid-details :global(.field) {
    padding: 0.35rem 0;
  }

  .grid-details :global(.field-label) {
    font-size: 0.85rem;
  }

  .grid-details :global(.field-help) {
    font-size: 0.8rem;
  }

  @media (max-width: 760px) {
    .grid-geometry {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
