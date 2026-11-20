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
  export let onstructurechange: () => Promise<void>;

  type FeatureSpec = {
    path: string;
    label: string;
    group: string;
    widget: ControlDescriptor["widget"];
    choices?: string[];
    min?: number;
    max?: number;
    step?: number;
    digits?: number;
    unit?: string;
  };

  type FieldEntry = {
    feature: FeatureSpec;
    record: ScanRecord;
  };

  type Tile = {
    monitor: ScanRecord;
    name: string;
    x: number;
    y: number;
    footprintW: number;
    footprintH: number;
    physicalW: number;
    physicalH: number;
    transform: string;
    enabled: boolean;
    primary: boolean;
    sized: boolean;
  };

  type Bounds = {
    minX: number;
    minY: number;
    maxX: number;
    maxY: number;
    spanX: number;
    spanY: number;
  };

  type DetectedOutput = {
    name: string;
    make?: string | null;
    model?: string | null;
    enabled?: boolean;
    width?: number | null;
    height?: number | null;
    refresh?: number | null;
  };

  type Mode = { width: number; height: number; refresh: number };

  const TRANSFORMS = [
    "normal",
    "90",
    "180",
    "270",
    "flipped",
    "flipped-90",
    "flipped-180",
    "flipped-270",
  ];

  const GEOMETRY_PATHS = new Set([
    "width",
    "height",
    "refresh",
    "xPos",
    "yPos",
    "scale",
    "transform",
  ]);

  const GROUPS = [
    { key: "match", label: "Identity" },
    { key: "color", label: "Color" },
    { key: "hdr", label: "HDR" },
    { key: "sync", label: "Sync" },
    { key: "render", label: "Rendering" },
    { key: "signal", label: "Signal" },
    { key: "reserved", label: "Reserved area" },
    { key: "hotCorners", label: "Hot corners" },
    { key: "background", label: "Background" },
  ];

  const FEATURES: FeatureSpec[] = [
    { path: "width", label: "Width", group: "", widget: "spin", min: 0, max: 16384, step: 1, digits: 0, unit: "px" },
    { path: "height", label: "Height", group: "", widget: "spin", min: 0, max: 16384, step: 1, digits: 0, unit: "px" },
    { path: "refresh", label: "Refresh", group: "", widget: "spin", min: 0, max: 1000, step: 0.01, digits: 3, unit: "Hz" },
    { path: "scale", label: "Scale", group: "", widget: "spin", min: 0.01, max: 16, step: 0.05, digits: 2 },
    { path: "xPos", label: "X position", group: "", widget: "spin", min: -16384, max: 16384, step: 1, digits: 0, unit: "px" },
    { path: "yPos", label: "Y position", group: "", widget: "spin", min: -16384, max: 16384, step: 1, digits: 0, unit: "px" },
    { path: "transform", label: "Transform", group: "", widget: "dropdown", choices: TRANSFORMS },
    { path: "enabled", label: "Enabled", group: "", widget: "switch" },
    { path: "primary", label: "Primary", group: "", widget: "switch" },
    { path: "mirror", label: "Mirror output", group: "", widget: "text" },
    { path: "priority", label: "Priority", group: "", widget: "spin", min: 0, max: 100, step: 1, digits: 0 },
    { path: "customMode", label: "Custom mode", group: "", widget: "switch" },
    { path: "modeline", label: "Modeline", group: "", widget: "text" },
    { path: "power", label: "Power", group: "", widget: "switch" },
    { path: "reflection", label: "Reflection", group: "", widget: "dropdown", choices: ["none", "horizontal", "vertical", "both"] },
    { path: "focusAtStartup", label: "Focus at startup", group: "", widget: "switch" },
    { path: "brightness", label: "Brightness", group: "", widget: "spin", min: 0, max: 2, step: 0.01, digits: 2 },
    { path: "autoRotation", label: "Auto rotation", group: "", widget: "dropdown", choices: ["automatic", "never"] },
    { path: "autoHdr", label: "Auto HDR", group: "", widget: "switch" },
    { path: "match.make", label: "Make", group: "match", widget: "text" },
    { path: "match.model", label: "Model", group: "match", widget: "text" },
    { path: "match.serial", label: "Serial", group: "match", widget: "text" },
    { path: "color.profile", label: "Profile", group: "color", widget: "dropdown", choices: ["default", "srgb", "gamma22", "wide", "hdr", "automatic", "edid", "icc"] },
    { path: "color.icc", label: "ICC profile", group: "color", widget: "path" },
    { path: "color.wideGamut", label: "Wide gamut", group: "color", widget: "dropdown", choices: ["auto", "on", "off"] },
    { path: "color.bitDepth", label: "Bit depth", group: "color", widget: "dropdown", choices: ["6", "8", "10", "12", "14", "16"] },
    { path: "hdr.enabled", label: "HDR", group: "hdr", widget: "switch" },
    { path: "hdr.force", label: "Force HDR", group: "hdr", widget: "switch" },
    { path: "hdr.minLuminance", label: "Min luminance", group: "hdr", widget: "spin", min: 0, max: 10000, step: 1, digits: 2, unit: "cd/m2" },
    { path: "hdr.maxLuminance", label: "Max luminance", group: "hdr", widget: "spin", min: 0, max: 10000, step: 1, digits: 2, unit: "cd/m2" },
    { path: "hdr.maxAvgLuminance", label: "Max average luminance", group: "hdr", widget: "spin", min: 0, max: 10000, step: 1, digits: 2, unit: "cd/m2" },
    { path: "hdr.sdrBrightness", label: "SDR brightness", group: "hdr", widget: "spin", min: 0, max: 10, step: 0.01, digits: 2 },
    { path: "hdr.sdrSaturation", label: "SDR saturation", group: "hdr", widget: "spin", min: 0, max: 10, step: 0.01, digits: 2 },
    { path: "hdr.sdrMinLuminance", label: "SDR min luminance", group: "hdr", widget: "spin", min: 0, max: 10000, step: 0.01, digits: 2, unit: "cd/m2" },
    { path: "hdr.sdrMaxLuminance", label: "SDR max luminance", group: "hdr", widget: "spin", min: 0, max: 10000, step: 1, digits: 2, unit: "cd/m2" },
    { path: "hdr.sdrEotf", label: "SDR EOTF", group: "hdr", widget: "dropdown", choices: ["default", "gamma22", "srgb"] },
    { path: "sync.vrr", label: "VRR", group: "sync", widget: "dropdown", choices: ["off", "on", "on-demand", "automatic", "always", "never"] },
    { path: "sync.allowTearing", label: "Allow tearing", group: "sync", widget: "switch" },
    { path: "sync.maxRenderTime", label: "Max render time", group: "sync", widget: "spin", min: 0, max: 1000, step: 1, digits: 0, unit: "ms" },
    { path: "render.scaleFilter", label: "Scale filter", group: "render", widget: "dropdown", choices: ["linear", "nearest", "smart"] },
    { path: "render.subpixel", label: "Subpixel", group: "render", widget: "dropdown", choices: ["rgb", "bgr", "vrgb", "vbgr", "none"] },
    { path: "signal.overscan", label: "Overscan", group: "signal", widget: "spin", min: 0, max: 100, step: 1, digits: 0, unit: "%" },
    { path: "signal.rgbRange", label: "RGB range", group: "signal", widget: "dropdown", choices: ["automatic", "full", "limited"] },
    { path: "reserved.top", label: "Top", group: "reserved", widget: "spin", min: 0, max: 1000, step: 1, digits: 0, unit: "px" },
    { path: "reserved.right", label: "Right", group: "reserved", widget: "spin", min: 0, max: 1000, step: 1, digits: 0, unit: "px" },
    { path: "reserved.bottom", label: "Bottom", group: "reserved", widget: "spin", min: 0, max: 1000, step: 1, digits: 0, unit: "px" },
    { path: "reserved.left", label: "Left", group: "reserved", widget: "spin", min: 0, max: 1000, step: 1, digits: 0, unit: "px" },
    { path: "hotCorners.topLeft", label: "Top left", group: "hotCorners", widget: "switch" },
    { path: "hotCorners.topRight", label: "Top right", group: "hotCorners", widget: "switch" },
    { path: "hotCorners.bottomLeft", label: "Bottom left", group: "hotCorners", widget: "switch" },
    { path: "hotCorners.bottomRight", label: "Bottom right", group: "hotCorners", widget: "switch" },
    { path: "background.image", label: "Image", group: "background", widget: "path" },
    { path: "background.color", label: "Color", group: "background", widget: "color" },
    { path: "background.backdropColor", label: "Backdrop color", group: "background", widget: "color" },
    { path: "background.mode", label: "Mode", group: "background", widget: "dropdown", choices: ["stretch", "fill", "fit", "center", "tile"] },
  ];

  let selectedPath = "";
  let draft: Record<string, number> = {};
  let zoom = 1;
  let panX = 0;
  let panY = 0;
  let canvasWidth = 640;
  let canvasHeight = 400;
  let sceneElement: HTMLElement | null = null;
  let busy = false;
  let error = "";
  let status = "";
  let pickerOpen = false;
  let detecting = false;
  let detected: DetectedOutput[] = [];
  let detectError = "";
  let manualName = "";
  let manualWidth = 1920;
  let manualHeight = 1080;
  let manualRefresh = 60;

  let drag: {
    xPath: string | null;
    yPath: string | null;
    startX: number;
    startY: number;
    originX: number;
    originY: number;
    scaleX: number;
    scaleY: number;
    moved: boolean;
  } | null = null;

  let monitors: ScanRecord[] = [];
  let selected: ScanRecord | null = null;
  let disabledEntries: Set<string> = new Set();
  let tiles: Tile[] = [];
  let bounds: Bounds = { minX: 0, minY: 0, maxX: 1, maxY: 1, spanX: 1, spanY: 1 };
  let liveBounds: Bounds = { minX: 0, minY: 0, maxX: 1, maxY: 1, spanX: 1, spanY: 1 };
  let fit = 1;
  let sceneWidth = 1;
  let sceneHeight = 1;
  let sceneStyle = "";
  let selectedTile: Tile | null = null;
  let geometryFields: FieldEntry[] = [];
  let topFields: FieldEntry[] = [];
  let groupedFields: Array<{
    key: string;
    label: string;
    fields: FieldEntry[];
  }> = [];
  let missingFeatures: FeatureSpec[] = [];

  let pan: {
    startX: number;
    startY: number;
    originX: number;
    originY: number;
  } | null = null;

  function leafName(path: string): string {
    return path.split(".").at(-1) ?? path;
  }

  function messageOf(cause: unknown, fallback: string): string {
    return cause instanceof Error ? cause.message : fallback;
  }

  function measureCanvas(node: HTMLElement): { destroy: () => void } {
    const measure = () => {
      canvasWidth = node.clientWidth;
      canvasHeight = node.clientHeight;
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(node);
    return {
      destroy() {
        observer.disconnect();
      },
    };
  }

  function fieldRecord(monitor: ScanRecord, field: string): ScanRecord | null {
    return (
      allRecords.find(
        (record) => record.path === `${monitor.path}.${field}`,
      ) ?? null
    );
  }

  function resolveField(
    monitor: ScanRecord,
    field: string,
  ): ScanRecord | null {
    if (field === "xPos") {
      return fieldRecord(monitor, "xPos") ?? fieldRecord(monitor, "x");
    }
    if (field === "yPos") {
      return fieldRecord(monitor, "yPos") ?? fieldRecord(monitor, "y");
    }
    return fieldRecord(monitor, field);
  }

  function recordNumber(
    record: ScanRecord | null,
    fallback: number,
    currentValues: Record<string, ControlValue>,
    currentDraft: Record<string, number>,
  ): number {
    if (!record) return fallback;
    const pending = currentDraft[record.path];
    if (pending != null) return pending;
    const parsed = Number(currentValues[record.path]);
    return Number.isFinite(parsed) ? parsed : fallback;
  }

  function numberFor(
    monitor: ScanRecord,
    field: string,
    fallback: number,
    currentValues: Record<string, ControlValue>,
    currentDraft: Record<string, number>,
  ): number {
    return recordNumber(
      resolveField(monitor, field),
      fallback,
      currentValues,
      currentDraft,
    );
  }

  function booleanFor(
    monitor: ScanRecord,
    field: string,
    fallback: boolean,
    currentValues: Record<string, ControlValue>,
  ): boolean {
    const record = resolveField(monitor, field);
    if (!record) return fallback;
    const value = currentValues[record.path];
    if (typeof value === "boolean") return value;
    return String(value ?? "").trim() === "true";
  }

  function transformFor(
    monitor: ScanRecord,
    currentValues: Record<string, ControlValue>,
  ): string {
    const record = fieldRecord(monitor, "transform");
    if (!record) return "normal";
    const value = currentValues[record.path];
    const text = typeof value === "string" ? value : String(value ?? "normal");
    return TRANSFORMS.includes(text) ? text : "normal";
  }

  function oddTransform(transform: string): boolean {
    return (
      transform === "90" ||
      transform === "270" ||
      transform === "flipped-90" ||
      transform === "flipped-270"
    );
  }

  function tileFor(
    monitor: ScanRecord,
    currentValues: Record<string, ControlValue>,
    currentDraft: Record<string, number>,
  ): Tile {
    const rawWidth = recordNumber(
      resolveField(monitor, "width"),
      0,
      currentValues,
      currentDraft,
    );
    const rawHeight = recordNumber(
      resolveField(monitor, "height"),
      0,
      currentValues,
      currentDraft,
    );
    const sized = rawWidth > 0 && rawHeight > 0;
    const physicalW = rawWidth > 0 ? rawWidth : 1920;
    const physicalH = rawHeight > 0 ? rawHeight : 1080;
    const scale = Math.max(
      0.01,
      numberFor(monitor, "scale", 1, currentValues, currentDraft),
    );
    const transform = transformFor(monitor, currentValues);
    const [width, height] = oddTransform(transform)
      ? [physicalH, physicalW]
      : [physicalW, physicalH];
    return {
      monitor,
      name: monitor.label ?? leafName(monitor.path),
      x: numberFor(monitor, "xPos", 0, currentValues, currentDraft),
      y: numberFor(monitor, "yPos", 0, currentValues, currentDraft),
      footprintW: width / scale,
      footprintH: height / scale,
      physicalW,
      physicalH,
      transform,
      enabled: booleanFor(monitor, "enabled", true, currentValues),
      primary: booleanFor(monitor, "primary", false, currentValues),
      sized,
    };
  }

  function sceneBounds(tiles: Tile[]): Bounds {
    if (tiles.length === 0) {
      return { minX: 0, minY: 0, maxX: 1, maxY: 1, spanX: 1, spanY: 1 };
    }
    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    for (const tile of tiles) {
      minX = Math.min(minX, tile.x);
      minY = Math.min(minY, tile.y);
      maxX = Math.max(maxX, tile.x + tile.footprintW);
      maxY = Math.max(maxY, tile.y + tile.footprintH);
    }
    return {
      minX,
      minY,
      maxX,
      maxY,
      spanX: Math.max(1, maxX - minX),
      spanY: Math.max(1, maxY - minY),
    };
  }

  function tileStyle(tile: Tile, bounds: Bounds): string {
    return [
      `left: ${((tile.x - bounds.minX) / bounds.spanX) * 100}%`,
      `top: ${((tile.y - bounds.minY) / bounds.spanY) * 100}%`,
      `width: ${(tile.footprintW / bounds.spanX) * 100}%`,
      `height: ${(tile.footprintH / bounds.spanY) * 100}%`,
    ].join("; ");
  }

  function faceStyle(tile: Tile): string {
    const odd = oddTransform(tile.transform);
    const width = odd ? (tile.physicalW / tile.physicalH) * 100 : 100;
    const height = odd ? (tile.physicalH / tile.physicalW) * 100 : 100;
    return [
      `width: ${width}%`,
      `height: ${height}%`,
      `transform: ${faceTransform(tile.transform)}`,
    ].join("; ");
  }

  function faceTransform(transform: string): string {
    switch (transform) {
      case "90":
        return "rotate(90deg)";
      case "180":
        return "rotate(180deg)";
      case "270":
        return "rotate(270deg)";
      case "flipped":
        return "scaleX(-1)";
      case "flipped-90":
        return "rotate(90deg) scaleX(-1)";
      case "flipped-180":
        return "rotate(180deg) scaleX(-1)";
      case "flipped-270":
        return "rotate(270deg) scaleX(-1)";
      default:
        return "none";
    }
  }

  function disabledPath(entries: Set<string>, path: string): boolean {
    if (entries.has(path)) return true;
    const dot = path.indexOf(".");
    return dot > 0 && entries.has(path.slice(0, dot));
  }

  function fieldsFor(
    monitor: ScanRecord | null,
    group: string,
    entries: Set<string>,
  ): FieldEntry[] {
    if (!monitor) return [];
    const out: FieldEntry[] = [];
    for (const feature of FEATURES) {
      if (feature.group !== group) continue;
      if (disabledPath(entries, feature.path)) continue;
      const record = resolveField(monitor, feature.path);
      if (record) out.push({ feature, record });
    }
    return out;
  }

  function missingFeaturesFor(
    monitor: ScanRecord | null,
    entries: Set<string>,
  ): FeatureSpec[] {
    if (!monitor) return [];
    return FEATURES.filter(
      (feature) =>
        !disabledPath(entries, feature.path) &&
        resolveField(monitor, feature.path) === null,
    );
  }

  function descriptorFor(entry: FieldEntry): ControlDescriptor {
    const { feature, record } = entry;
    return {
      path: record.path,
      widget: feature.widget,
      label: feature.label,
      description: null,
      choices: feature.choices ?? record.choices ?? null,
      min: feature.min ?? record.min ?? null,
      max: feature.max ?? record.max ?? null,
      step: feature.step ?? record.step ?? null,
      digits: feature.digits ?? record.digits ?? null,
      unit: feature.unit ?? record.unit ?? null,
      readonly: record.readonly === true,
      origin: null,
    };
  }

  function selectMonitor(monitor: ScanRecord): void {
    selectedPath = monitor.path;
  }

  function selectRelative(offset: number): void {
    if (monitors.length === 0) return;
    const index = monitors.findIndex(
      (monitor) => monitor.path === selectedPath,
    );
    const next = Math.min(
      monitors.length - 1,
      Math.max(0, (index < 0 ? 0 : index) + offset),
    );
    selectedPath = monitors[next].path;
  }

  function beginTileDrag(event: PointerEvent, tile: Tile): void {
    event.stopPropagation();
    selectMonitor(tile.monitor);
    if (event.button !== 0) return;
    const xField = resolveField(tile.monitor, "xPos");
    const yField = resolveField(tile.monitor, "yPos");
    if (!xField && !yField) return;
    event.preventDefault();
    const rect = sceneElement?.getBoundingClientRect();
    drag = {
      xPath: xField?.path ?? null,
      yPath: yField?.path ?? null,
      startX: event.clientX,
      startY: event.clientY,
      originX: numberFor(tile.monitor, "xPos", 0, values, draft),
      originY: numberFor(tile.monitor, "yPos", 0, values, draft),
      scaleX: rect && rect.width > 0 ? bounds.spanX / rect.width : 1,
      scaleY: rect && rect.height > 0 ? bounds.spanY / rect.height : 1,
      moved: false,
    };
    try {
      (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    } catch {
      // Synthetic pointer events have no active pointer.
    }
  }

  function moveTileDrag(event: PointerEvent): void {
    if (!drag) return;
    const dx = event.clientX - drag.startX;
    const dy = event.clientY - drag.startY;
    if (Math.abs(dx) > 3 || Math.abs(dy) > 3) drag.moved = true;
    if (!drag.moved) return;
    const next = { ...draft };
    if (drag.xPath) {
      next[drag.xPath] = Math.round(drag.originX + dx * drag.scaleX);
    }
    if (drag.yPath) {
      next[drag.yPath] = Math.round(drag.originY + dy * drag.scaleY);
    }
    draft = next;
  }

  function endTileDrag(event: PointerEvent): void {
    const pending = drag;
    drag = null;
    try {
      (event.currentTarget as HTMLElement).releasePointerCapture(
        event.pointerId,
      );
    } catch {
      // Synthetic pointer events have no active pointer.
    }
    if (!pending || !pending.moved) return;
    if (pending.xPath && draft[pending.xPath] != null) {
      onchange(pending.xPath, draft[pending.xPath]);
    }
    if (pending.yPath && draft[pending.yPath] != null) {
      onchange(pending.yPath, draft[pending.yPath]);
    }
  }

  function beginPan(event: PointerEvent): void {
    if (event.button !== 0) return;
    pan = {
      startX: event.clientX,
      startY: event.clientY,
      originX: panX,
      originY: panY,
    };
    try {
      (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    } catch {
      // Synthetic pointer events have no active pointer.
    }
  }

  function movePan(event: PointerEvent): void {
    if (!pan) return;
    panX = pan.originX + (event.clientX - pan.startX);
    panY = pan.originY + (event.clientY - pan.startY);
  }

  function endPan(event: PointerEvent): void {
    pan = null;
    try {
      (event.currentTarget as HTMLElement).releasePointerCapture(
        event.pointerId,
      );
    } catch {
      // Synthetic pointer events have no active pointer.
    }
  }

  function zoomCanvas(event: WheelEvent): void {
    event.preventDefault();
    const factor = event.deltaY < 0 ? 1.15 : 1 / 1.15;
    const next = Math.min(8, Math.max(0.25, zoom * factor));
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const dx = event.clientX - (rect.left + rect.width / 2);
    const dy = event.clientY - (rect.top + rect.height / 2);
    panX = dx - next * ((dx - panX) / zoom);
    panY = dy - next * ((dy - panY) / zoom);
    zoom = next;
  }

  function resetView(): void {
    zoom = 1;
    panX = 0;
    panY = 0;
  }

  function recenterMonitor(): void {
    const monitor = selected;
    if (!monitor) return;
    const xField = resolveField(monitor, "xPos");
    const yField = resolveField(monitor, "yPos");
    if (xField) onchange(xField.path, 0);
    if (yField) onchange(yField.path, 0);
    status = `Moved ${monitor.label ?? leafName(monitor.path)} to 0, 0`;
  }

  function configuredOutput(name: string): boolean {
    return monitors.some((monitor) => leafName(monitor.path) === name);
  }

  async function openPicker(): Promise<void> {
    pickerOpen = true;
    error = "";
    detectError = "";
    detecting = true;
    detected = [];
    try {
      const result = await rpc<{ outputs: DetectedOutput[] }>(
        "monitor.detect",
        {},
        csrf,
      );
      detected = result.outputs ?? [];
    } catch (cause) {
      detectError = messageOf(cause, "Cannot detect outputs");
    } finally {
      detecting = false;
    }
  }

  function closePicker(): void {
    pickerOpen = false;
    manualName = "";
    detected = [];
    detectError = "";
  }

  function modeOf(output: DetectedOutput): Mode | null {
    if (
      output.width == null ||
      output.height == null ||
      output.refresh == null
    ) {
      return null;
    }
    return {
      width: Math.round(output.width),
      height: Math.round(output.height),
      refresh: output.refresh,
    };
  }

  async function addMonitor(name: string, mode: Mode | null): Promise<void> {
    const key = name.trim();
    if (!key) return;
    busy = true;
    error = "";
    status = "";
    try {
      await rpc(
        "structure.add",
        { path: parent.path, key, mode, defaults: {} },
        csrf,
      );
      selectedPath = `${parent.path}.${key}`;
      closePicker();
      await onstructurechange();
      status = `Added ${key}`;
    } catch (cause) {
      error = messageOf(cause, "Cannot add the monitor");
    } finally {
      busy = false;
    }
  }

  async function addDetected(output: DetectedOutput): Promise<void> {
    if (configuredOutput(output.name)) return;
    await addMonitor(output.name, modeOf(output));
  }

  async function addManual(): Promise<void> {
    const width = Number(manualWidth);
    const height = Number(manualHeight);
    const refresh = Number(manualRefresh);
    const mode: Mode = {
      width: Math.max(1, Math.round(Number.isFinite(width) ? width : 1920)),
      height: Math.max(1, Math.round(Number.isFinite(height) ? height : 1080)),
      refresh: Number.isFinite(refresh) && refresh > 0 ? refresh : 60,
    };
    await addMonitor(manualName, mode);
  }

  async function removeSelected(): Promise<void> {
    const monitor = selected;
    if (!monitor) return;
    const name = monitor.label ?? leafName(monitor.path);
    if (!confirm(`Remove monitor ${name}?`)) return;
    busy = true;
    error = "";
    status = "";
    try {
      await rpc(
        "structure.remove",
        { parent: parent.path, child: monitor.path },
        csrf,
      );
      await onstructurechange();
      status = `Removed ${name}`;
    } catch (cause) {
      error = messageOf(cause, "Cannot remove the monitor");
    } finally {
      busy = false;
    }
  }

  async function renameSelected(): Promise<void> {
    const monitor = selected;
    if (!monitor) return;
    const current = leafName(monitor.path);
    const next = prompt(
      "New output name. The output key is the identity used by the compositor.",
      current,
    );
    if (!next || next.trim() === current) return;
    busy = true;
    error = "";
    status = "";
    try {
      await rpc(
        "structure.rename",
        { parent: parent.path, child: monitor.path, key: next.trim() },
        csrf,
      );
      selectedPath = `${parent.path}.${next.trim()}`;
      await onstructurechange();
      status = `Renamed ${current} to ${next.trim()}`;
    } catch (cause) {
      error = messageOf(cause, "Cannot rename the monitor");
    } finally {
      busy = false;
    }
  }

  $: monitors = parent.children ?? [];

  $: if (
    monitors.length > 0 &&
    !monitors.some((monitor) => monitor.path === selectedPath)
  ) {
    selectedPath = monitors[0].path;
  }

  $: selected =
    monitors.find((monitor) => monitor.path === selectedPath) ?? null;

  $: disabledEntries = new Set(parent.disabledFeatures ?? []);

  $: tiles = allRecords
    ? monitors.map((monitor) => tileFor(monitor, values, draft))
    : [];

  $: liveBounds = sceneBounds(tiles);

  $: if (!drag) bounds = liveBounds;

  $: fit = Math.min(
    Math.max(1, canvasWidth - 24) / bounds.spanX,
    Math.max(1, canvasHeight - 24) / bounds.spanY,
  );

  $: sceneWidth = Math.max(1, bounds.spanX * fit);

  $: sceneHeight = Math.max(1, bounds.spanY * fit);

  $: sceneStyle = [
    `width: ${sceneWidth}px`,
    `height: ${sceneHeight}px`,
    `transform: translate(${panX}px, ${panY}px) scale(${zoom})`,
  ].join("; ");

  $: selectedTile =
    tiles.find((tile) => tile.monitor.path === selected?.path) ?? null;

  $: geometryFields = allRecords
    ? fieldsFor(selected, "", disabledEntries).filter((entry) =>
        GEOMETRY_PATHS.has(entry.feature.path),
      )
    : [];

  $: topFields = allRecords
    ? fieldsFor(selected, "", disabledEntries).filter(
        (entry) => !GEOMETRY_PATHS.has(entry.feature.path),
      )
    : [];

  $: groupedFields = allRecords
    ? GROUPS.map((group) => ({
        ...group,
        fields: fieldsFor(selected, group.key, disabledEntries),
      })).filter((group) => group.fields.length > 0)
    : [];

  $: missingFeatures = allRecords
    ? missingFeaturesFor(selected, disabledEntries)
    : [];

  $: if (Object.keys(draft).length > 0) {
    const next = { ...draft };
    let changed = false;
    for (const [path, pending] of Object.entries(draft)) {
      if (Math.abs(Number(values[path]) - pending) < 1e-9) {
        delete next[path];
        changed = true;
      }
    }
    if (changed) draft = next;
  }
</script>

<div class="display-editor">
  {#if monitors.length === 0}
    <div class="empty-state">
      <h4>No monitors configured</h4>
      <p class="muted small">
        Add an output to describe its resolution, position, and features.
      </p>
      <button class="ghost" type="button" disabled={busy} onclick={openPicker}>
        Add monitor
      </button>
    </div>
  {:else}
    <div class="workspace">
      <div class="canvas-pane">
        <div
          class="canvas"
          use:measureCanvas
          role="group"
          aria-label="Monitor layout"
          onpointerdown={beginPan}
          onpointermove={movePan}
          onpointerup={endPan}
          onpointercancel={endPan}
          onwheel={zoomCanvas}
        >
          <div
            class="scene"
            bind:this={sceneElement}
            style={sceneStyle}
          >
            {#each tiles as tile (tile.monitor.path)}
              <div
                class="monitor-tile"
                class:selected={tile.monitor.path === selectedPath}
                class:primary={tile.primary}
                class:disabled={!tile.enabled}
                class:unsized={!tile.sized}
                style={tileStyle(tile, bounds)}
                role="button"
                tabindex="0"
                aria-label={`${tile.name}, ${tile.physicalW}x${tile.physicalH}, ${tile.x},${tile.y}${tile.primary ? ", primary" : ""}`}
                onpointerdown={(event) => beginTileDrag(event, tile)}
                onpointermove={moveTileDrag}
                onpointerup={endTileDrag}
                onpointercancel={endTileDrag}
                onclick={() => selectMonitor(tile.monitor)}
                onkeydown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    selectMonitor(tile.monitor);
                  }
                }}
              >
                <div class="monitor-face" style={faceStyle(tile)}>
                  <span class="face-marker"></span>
                </div>
                <div class="monitor-label">
                  <strong>{tile.name}</strong>
                  <span class="muted small">
                    {tile.physicalW}x{tile.physicalH} @ {tile.x},{tile.y}
                  </span>
                </div>
              </div>
            {/each}
          </div>
        </div>
        <div class="canvas-actions">
          <button class="ghost" type="button" disabled={busy} onclick={openPicker}>
            Add monitor
          </button>
          <button
            class="ghost"
            type="button"
            disabled={busy || !selected}
            onclick={() => selectRelative(-1)}
          >
            Previous
          </button>
          <button
            class="ghost"
            type="button"
            disabled={busy || !selected}
            onclick={() => selectRelative(1)}
          >
            Next
          </button>
          <button
            class="ghost"
            type="button"
            disabled={busy || !selected}
            onclick={recenterMonitor}
          >
            Re-center monitor
          </button>
          <button class="ghost" type="button" onclick={resetView}>
            Re-center view
          </button>
          <button
            class="ghost"
            type="button"
            disabled={busy || !selected}
            onclick={renameSelected}
          >
            Rename
          </button>
          <button
            class="ghost destructive"
            type="button"
            disabled={busy || !selected}
            onclick={removeSelected}
          >
            Remove monitor
          </button>
        </div>
        <div class="legend muted small">
          <span><i class="swatch selected"></i> selected</span>
          <span><i class="swatch primary"></i> primary</span>
          <span><i class="swatch unsized"></i> size not set</span>
          <span>drag to move, scroll to zoom</span>
        </div>
      </div>

      {#if selected && selectedTile}
        <aside class="details">
          <header class="details-head">
            <div>
              <h4>{selectedTile.name}</h4>
              <p class="muted small">
                {selectedTile.physicalW}x{selectedTile.physicalH}
                {selectedTile.transform !== "normal"
                  ? ` · ${selectedTile.transform}`
                  : ""}
                {selectedTile.enabled ? "" : " · disabled"}
              </p>
            </div>
            <div class="badges">
              {#if selectedTile.primary}
                <span class="badge primary">Primary</span>
              {/if}
              {#if !selectedTile.sized}
                <span class="badge warning">Unsized</span>
              {/if}
            </div>
          </header>

          {#if geometryFields.length > 0}
            <section>
              <h5>Geometry</h5>
              <div class="field-grid">
                {#each geometryFields as entry (entry.record.path)}
                  <Control
                    descriptor={descriptorFor(entry)}
                    value={values[entry.record.path]}
                    {onchange}
                  />
                {/each}
              </div>
            </section>
          {/if}

          {#if topFields.length > 0}
            <section>
              <h5>Output</h5>
              <div class="field-grid">
                {#each topFields as entry (entry.record.path)}
                  <Control
                    descriptor={descriptorFor(entry)}
                    value={values[entry.record.path]}
                    {onchange}
                  />
                {/each}
              </div>
            </section>
          {/if}

          {#each groupedFields as group (group.key)}
            <details class="group">
              <summary>{group.label}</summary>
              <div class="field-grid">
                {#each group.fields as entry (entry.record.path)}
                  <Control
                    descriptor={descriptorFor(entry)}
                    value={values[entry.record.path]}
                    {onchange}
                  />
                {/each}
              </div>
            </details>
          {/each}

          {#if missingFeatures.length > 0}
            <details class="group missing">
              <summary>
                Not present in this output ({missingFeatures.length})
              </summary>
              <p class="muted small">
                {missingFeatures.map((feature) => feature.label).join(", ")}
              </p>
              <p class="muted small">
                The web editor can only edit fields that already exist in the
                attribute set. Add them in the Nix file first.
              </p>
            </details>
          {/if}
        </aside>
      {/if}
    </div>
  {/if}

  {#if pickerOpen}
    <div class="picker">
      <header class="picker-head">
        <strong>Add monitor</strong>
        <button class="ghost tiny" type="button" onclick={closePicker}>
          Close
        </button>
      </header>

      {#if detecting}
        <p class="muted small">Detecting outputs...</p>
      {:else if detected.length > 0}
        <ul class="output-list">
          {#each detected as output (output.name)}
            <li>
              <button
                class="output"
                type="button"
                disabled={busy || configuredOutput(output.name)}
                onclick={() => addDetected(output)}
              >
                <strong>{output.name}</strong>
                <span class="muted small">
                  {[output.make, output.model].filter(Boolean).join(" ") ||
                    "Unknown model"}
                  {output.enabled === false ? " (disabled)" : ""}
                  {configuredOutput(output.name) ? " (configured)" : ""}
                </span>
                <span class="muted small">
                  {output.width ?? 1920}x{output.height ?? 1080} @
                  {output.refresh ?? 60} Hz
                </span>
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="muted small">
          {detectError || "No outputs detected. Add a monitor manually."}
        </p>
      {/if}

      <div class="manual">
        <label>
          Name
          <input bind:value={manualName} placeholder="DP-1" />
        </label>
        <label>
          Width
          <input type="number" min="1" max="16384" bind:value={manualWidth} />
        </label>
        <label>
          Height
          <input type="number" min="1" max="16384" bind:value={manualHeight} />
        </label>
        <label>
          Refresh
          <input type="number" min="0" max="1000" step="0.01" bind:value={manualRefresh} />
        </label>
        <button
          class="ghost"
          type="button"
          disabled={busy || !manualName.trim()}
          onclick={addManual}
        >
          Add manually
        </button>
      </div>
    </div>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}
  {#if status}
    <p class="muted small">{status}</p>
  {/if}
</div>

<style>
  .display-editor {
    display: grid;
    gap: 0.75rem;
  }

  .workspace {
    display: grid;
    grid-template-columns: minmax(0, 1.35fr) minmax(19rem, 1fr);
    gap: 1rem;
    align-items: start;
  }

  .canvas-pane {
    display: grid;
    gap: 0.5rem;
    min-width: 0;
  }

  .canvas {
    position: relative;
    width: 100%;
    height: clamp(260px, 50vh, 560px);
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 12px;
    background:
      radial-gradient(
        circle at 50% 40%,
        color-mix(in srgb, var(--accent) 8%, transparent),
        transparent 60%
      ),
      color-mix(in srgb, var(--bg) 70%, var(--surface));
    cursor: grab;
    touch-action: none;
    user-select: none;
  }

  .canvas:active {
    cursor: grabbing;
  }

  .scene {
    position: absolute;
    inset: 0;
    margin: auto;
    transform-origin: center center;
  }

  .monitor-tile {
    position: absolute;
    display: grid;
    place-items: center;
    min-width: 18px;
    min-height: 14px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: color-mix(in srgb, var(--surface) 72%, transparent);
    overflow: hidden;
    cursor: move;
    touch-action: none;
  }

  .monitor-tile:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .monitor-tile.selected {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    z-index: 3;
  }

  .monitor-tile.primary {
    z-index: 2;
  }

  .monitor-tile.primary::after {
    content: "★";
    position: absolute;
    top: 1px;
    left: 4px;
    color: var(--accent-2);
    font-size: 0.8rem;
    line-height: 1;
    pointer-events: none;
  }

  .monitor-tile.disabled {
    opacity: 0.45;
  }

  .monitor-tile.unsized .monitor-face {
    border-style: dashed;
    border-color: var(--warning);
  }

  .monitor-face {
    position: absolute;
    inset: 0;
    margin: auto;
    transform-origin: center center;
    border: 1px solid color-mix(in srgb, var(--text) 30%, transparent);
    border-radius: 4px;
    background:
      linear-gradient(
        135deg,
        color-mix(in srgb, var(--accent) 22%, transparent),
        color-mix(in srgb, var(--surface-2) 60%, transparent)
      );
    pointer-events: none;
  }

  .monitor-tile.primary .monitor-face {
    border-color: color-mix(in srgb, var(--accent-2) 70%, transparent);
  }

  .face-marker {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 22%;
    height: 22%;
    max-width: 18px;
    max-height: 18px;
    border-radius: 2px;
    background: color-mix(in srgb, var(--accent-2) 70%, transparent);
  }

  .monitor-label {
    position: absolute;
    inset: 0;
    display: grid;
    place-content: center;
    gap: 0.1rem;
    padding: 0.2rem 0.35rem;
    text-align: center;
    pointer-events: none;
    overflow: hidden;
  }

  .monitor-label strong {
    font-size: clamp(0.7rem, 1.4vw, 0.95rem);
    line-height: 1.1;
    text-shadow: 0 1px 3px rgb(0 0 0 / 55%);
  }

  .monitor-label .small {
    font-size: 0.68rem;
    line-height: 1.1;
    text-shadow: 0 1px 3px rgb(0 0 0 / 55%);
  }

  .canvas-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }

  .canvas-actions .ghost {
    font-size: 0.78rem;
    padding: 0.25rem 0.55rem;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    align-items: center;
  }

  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
  }

  .swatch {
    display: inline-block;
    width: 0.7rem;
    height: 0.7rem;
    border-radius: 2px;
    border: 1px solid var(--border);
  }

  .swatch.selected {
    border: 2px solid var(--accent);
  }

  .swatch.primary {
    border-color: var(--accent-2);
    background: color-mix(in srgb, var(--accent-2) 25%, transparent);
  }

  .swatch.unsized {
    border-style: dashed;
    border-color: var(--warning);
  }

  .details {
    display: grid;
    gap: 0.85rem;
    padding: 0.85rem;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: color-mix(in srgb, var(--surface) 55%, transparent);
    min-width: 0;
  }

  .details-head {
    display: flex;
    align-items: start;
    justify-content: space-between;
    gap: 0.6rem;
  }

  .details h4,
  .details h5 {
    margin: 0;
  }

  .details h4 {
    color: var(--text-strong);
    font-size: 1.05rem;
  }

  .details h5 {
    color: var(--accent);
    font-size: 0.75rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .details section {
    display: grid;
    gap: 0.4rem;
  }

  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }

  .badge {
    padding: 0.1rem 0.45rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: 0.7rem;
    font-weight: 650;
    white-space: nowrap;
  }

  .badge.primary {
    border-color: var(--accent-2);
    color: var(--accent-2);
  }

  .badge.warning {
    border-color: var(--warning);
    color: var(--warning);
  }

  .field-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.15rem 0.85rem;
  }

  .field-grid :global(.field) {
    padding: 0.3rem 0;
    border-bottom: 0;
    gap: 0.15rem;
  }

  .field-grid :global(.field-label) {
    color: var(--muted);
    font-size: 0.78rem;
    font-weight: 500;
  }

  .group {
    border-top: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
    padding-top: 0.5rem;
  }

  .group summary {
    cursor: pointer;
    color: var(--text-strong);
    font-weight: 600;
    font-size: 0.85rem;
  }

  .group[open] summary {
    margin-bottom: 0.4rem;
  }

  .missing summary {
    color: var(--muted);
    font-weight: 500;
  }

  .empty-state {
    display: grid;
    gap: 0.5rem;
    justify-items: start;
    padding: 1rem;
    border: 1px dashed var(--border);
    border-radius: 12px;
    background: color-mix(in srgb, var(--surface) 45%, transparent);
  }

  .empty-state h4 {
    margin: 0;
  }

  .picker {
    display: grid;
    gap: 0.6rem;
    padding: 0.85rem;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: color-mix(in srgb, var(--surface) 65%, transparent);
  }

  .picker-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .output-list {
    display: grid;
    gap: 0.35rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .output {
    display: grid;
    gap: 0.1rem;
    width: 100%;
    padding: 0.45rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: color-mix(in srgb, var(--surface-2) 40%, transparent);
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }

  .output:hover:not(:disabled) {
    border-color: var(--accent);
  }

  .output:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .manual {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr)) auto;
    gap: 0.5rem;
    align-items: end;
    padding-top: 0.5rem;
    border-top: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
  }

  .manual label {
    display: grid;
    gap: 0.2rem;
    color: var(--muted);
    font-size: 0.78rem;
  }

  .manual input {
    width: 100%;
    padding: 0.3rem 0.45rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
  }

  @media (max-width: 960px) {
    .workspace {
      grid-template-columns: minmax(0, 1fr);
    }

    .canvas {
      height: 40vh;
    }

    .manual {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
