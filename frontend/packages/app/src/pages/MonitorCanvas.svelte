<script lang="ts">
  import type { ControlValue } from "@nix-web-gui/ui";
  import type { ScanRecord } from "../api";

  export let parent: ScanRecord;
  export let allRecords: ScanRecord[];
  export let values: Record<string, ControlValue>;
  export let onchange: (path: string, value: ControlValue) => void;

  let drag: {
    xPath: string;
    yPath: string;
    startX: number;
    startY: number;
    originX: number;
    originY: number;
  } | null = null;
  let draft: Record<string, number> = {};

  function fieldRecord(record: ScanRecord, field: string): ScanRecord | null {
    return (
      allRecords.find(
        (candidate) => candidate.path === `${record.path}.${field}`,
      ) ?? null
    );
  }

  function numberField(record: ScanRecord, field: string, fallback = 0): number {
    const key = `${record.path}.${field}`;
    if (draft[key] != null) return draft[key];
    const fieldRecord = allRecords.find(
      (candidate) => candidate.path === key,
    );
    if (!fieldRecord) return fallback;
    const value = Number(values[fieldRecord.path]);
    return Number.isFinite(value) ? value : fallback;
  }

  function axisField(record: ScanRecord, axis: string): ScanRecord | null {
    return fieldRecord(record, `${axis}Pos`) ?? fieldRecord(record, axis);
  }

  function axisValue(record: ScanRecord, axis: string): number {
    const field = axisField(record, axis);
    if (!field) return 0;
    const value = Number(values[field.path]);
    return Number.isFinite(value) ? value : 0;
  }

  $: monitors = parent.children ?? [];
  $: maxX = Math.max(
    1,
    ...monitors.map(
      (monitor) => axisValue(monitor, "x") + numberField(monitor, "width", 1920),
    ),
  );
  $: maxY = Math.max(
    1,
    ...monitors.map(
      (monitor) => axisValue(monitor, "y") + numberField(monitor, "height", 1080),
    ),
  );

  let selectedPath = "";
  let moved = false;

  function selectMonitor(monitor: ScanRecord) {
    selectedPath = monitor.path;
    const card = [...document.querySelectorAll<HTMLElement>(".row-card")].find(
      (element) => element.dataset.path === monitor.path,
    );
    card?.scrollIntoView({ block: "center", behavior: "smooth" });
  }

  function styleFor(monitor: ScanRecord): string {
    const x = axisValue(monitor, "x");
    const y = axisValue(monitor, "y");
    const width = numberField(monitor, "width", 1920);
    const height = numberField(monitor, "height", 1080);
    return [
      `left: ${(x / maxX) * 100}%`,
      `top: ${(y / maxY) * 100}%`,
      `width: ${(width / maxX) * 100}%`,
      `height: ${(height / maxY) * 100}%`,
    ].join("; ");
  }

  function beginDrag(event: PointerEvent, monitor: ScanRecord) {
    const x = axisField(monitor, "x");
    const y = axisField(monitor, "y");
    if (!x || !y) return;
    moved = false;
    drag = {
      xPath: x.path,
      yPath: y.path,
      startX: event.clientX,
      startY: event.clientY,
      originX: axisValue(monitor, "x"),
      originY: axisValue(monitor, "y"),
    };
    try {
      (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    } catch {
      // Synthetic pointer events in tests have no active pointer.
    }
  }

  function moveDrag(event: PointerEvent) {
    if (!drag) return;
    const container = (event.currentTarget as HTMLElement).closest(
      ".monitor-canvas",
    ) as HTMLElement | null;
    if (!container) return;
    const scaleX = maxX / container.clientWidth;
    const scaleY = maxY / container.clientHeight;
    if (
      Math.abs(event.clientX - drag.startX) > 3 ||
      Math.abs(event.clientY - drag.startY) > 3
    ) {
      moved = true;
    }
    draft = {
      ...draft,
      [drag.xPath]: Math.max(
        0,
        Math.round(drag.originX + (event.clientX - drag.startX) * scaleX),
      ),
      [drag.yPath]: Math.max(
        0,
        Math.round(drag.originY + (event.clientY - drag.startY) * scaleY),
      ),
    };
  }

  function endDrag(event: PointerEvent) {
    if (!drag) return;
    const xPath = drag.xPath;
    const yPath = drag.yPath;
    drag = null;
    if (draft[xPath] != null) onchange(xPath, draft[xPath]);
    if (draft[yPath] != null) onchange(yPath, draft[yPath]);
    try {
      (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    } catch {
      // Synthetic pointer events in tests have no active pointer.
    }
  }
</script>

{#if monitors.length}
  <div
    class="monitor-canvas"
    style={`aspect-ratio: ${maxX} / ${maxY};`}
  >
    {#each monitors as monitor (monitor.path)}
      <div
        class="monitor-tile"
        class:selected={selectedPath === monitor.path}
        role="button"
        tabindex="0"
        aria-label={monitor.label ?? monitor.path}
        style={styleFor(monitor)}
        onpointermove={moveDrag}
        onpointerup={endDrag}
        onpointerdown={(event) => beginDrag(event, monitor)}
        onclick={() => {
          if (!moved) selectMonitor(monitor);
          moved = false;
        }}
        onkeydown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            selectMonitor(monitor);
          }
        }}
      >
        <strong>{monitor.label ?? monitor.path.split(".").at(-1)}</strong>
        <span class="muted small">
          {numberField(monitor, "width", 1920)}x{numberField(monitor, "height", 1080)}
          @ {axisValue(monitor, "x")},{axisValue(monitor, "y")}
        </span>
      </div>
    {/each}
  </div>
{/if}
