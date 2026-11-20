<script lang="ts">
  import type { ControlValue } from "@nix-web-gui/ui";
  import {
    rpc,
    type ScanRecord,
    type WidgetPreset,
  } from "../../api";

  export let parent: ScanRecord;
  export let allRecords: ScanRecord[];
  export let values: Record<string, ControlValue>;
  export let onchange: (path: string, value: ControlValue) => void;
  export let csrf: string;
  export let widgetPresets: WidgetPreset[];
  export let onstructurechange: () => Promise<void>;

  type SectionKey = "left" | "center" | "right" | "dock";
  type SectionMode = "attrset" | "rows" | "strings";

  type SectionSpec = {
    key: SectionKey;
    name: string;
    heading: string;
  };

  type Tile = {
    id: string;
    key: string;
    path: string;
    record: ScanRecord | null;
    fields: ScanRecord[];
    preset: WidgetPreset | null;
    label: string;
    description: string;
    enabled: boolean;
    enabledPath: string | null;
    orderField: ScanRecord | null;
    order: number | null;
    index: number;
    mode: SectionMode;
  };

  type Section = SectionSpec & {
    record: ScanRecord | null;
    mode: SectionMode;
    tiles: Tile[];
    presets: WidgetPreset[];
  };

  const BAR_SECTIONS: SectionSpec[] = [
    { key: "left", name: "leftWidgets", heading: "Left widgets" },
    { key: "center", name: "centerWidgets", heading: "Center widgets" },
    { key: "right", name: "rightWidgets", heading: "Right widgets" },
  ];

  const DOCK_SECTIONS: SectionSpec[] = [
    { key: "dock", name: "widgets", heading: "Widgets" },
  ];

  const ROW_ID_FIELDS = ["id", "widgetId"];
  const WIDGET_INTERNALS = ["enabled", "_order"];

  let selectedPath = "";
  let addingSection: SectionKey | null = null;
  let addingPreset: WidgetPreset | null = null;
  let addLiterals: Record<string, string> = {};
  let addSearch = "";
  let customId = "";
  let renameDraft = "";
  let busy = false;
  let error = "";
  let dragSource: { section: SectionKey; index: number } | null = null;

  $: specs = parent.widget === "dock" ? DOCK_SECTIONS : BAR_SECTIONS;
  $: sections = buildSections(
    parent,
    allRecords,
    values,
    widgetPresets,
    specs,
  );
  $: extras = directChildren(parent, allRecords).filter(
    (record) => !specs.some((spec) => lastSegment(record.path) === spec.name),
  );
  $: settingsExtras = sortRecords(
    extras.filter(
      (record) =>
        record.hidden !== true &&
        overflowSection(lastSegment(record.path)) === null,
    ),
  );
  $: overflowBySection = buildOverflow(extras);
  $: selected = findTile(sections, selectedPath);
  $: selectedSection =
    sections.find((section) =>
      section.tiles.some((tile) => tile.path === selectedPath),
    ) ?? null;
  $: addTarget = sections.find((section) => section.key === addingSection) ?? null;
  $: availablePresets = addTarget ? filterPresets(addTarget, addSearch) : [];
  $: presetFieldNames = addingPreset
    ? orderedPresetFields(addingPreset, addLiterals)
    : [];

  function lastSegment(path: string): string {
    return path.split(".").at(-1) ?? path;
  }

  function directChildren(
    record: ScanRecord,
    records: ScanRecord[],
  ): ScanRecord[] {
    const prefix = `${record.path}.`;
    const direct: ScanRecord[] = [];
    const seen = new Set<string>();
    for (const candidate of [...(record.children ?? []), ...records]) {
      if (!candidate.path.startsWith(prefix)) continue;
      if (candidate.path.slice(prefix.length).includes(".")) continue;
      if (seen.has(candidate.path)) continue;
      seen.add(candidate.path);
      direct.push(candidate);
    }
    return direct;
  }

  function sectionRecord(
    parentRecord: ScanRecord,
    records: ScanRecord[],
    name: string,
  ): ScanRecord | null {
    return (
      records.find(
        (record) => record.path === `${parentRecord.path}.${name}`,
      ) ??
      (parentRecord.children ?? []).find(
        (record) => lastSegment(record.path) === name,
      ) ??
      null
    );
  }

  function sectionMode(
    record: ScanRecord | null,
    name: string,
  ): SectionMode {
    if (!record) return name === "centerWidgets" ? "strings" : "attrset";
    const text = record.value.trim();
    if (!text.startsWith("[")) return "attrset";
    if ((record.children?.length ?? 0) > 0) return "rows";
    return /\[\s*\{/.test(text) ? "rows" : "strings";
  }

  function buildSections(
    parentRecord: ScanRecord,
    records: ScanRecord[],
    current: Record<string, ControlValue>,
    presets: WidgetPreset[],
    sectionSpecs: SectionSpec[],
  ): Section[] {
    return sectionSpecs.map((spec) => {
      const record = sectionRecord(parentRecord, records, spec.name);
      const mode = sectionMode(record, spec.name);
      return {
        ...spec,
        record,
        mode,
        tiles: record
          ? buildTiles(record, mode, records, current, presets)
          : [],
        presets: presetsFor(spec.key, presets),
      };
    });
  }

  function presetsFor(
    key: SectionKey,
    presets: WidgetPreset[],
  ): WidgetPreset[] {
    return presets.filter((preset) => preset.sections.includes(key));
  }

  function listValue(
    record: ScanRecord,
    current: Record<string, ControlValue>,
  ): string[] {
    const value = current[record.path];
    if (Array.isArray(value)) return value.map((item) => String(item));
    return record.list ?? [];
  }

  function fieldValue(
    record: ScanRecord,
    current: Record<string, ControlValue>,
  ): ControlValue {
    if (record.path in current) return current[record.path];
    const raw = record.value.trim();
    if (record.widget === "switch") return raw === "true";
    if (record.widget === "spin" || record.widget === "slider") {
      const number = Number(raw);
      return Number.isFinite(number) ? number : 0;
    }
    if (
      record.widget === "list" ||
      record.widget === "packages" ||
      record.widget === "nixos-modules" ||
      record.widget === "hm-modules"
    ) {
      return record.list ?? [];
    }
    if (raw.startsWith('"') && raw.endsWith('"')) {
      return raw
        .slice(1, -1)
        .replaceAll('\\"', '"')
        .replaceAll("\\\\", "\\")
        .replaceAll("\\${", "${");
    }
    return raw;
  }

  function boolValue(
    record: ScanRecord,
    current: Record<string, ControlValue>,
  ): boolean {
    const value = fieldValue(record, current);
    if (typeof value === "boolean") return value;
    return String(value).trim() === "true";
  }

  function numberValue(
    record: ScanRecord,
    current: Record<string, ControlValue>,
  ): number | null {
    const value = Number(fieldValue(record, current));
    return Number.isFinite(value) ? value : null;
  }

  function prettyId(id: string): string {
    const spaced = id
      .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
      .replace(/[_-]+/g, " ")
      .trim();
    return spaced.replace(/\b\w/g, (letter) => letter.toUpperCase());
  }

  function presetLabel(preset: WidgetPreset | null, id: string): string {
    return preset?.label?.trim() || prettyId(id);
  }

  function presetDescription(
    preset: WidgetPreset | null,
    fields: ScanRecord[],
  ): string {
    if (preset?.description) return preset.description;
    if (fields.length === 0) return "No settings";
    return `${fields.length} ${fields.length === 1 ? "setting" : "settings"}`;
  }

  function buildTiles(
    record: ScanRecord,
    mode: SectionMode,
    records: ScanRecord[],
    current: Record<string, ControlValue>,
    presets: WidgetPreset[],
  ): Tile[] {
    if (mode === "strings") {
      return listValue(record, current).map((id, index) => ({
        id,
        key: id,
        path: `${record.path}.[${index}]`,
        record: null,
        fields: [],
        preset: presets.find((preset) => preset.id === id) ?? null,
        label: presetLabel(
          presets.find((preset) => preset.id === id) ?? null,
          id,
        ),
        description: "String entry",
        enabled: true,
        enabledPath: null,
        orderField: null,
        order: index + 1,
        index,
        mode,
      }));
    }
    if (mode === "rows") {
      return directChildren(record, records).map((child, index) => {
        const fields = directChildren(child, records);
        return tileFromFields(
          child.path,
          child.path,
          "",
          fields,
          child,
          mode,
          index,
          current,
          presets,
        );
      });
    }
    return attrsetTiles(record, records, current, presets);
  }

  /**
   * Attribute-set widget lists have no record for the widget itself: the scan
   * flattens unannotated nested sets, so only `section.id.field` leaves exist.
   * Group the leaves by their first segment under the section.
   */
  function attrsetTiles(
    record: ScanRecord,
    records: ScanRecord[],
    current: Record<string, ControlValue>,
    presets: WidgetPreset[],
  ): Tile[] {
    const prefix = `${record.path}.`;
    const grouped = new Map<string, ScanRecord[]>();
    for (const candidate of records) {
      if (!candidate.path.startsWith(prefix)) continue;
      const rest = candidate.path.slice(prefix.length);
      if (!rest) continue;
      const parts = rest.split(".");
      const id = parts[0];
      if (!grouped.has(id)) grouped.set(id, []);
      if (parts.length === 2) grouped.get(id)?.push(candidate);
    }
    const tiles = [...grouped.entries()].map(([id, fields], index) =>
      tileFromFields(
        id,
        `${record.path}.${id}`,
        id,
        fields,
        null,
        "attrset",
        index,
        current,
        presets,
      ),
    );
    if (!tiles.every((tile) => tile.order !== null)) return tiles;
    return [...tiles]
      .sort((left, right) => (left.order ?? 0) - (right.order ?? 0))
      .map((tile, index) => ({ ...tile, index }));
  }

  function tileFromFields(
    id: string,
    path: string,
    key: string,
    fields: ScanRecord[],
    record: ScanRecord | null,
    mode: SectionMode,
    index: number,
    current: Record<string, ControlValue>,
    presets: WidgetPreset[],
  ): Tile {
    const enabledField =
      fields.find((field) => lastSegment(field.path) === "enabled") ?? null;
    const orderField =
      fields.find((field) => lastSegment(field.path) === "_order") ?? null;
    const rowIdField =
      mode === "rows"
        ? (fields.find((field) =>
            ROW_ID_FIELDS.includes(lastSegment(field.path)),
          ) ?? null)
        : null;
    const widgetId =
      mode === "rows" && rowIdField
        ? String(fieldValue(rowIdField, current))
        : id;
    const preset = presets.find((entry) => entry.id === widgetId) ?? null;
    const visibleFields = fields
      .filter(
        (field) =>
          !WIDGET_INTERNALS.includes(lastSegment(field.path)) &&
          !(mode === "rows" && ROW_ID_FIELDS.includes(lastSegment(field.path))),
      )
      .map((field) => applyFieldSpec(field, preset));
    return {
      id: widgetId,
      key: mode === "rows" ? "" : key,
      path,
      record,
      fields: visibleFields,
      preset,
      label: presetLabel(preset, widgetId),
      description: presetDescription(preset, visibleFields),
      enabled: enabledField ? boolValue(enabledField, current) : true,
      enabledPath: enabledField?.path ?? null,
      orderField,
      order: orderField ? numberValue(orderField, current) : null,
      index,
      mode,
    };
  }

  /**
   * The scan has no annotations for fields inside widget lists, so the preset
   * field spec supplies the label, editor kind, bounds and choices.
   */
  function applyFieldSpec(
    record: ScanRecord,
    preset: WidgetPreset | null,
  ): ScanRecord {
    const spec =
      preset?.fields.find(
        (field) => field.name === lastSegment(record.path),
      ) ?? null;
    if (!spec) return record;
    return {
      ...record,
      label: record.label ?? spec.label ?? null,
      description: record.description ?? spec.description ?? null,
      choices: record.choices ?? spec.choices ?? null,
      min: record.min ?? spec.min ?? null,
      max: record.max ?? spec.max ?? null,
      step: record.step ?? spec.step ?? null,
      unit: record.unit ?? spec.unit ?? null,
    };
  }

  function findTile(sections: Section[], path: string): Tile | null {
    for (const section of sections) {
      const tile = section.tiles.find((entry) => entry.path === path);
      if (tile) return tile;
    }
    return null;
  }

  function overflowSection(name: string): SectionKey | null {
    const match = /^(left|center|right)Overflow/.exec(name);
    return match ? (match[1] as SectionKey) : null;
  }

  function sortRecords(records: ScanRecord[]): ScanRecord[] {
    return [...records].sort((left, right) => {
      const group = (left.groupOrder ?? 0) - (right.groupOrder ?? 0);
      if (group !== 0) return group;
      const order = (left.order ?? 0) - (right.order ?? 0);
      if (order !== 0) return order;
      return left.path.localeCompare(right.path);
    });
  }

  function buildOverflow(
    records: ScanRecord[],
  ): Record<string, ScanRecord[]> {
    const grouped: Record<string, ScanRecord[]> = {};
    for (const record of sortRecords(records)) {
      if (record.hidden === true) continue;
      const key = overflowSection(lastSegment(record.path));
      if (!key) continue;
      grouped[key] = [...(grouped[key] ?? []), record];
    }
    return grouped;
  }

  function filterPresets(section: Section, term: string): WidgetPreset[] {
    const query = term.trim().toLowerCase();
    return section.presets.filter((preset) => {
      if (section.tiles.some((tile) => tile.id === preset.id)) return false;
      if (!query) return true;
      return (
        preset.id.toLowerCase().includes(query) ||
        preset.label.toLowerCase().includes(query) ||
        (preset.description ?? "").toLowerCase().includes(query)
      );
    });
  }

  function presetFieldSpec(
    preset: WidgetPreset,
    name: string,
  ): WidgetPreset["fields"][number] | null {
    return preset.fields.find((field) => field.name === name) ?? null;
  }

  function orderedPresetFields(
    preset: WidgetPreset,
    literals: Record<string, string>,
  ): string[] {
    const position = new Map<string, number>(
      preset.fields.map((field, index) => [field.name, index] as const),
    );
    return Object.keys(literals)
      .filter((name) => name !== "enabled")
      .sort((left, right) => {
        const leftIndex = position.get(left) ?? Number.MAX_SAFE_INTEGER;
        const rightIndex = position.get(right) ?? Number.MAX_SAFE_INTEGER;
        if (leftIndex !== rightIndex) return leftIndex - rightIndex;
        return left.localeCompare(right);
      });
  }

  function isValidId(id: string): boolean {
    return /^[A-Za-z_][A-Za-z0-9_]*$/.test(id);
  }

  function sectionContains(section: Section, id: string): boolean {
    if (!section.record) return false;
    return new RegExp(`(^|[\\s{;])${id}\\s*=`).test(section.record.value);
  }

  function iconLetter(tile: Tile): string {
    return (tile.label.trim()[0] ?? "?").toUpperCase();
  }

  function selectTile(tile: Tile) {
    selectedPath = tile.path;
    renameDraft = tile.key || tile.id;
    addingSection = null;
    addingPreset = null;
    error = "";
  }

  function openAdd(section: Section) {
    addingSection = addingSection === section.key ? null : section.key;
    addingPreset = null;
    addLiterals = {};
    addSearch = "";
    customId = "";
    error = "";
  }

  function choosePreset(preset: WidgetPreset) {
    addingPreset = preset;
    const literals: Record<string, string> = {};
    for (const [name, literal] of Object.entries(preset.defaults)) {
      if (name === "enabled") continue;
      literals[name] = literal;
    }
    addLiterals = literals;
  }

  function cancelPreset() {
    addingPreset = null;
    addLiterals = {};
  }

  function nextOrder(section: Section): number {
    const orders = section.tiles
      .map((tile) => tile.order)
      .filter((order): order is number => order !== null);
    return orders.length === 0 ? 10 : Math.max(...orders) + 10;
  }

  function attrsetLiteral(
    section: Section,
    literals: Record<string, string>,
  ): string {
    const parts = ["enabled = true;", `_order = ${nextOrder(section)};`];
    for (const [name, raw] of Object.entries(literals)) {
      const literal = raw.trim();
      if (
        !isValidId(name) ||
        WIDGET_INTERNALS.includes(name) ||
        literal.length === 0
      ) {
        continue;
      }
      parts.push(`${name} = ${literal};`);
    }
    return `{ ${parts.join(" ")} }`;
  }

  function currentList(section: Section): string[] {
    if (!section.record) return [];
    return listValue(section.record, values);
  }

  async function addTile(
    section: Section,
    id: string,
    literals: Record<string, string>,
  ) {
    const widgetId = id.trim();
    if (!isValidId(widgetId)) {
      error = "Widget ids start with a letter or underscore and contain only letters, digits and underscores.";
      return;
    }
    if (
      section.tiles.some((tile) => tile.id === widgetId) ||
      sectionContains(section, widgetId)
    ) {
      error = `${widgetId} is already in ${section.heading}.`;
      return;
    }
    busy = true;
    error = "";
    const sectionPath = section.record?.path ?? `${parent.path}.${section.name}`;
    try {
      if (section.mode === "strings") {
        if (section.record) {
          onchange(sectionPath, [...currentList(section), widgetId]);
        } else {
          await rpc(
            "structure.addField",
            { parent: parent.path, field: section.name, literal: "[ ]" },
            csrf,
          );
          await rpc(
            "structure.add",
            { path: sectionPath, key: widgetId },
            csrf,
          );
          await onstructurechange();
        }
      } else {
        if (!section.record) {
          await rpc(
            "structure.addField",
            {
              parent: parent.path,
              field: section.name,
              literal: section.mode === "rows" ? "[ ]" : "{ }",
            },
            csrf,
          );
        }
        if (section.mode === "rows") {
          await rpc(
            "structure.add",
            {
              path: sectionPath,
              key: widgetId,
              defaults: {
                enabled: "true",
                _order: String(nextOrder(section)),
                widgetId: JSON.stringify(widgetId),
                ...literals,
              },
            },
            csrf,
          );
        } else {
          await rpc(
            "structure.addField",
            {
              parent: sectionPath,
              field: widgetId,
              literal: attrsetLiteral(section, literals),
            },
            csrf,
          );
        }
        await onstructurechange();
      }
      addingSection = null;
      addingPreset = null;
      addLiterals = {};
      addSearch = "";
      customId = "";
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Cannot add the widget";
    } finally {
      busy = false;
    }
  }

  async function removeTile(section: Section, tile: Tile) {
    if (!confirm(`Remove ${tile.label}?`)) return;
    busy = true;
    error = "";
    try {
      if (section.mode === "strings") {
        if (!section.record) return;
        onchange(
          section.record.path,
          currentList(section).filter((_, index) => index !== tile.index),
        );
      } else {
        if (!section.record) return;
        await rpc(
          "structure.remove",
          { parent: section.record.path, child: tile.path },
          csrf,
        );
        await onstructurechange();
      }
      if (selectedPath === tile.path) selectedPath = "";
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Cannot remove the widget";
    } finally {
      busy = false;
    }
  }

  async function renameTile(section: Section, tile: Tile) {
    const next = renameDraft.trim();
    if (!next || next === tile.id) return;
    if (!isValidId(next)) {
      error =
        "Widget ids start with a letter or underscore and contain only letters, digits and underscores.";
      return;
    }
    if (section.tiles.some((other) => other !== tile && other.id === next)) {
      error = `${next} is already in ${section.heading}.`;
      return;
    }
    busy = true;
    error = "";
    try {
      if (section.mode === "strings") {
        if (!section.record) return;
        onchange(
          section.record.path,
          currentList(section).map((value, index) =>
            index === tile.index ? next : value,
          ),
        );
      } else if (section.mode === "rows") {
        const idField = tile.record
          ? (directChildren(tile.record, allRecords).find((field) =>
              ROW_ID_FIELDS.includes(lastSegment(field.path)),
            ) ?? null)
          : null;
        if (!idField) {
          error = "This row has no id field to rename.";
          return;
        }
        onchange(idField.path, next);
      } else {
        if (!section.record) return;
        await rpc(
          "structure.rename",
          { parent: section.record.path, child: tile.path, key: next },
          csrf,
        );
        await onstructurechange();
        selectedPath = "";
      }
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Cannot rename the widget";
    } finally {
      busy = false;
    }
  }

  function toggleEnabled(tile: Tile, enabled: boolean) {
    if (!tile.enabledPath) {
      error = "This widget has no enabled field to toggle.";
      return;
    }
    onchange(tile.enabledPath, enabled);
  }

  function assignOrders(tiles: Tile[]): number[] {
    const pool = tiles
      .map((tile) => tile.order)
      .filter((order): order is number => order !== null)
      .sort((left, right) => left - right);
    let last = 0;
    return tiles.map((_, index) => {
      const order = pool[index] ?? Math.max((index + 1) * 10, last + 10);
      last = order;
      return order;
    });
  }

  function moveTile(section: Section, tile: Tile, targetIndex: number) {
    const count = section.tiles.length;
    if (count === 0) return;
    const target = Math.max(0, Math.min(count - 1, targetIndex));
    if (target === tile.index) return;
    error = "";
    if (section.mode === "rows") {
      void moveRow(section, tile, target);
      return;
    }
    if (section.mode === "strings") {
      if (!section.record) return;
      const reordered = [...section.tiles];
      reordered.splice(tile.index, 1);
      reordered.splice(target, 0, tile);
      onchange(
        section.record.path,
        reordered.map((entry) => entry.id),
      );
      selectedPath = `${section.record.path}.[${target}]`;
      return;
    }
    if (section.tiles.some((entry) => !entry.orderField)) {
      error = "Every widget needs an _order field before this section can be reordered here.";
      return;
    }
    const reordered = [...section.tiles];
    reordered.splice(tile.index, 1);
    reordered.splice(target, 0, tile);
    const orders = assignOrders(reordered);
    reordered.forEach((entry, position) => {
      if (entry.orderField && entry.order !== orders[position]) {
        onchange(entry.orderField.path, orders[position]);
      }
    });
    selectedPath = tile.path;
  }

  async function moveRow(section: Section, tile: Tile, targetIndex: number) {
    if (!section.record || !tile.record) return;
    const offset = targetIndex - tile.index;
    if (offset === 0) return;
    busy = true;
    error = "";
    try {
      await rpc(
        "structure.move",
        { parent: section.record.path, child: tile.record.path, offset },
        csrf,
      );
      await onstructurechange();
      selectedPath = "";
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Cannot move the widget";
    } finally {
      busy = false;
    }
  }

  function startDrag(
    event: DragEvent,
    section: Section,
    tile: Tile,
  ) {
    dragSource = { section: section.key, index: tile.index };
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData(
        "text/plain",
        `${section.key}:${tile.index}`,
      );
    }
  }

  function dropOn(event: DragEvent, section: Section, index: number) {
    event.preventDefault();
    const source = dragSource;
    dragSource = null;
    if (!source || source.section !== section.key) return;
    const tile = section.tiles[source.index];
    if (tile) moveTile(section, tile, index);
  }
</script>

<div class="bar-dock-editor">
  <p class="muted small">
    Drag to reorder, or use the arrow buttons. Select a widget to configure it.
  </p>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#snippet settingRow(record: ScanRecord)}
    {@const disabled = record.readonly === true}
    <div class="setting-row">
      <label class="setting-label" for={`widget-field-${record.path}`}>
        {record.label ?? prettyId(lastSegment(record.path))}
      </label>
      {#if record.widget === "switch"}
        <input
          id={`widget-field-${record.path}`}
          type="checkbox"
          checked={boolValue(record, values)}
          {disabled}
          onchange={(event) =>
            onchange(record.path, event.currentTarget.checked)}
        />
      {:else if record.widget === "dropdown" || record.widget === "radio" || record.widget === "font" || record.widget === "icon-theme" || record.widget === "cursor-theme" || (record.widget === "text" && (record.choices?.length ?? 0) > 0)}
        <select
          id={`widget-field-${record.path}`}
          value={String(fieldValue(record, values) ?? "")}
          {disabled}
          onchange={(event) => onchange(record.path, event.currentTarget.value)}
        >
          {#each record.choices ?? [] as choice (choice)}
            <option value={choice}>{choice}</option>
          {/each}
        </select>
      {:else if (record.widget === "spin" || record.widget === "slider") && (record.choices?.length ?? 0) > 0}
        <select
          id={`widget-field-${record.path}`}
          value={String(numberValue(record, values) ?? "")}
          {disabled}
          onchange={(event) =>
            onchange(record.path, Number(event.currentTarget.value))}
        >
          {#each record.choices ?? [] as choice (choice)}
            <option value={choice}>{choice}</option>
          {/each}
        </select>
      {:else if record.widget === "spin" || record.widget === "slider"}
        <input
          id={`widget-field-${record.path}`}
          type="number"
          value={numberValue(record, values) ?? 0}
          min={record.min ?? undefined}
          max={record.max ?? undefined}
          step={record.step ?? undefined}
          {disabled}
          onchange={(event) =>
            onchange(record.path, Number(event.currentTarget.value))}
        />
      {:else if record.widget === "list" || record.widget === "packages" || record.widget === "nixos-modules" || record.widget === "hm-modules"}
        <input
          id={`widget-field-${record.path}`}
          type="text"
          value={Array.isArray(fieldValue(record, values))
            ? (fieldValue(record, values) as string[]).join(", ")
            : ""}
          placeholder="comma separated"
          {disabled}
          oninput={(event) =>
            onchange(
              record.path,
              event.currentTarget.value
                .split(",")
                .map((item) => item.trim())
                .filter(Boolean),
            )}
        />
      {:else if record.widget === "multiline" || record.widget === "raw"}
        <textarea
          id={`widget-field-${record.path}`}
          rows={record.rows ?? 3}
          value={String(fieldValue(record, values) ?? "")}
          placeholder={record.placeholder ?? undefined}
          {disabled}
          oninput={(event) => onchange(record.path, event.currentTarget.value)}
        ></textarea>
      {:else}
        <input
          id={`widget-field-${record.path}`}
          type="text"
          value={String(fieldValue(record, values) ?? "")}
          placeholder={record.placeholder ?? undefined}
          {disabled}
          oninput={(event) => onchange(record.path, event.currentTarget.value)}
        />
      {/if}
      {#if record.description}
        <p class="muted small setting-help">{record.description}</p>
      {/if}
    </div>
  {/snippet}

  {#each sections as section (section.key)}
    <section class="tile-section" data-section={section.key}>
      <header class="section-head">
        <h4>{section.heading}</h4>
        <span class="count">{section.tiles.length}</span>
        <span class="spacer"></span>
        <button
          type="button"
          class="ghost tiny"
          disabled={busy}
          onclick={() => openAdd(section)}
        >
          {section.mode === "strings" ? "Add entry" : "Add widget"}
        </button>
      </header>

      {#if section.tiles.length === 0}
        <p class="muted small empty">No widgets in this section.</p>
      {:else}
        <div class="tile-list">
          {#each section.tiles as tile (tile.path)}
            <div
              class="tile"
              class:selected={selectedPath === tile.path}
              class:disabled={!tile.enabled}
              draggable={!busy}
              tabindex="0"
              role="button"
              aria-pressed={selectedPath === tile.path}
              data-path={tile.path}
              ondragstart={(event) => startDrag(event, section, tile)}
              ondragover={(event) => event.preventDefault()}
              ondrop={(event) => dropOn(event, section, tile.index)}
              onclick={() => selectTile(tile)}
              onkeydown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  selectTile(tile);
                }
              }}
            >
              <span class="grip" title="Drag to reorder">⠿</span>
              <span
                class="tile-icon"
                title={tile.preset?.icon ?? tile.id}
              >
                {iconLetter(tile)}
              </span>
              <span class="tile-text">
                <strong>{tile.label}</strong>
                <span class="tile-preview">{tile.description}</span>
              </span>
              {#if section.mode !== "strings"}
                <label
                  class="tile-switch"
                  title="Whether this widget is enabled."
                >
                  <input
                    type="checkbox"
                    checked={tile.enabled}
                    disabled={busy}
                    onclick={(event) => event.stopPropagation()}
                    onchange={(event) =>
                      toggleEnabled(tile, event.currentTarget.checked)}
                  />
                </label>
              {/if}
              <span class="tile-buttons">
                <button
                  type="button"
                  class="ghost tiny"
                  title="Move up"
                  disabled={busy || tile.index === 0}
                  onclick={(event) => {
                    event.stopPropagation();
                    moveTile(section, tile, tile.index - 1);
                  }}
                >
                  ↑
                </button>
                <button
                  type="button"
                  class="ghost tiny"
                  title="Move down"
                  disabled={busy || tile.index === section.tiles.length - 1}
                  onclick={(event) => {
                    event.stopPropagation();
                    moveTile(section, tile, tile.index + 1);
                  }}
                >
                  ↓
                </button>
                <button
                  type="button"
                  class="ghost tiny"
                  title="Remove widget"
                  disabled={busy}
                  onclick={(event) => {
                    event.stopPropagation();
                    void removeTile(section, tile);
                  }}
                >
                  ✕
                </button>
              </span>
            </div>
          {/each}
        </div>
      {/if}

      {#if addingSection === section.key}
        <div class="add-panel">
          {#if addingPreset}
            {@const preset = addingPreset as WidgetPreset}
            <div class="add-head">
              <strong>Add {preset.label || preset.id}</strong>
              <span class="spacer"></span>
              <button
                type="button"
                class="ghost tiny"
                disabled={busy}
                onclick={cancelPreset}
              >
                Back
              </button>
            </div>
            {#if preset.description}
              <p class="muted small">{preset.description}</p>
            {/if}
            {#if section.mode === "strings"}
              <p class="muted small">
                Center widgets are plain ids, so no settings are stored.
              </p>
            {:else if presetFieldNames.length === 0}
              <p class="muted small">
                This widget has no declared settings.
              </p>
            {:else}
              {#each presetFieldNames as name (name)}
                {@const spec = presetFieldSpec(preset, name)}
                <label class="preset-field">
                  <span>{spec?.label ?? name}</span>
                  <input
                    type="text"
                    bind:value={addLiterals[name]}
                    spellcheck="false"
                    disabled={busy}
                  />
                </label>
                {#if spec?.description}
                  <p class="muted small setting-help">{spec.description}</p>
                {/if}
              {/each}
            {/if}
            <div class="add-actions">
              <button
                type="button"
                class="ghost tiny"
                disabled={busy}
                onclick={cancelPreset}
              >
                Cancel
              </button>
              <button
                type="button"
                disabled={busy}
                onclick={() => void addTile(section, preset.id, addLiterals)}
              >
                Add
              </button>
            </div>
          {:else}
            <input
              type="search"
              class="preset-search"
              placeholder="Search widgets"
              bind:value={addSearch}
              disabled={busy}
            />
            <div class="preset-list">
              {#each availablePresets as preset (preset.id)}
                <button
                  type="button"
                  class="preset-row"
                  disabled={busy}
                  onclick={() => choosePreset(preset)}
                >
                  <span class="tile-icon small-icon">
                    {(preset.label.trim()[0] ?? "?").toUpperCase()}
                  </span>
                  <span class="preset-text">
                    <strong>{preset.label || prettyId(preset.id)}</strong>
                    <span class="tile-preview">
                      {preset.description ?? preset.id}
                    </span>
                  </span>
                </button>
              {/each}
              {#if availablePresets.length === 0}
                <p class="muted small">No presets left to add.</p>
              {/if}
            </div>
            {#if section.mode !== "strings"}
              <div class="custom-add">
                <input
                  type="text"
                  placeholder="customWidgetId"
                  bind:value={customId}
                  spellcheck="false"
                  disabled={busy}
                />
                <button
                  type="button"
                  class="ghost tiny"
                  disabled={busy || !customId.trim()}
                  onclick={() => void addTile(section, customId, {})}
                >
                  Add custom
                </button>
              </div>
            {/if}
          {/if}
        </div>
      {/if}

      {#if (overflowBySection[section.key] ?? []).length > 0}
        <div class="overflow-block">
          <p class="overflow-label">Overflow</p>
          {#each overflowBySection[section.key] ?? [] as record (record.path)}
            {@render settingRow(record)}
          {/each}
        </div>
      {/if}

      {#if selected !== null && selectedSection !== null && selectedSection.key === section.key}
        {@const tile = selected as Tile}
        {@const owner = selectedSection as Section}
        <div class="tile-details" data-path={tile.path}>
          <header class="details-head">
            <span
              class="tile-icon"
              title={tile.preset?.icon ?? tile.id}
            >
              {iconLetter(tile)}
            </span>
            <div class="details-title">
              <strong>{tile.label}</strong>
              <span class="muted small">{tile.id}</span>
            </div>
            <span class="spacer"></span>
            <button
              type="button"
              class="ghost tiny"
              onclick={() => (selectedPath = "")}
            >
              Close
            </button>
          </header>

          {#if tile.preset === null && tile.mode !== "strings"}
            <p class="muted small">
              This widget is not in the preset catalog.
            </p>
          {/if}

          <div class="details-grid">
            {#if tile.mode !== "strings"}
              <div class="setting-row">
                <span class="setting-label">Enabled</span>
                <input
                  type="checkbox"
                  checked={tile.enabled}
                  disabled={busy || tile.enabledPath === null}
                  onchange={(event) =>
                    toggleEnabled(tile, event.currentTarget.checked)}
                />
              </div>
            {/if}
            <div class="setting-row">
              <span class="setting-label">Id</span>
              <div class="rename-row">
                <input
                  type="text"
                  bind:value={renameDraft}
                  spellcheck="false"
                  disabled={busy}
                />
                <button
                  type="button"
                  class="ghost tiny"
                  disabled={busy}
                  onclick={() => void renameTile(owner, tile)}
                >
                  Rename
                </button>
              </div>
            </div>
          </div>

          {#if tile.fields.length === 0}
            <p class="muted small">No settings on this widget.</p>
          {:else}
            <div class="details-fields">
              {#each tile.fields as field (field.path)}
                {@render settingRow(field)}
              {/each}
            </div>
          {/if}

          <div class="details-actions">
            <button
              type="button"
              class="ghost tiny"
              disabled={busy}
              onclick={() => moveTile(owner, tile, 0)}
            >
              Move to top
            </button>
            <button
              type="button"
              class="ghost tiny"
              disabled={busy}
              onclick={() => moveTile(owner, tile, owner.tiles.length - 1)}
            >
              Move to bottom
            </button>
            <span class="spacer"></span>
            <button
              type="button"
              class="ghost tiny danger"
              disabled={busy}
              onclick={() => void removeTile(owner, tile)}
            >
              Remove widget
            </button>
          </div>
        </div>
      {/if}
    </section>
  {/each}

  {#if settingsExtras.length > 0}
    <section class="settings-section">
      <h4>Settings</h4>
      {#each settingsExtras as record (record.path)}
        {@render settingRow(record)}
      {/each}
    </section>
  {/if}
</div>

<style>
  .bar-dock-editor {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .tile-section,
  .settings-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
  }

  .section-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .section-head h4,
  .settings-section h4 {
    margin: 0;
    color: var(--text-strong);
    font-size: 0.95rem;
    font-weight: 650;
  }

  .section-head .count {
    padding: 0 8px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--muted);
    font-size: 0.75rem;
  }

  .spacer {
    flex: 1;
  }

  .empty {
    margin: 0;
  }

  .tile-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .tile {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: 1px solid transparent;
    border-radius: 10px;
    background: var(--surface-2);
    cursor: pointer;
  }

  .tile:hover {
    border-color: var(--border);
  }

  .tile.selected {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent);
  }

  .tile.disabled {
    opacity: 0.55;
  }

  .tile:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .grip {
    color: var(--muted);
    cursor: grab;
    font-size: 0.9rem;
    user-select: none;
  }

  .tile-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: var(--surface);
    color: var(--accent);
    font-weight: 700;
    flex: 0 0 auto;
  }

  .small-icon {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    font-size: 0.8rem;
  }

  .tile-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .tile-text strong,
  .preset-text strong {
    color: var(--text-strong);
    font-weight: 600;
  }

  .tile-preview {
    overflow: hidden;
    color: var(--muted);
    font-size: 0.78rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tile-switch {
    display: inline-flex;
    align-items: center;
  }

  .tile-buttons {
    display: inline-flex;
    gap: 2px;
  }

  .add-panel {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    border: 1px dashed var(--border);
    border-radius: 10px;
  }

  .add-head,
  .add-actions,
  .details-actions,
  .details-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .preset-search,
  .custom-add input {
    width: 100%;
  }

  .preset-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 260px;
    overflow-y: auto;
  }

  .preset-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--text);
    text-align: left;
  }

  .preset-row:hover {
    border-color: var(--border);
  }

  .preset-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .custom-add {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .preset-field {
    display: grid;
    grid-template-columns: minmax(140px, 220px) 1fr;
    align-items: center;
    gap: 8px;
  }

  .preset-field input {
    width: 100%;
    font-family: "JetBrains Mono", "Cascadia Code", monospace;
    font-size: 0.82rem;
  }

  .setting-row {
    display: grid;
    grid-template-columns: minmax(140px, 220px) 1fr;
    align-items: center;
    gap: 8px;
  }

  .setting-label {
    color: var(--text);
    font-size: 0.86rem;
  }

  .setting-help {
    grid-column: 2;
    margin: 0;
  }

  .setting-row > input[type="checkbox"] {
    justify-self: start;
  }

  .overflow-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: 4px;
  }

  .overflow-label {
    margin: 0;
    color: var(--muted);
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .tile-details {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border: 1px solid var(--accent);
    border-radius: 10px;
    background: var(--surface-2);
  }

  .details-title {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .details-grid,
  .details-fields {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .rename-row {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .rename-row input {
    flex: 1;
    min-width: 0;
  }

  .danger {
    color: var(--danger);
  }

  @media (max-width: 720px) {
    .setting-row,
    .preset-field {
      grid-template-columns: 1fr;
    }

    .setting-help {
      grid-column: 1;
    }
  }
</style>
