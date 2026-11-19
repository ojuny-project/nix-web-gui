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

  $: monitors = parent.children ?? [];
  $: maxX = Math.max(
    1,
    ...monitors.map(
      (monitor) => numberField(monitor, "xPos") + numberField(monitor, "width", 1920),
    ),
  );
  $: maxY = Math.max(
    1,
    ...monitors.map(
      (monitor) => numberField(monitor, "yPos") + numberField(monitor, "height", 1080),
    ),
  );

  function styleFor(monitor: ScanRecord): string {
    const x = numberField(monitor, "xPos");
    const y = numberField(monitor, "yPos");
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
    const x = fieldRecord(monitor, "xPos");
    const y = fieldRecord(monitor, "yPos");
    if (!x || !y) return;
    drag = {
      xPath: x.path,
      yPath: y.path,
      startX: event.clientX,
      startY: event.clientY,
      originX: numberField(monitor, "xPos"),
      originY: numberField(monitor, "yPos"),
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
  <div class="monitor-canvas">
    {#each monitors as monitor (monitor.path)}
      <div
        class="monitor-tile"
        role="group"
        aria-label={monitor.label ?? monitor.path}
        style={styleFor(monitor)}
        onpointermove={moveDrag}
        onpointerup={endDrag}
        onpointerdown={(event) => beginDrag(event, monitor)}
      >
        <strong>{monitor.label ?? monitor.path.split(".").at(-1)}</strong>
        <span class="muted small">
          {numberField(monitor, "width", 1920)}x{numberField(monitor, "height", 1080)}
          @ {numberField(monitor, "xPos")},{numberField(monitor, "yPos")}
        </span>
      </div>
    {/each}
  </div>
{/if}
