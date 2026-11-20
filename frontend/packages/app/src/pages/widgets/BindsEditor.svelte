<script lang="ts">
  import { tick } from "svelte";
  import type { ControlValue } from "@nix-web-gui/ui";
  import { rpc, type ScanRecord } from "../../api";

  export let parent: ScanRecord;
  export let allRecords: ScanRecord[];
  export let values: Record<string, ControlValue>;
  export let onchange: (path: string, value: ControlValue) => void;
  export let csrf: string;
  export let onstructurechange: () => Promise<void>;

  type BindKind = "bool" | "int" | "float" | "text" | "list";
  type BindField = { name: string; kind: BindKind };
  type BindSchema = {
    fields: BindField[];
    preview: string[];
    sort: string[];
    keyField: string | null;
  };

  const DEFAULT_BIND_FIELDS: BindField[] = [
    { name: "modifier", kind: "list" },
    { name: "keys", kind: "list" },
    { name: "dispatcher", kind: "text" },
    { name: "args", kind: "text" },
    { name: "description", kind: "text" },
    { name: "group", kind: "text" },
    { name: "subcat", kind: "text" },
  ];
  const DEFAULT_BIND_PREVIEW = ["modifier", "keys"];
  const DEFAULT_BIND_SORT = ["group", "subcat"];
  const EXTRAS_LABEL = "Other fields";
  const EXTRAS_HELP = "Unrecognized fields are preserved verbatim.";
  const FIELD_HELP: Record<string, string> = {
    dispatcher: "mango action the key triggers, e.g. focusdir, spawn.",
    args: "Argument passed to the dispatcher.",
    group: "Coarse category used to sort the main list.",
    subcat: "Finer category used within the group.",
  };

  let editingPath: string | null = null;
  let draftName = "";
  let draft: Record<string, ControlValue> = {};
  let original: Record<string, ControlValue> = {};
  let draftExtras: Record<string, ControlValue> = {};
  let originalExtras: Record<string, ControlValue> = {};
  let error = "";
  let busy = false;

  $: isList = parent.value.trimStart().startsWith("[");
  $: schema = schemaOf(parent);
  $: rows = sortRows(parent.children ?? [], schema, values);
  $: if (editingPath && !rows.some((row) => row.path === editingPath)) {
    editingPath = null;
  }

  function schemaOf(record: ScanRecord): BindSchema {
    const names = record.bindFields?.length
      ? record.bindFields
      : DEFAULT_BIND_FIELDS.map((field) => field.name);
    const types = record.bindFieldTypes ?? [];
    const fields = names.map((name, index) => ({
      name,
      kind: parseKind(types[index]) ?? defaultKind(name),
    }));
    const keep = (name: string) => fields.some((field) => field.name === name);
    return {
      fields,
      preview: (record.bindPreview ?? DEFAULT_BIND_PREVIEW).filter(keep),
      sort: (record.bindSort ?? DEFAULT_BIND_SORT).filter(keep),
      keyField: record.bindKeyField ?? null,
    };
  }

  function parseKind(text: string | null | undefined): BindKind | null {
    switch ((text ?? "").trim().toLowerCase()) {
      case "bool":
      case "boolean":
        return "bool";
      case "int":
      case "integer":
        return "int";
      case "float":
      case "number":
        return "float";
      case "list":
      case "string-list":
      case "strings":
        return "list";
      case "str":
      case "string":
      case "text":
        return "text";
      default:
        return null;
    }
  }

  function defaultKind(name: string): BindKind {
    return DEFAULT_BIND_FIELDS.find((field) => field.name === name)?.kind ?? "text";
  }

  function kindOf(name: string): BindKind {
    return schema.fields.find((field) => field.name === name)?.kind ?? "text";
  }

  function sortRows(
    children: ScanRecord[],
    bindSchema: BindSchema,
    current: Record<string, ControlValue>,
  ): ScanRecord[] {
    const sortKey = (row: ScanRecord) => {
      const parts = bindSchema.sort.map((name) =>
        valueText(current[fieldRecord(row, name)?.path ?? ""]),
      );
      parts.push(keyOf(row, bindSchema));
      return parts.join("\u0000");
    };
    return [...children].sort((left, right) => {
      const a = sortKey(left);
      const b = sortKey(right);
      return a < b ? -1 : a > b ? 1 : 0;
    });
  }

  function fieldRecord(row: ScanRecord, name: string): ScanRecord | null {
    return allRecords.find((record) => record.path === `${row.path}.${name}`) ?? null;
  }

  function rowFields(row: ScanRecord): ScanRecord[] {
    const prefix = `${row.path}.`;
    return allRecords.filter(
      (record) =>
        record.path.startsWith(prefix) &&
        !record.path.slice(prefix.length).includes("."),
    );
  }

  function unknownFields(row: ScanRecord): ScanRecord[] {
    return rowFields(row).filter(
      (record) => !schema.fields.some((field) => field.name === fieldName(record)),
    );
  }

  function fieldName(record: ScanRecord): string {
    return record.path.split(".").at(-1) ?? record.path;
  }

  function valueText(value: ControlValue): string {
    if (Array.isArray(value)) return (value as string[]).join(", ");
    if (typeof value === "boolean") return value ? "true" : "false";
    if (value === null || value === undefined) return "";
    return String(value);
  }

  function textOf(record: ScanRecord | null): string {
    if (!record) return "";
    return valueText(values[record.path]);
  }

  function csvList(text: string): string[] {
    return text
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean);
  }

  function keyOf(row: ScanRecord, bindSchema: BindSchema = schema): string {
    if (isList && bindSchema.keyField) {
      return textOf(fieldRecord(row, bindSchema.keyField)).trim();
    }
    const prefix = `${parent.path}.`;
    return row.path.startsWith(prefix)
      ? row.path.slice(prefix.length)
      : (row.path.split(".").at(-1) ?? "");
  }

  function labelOverride(row: ScanRecord): string | null {
    const label = row.label?.trim();
    if (!label || label === keyOf(row)) return null;
    return label;
  }

  function titleCaseId(id: string): string {
    const expanded = id.replaceAll("_", " ");
    let out = "";
    for (let index = 0; index < expanded.length; index += 1) {
      const character = expanded[index];
      if (
        character !== character.toLowerCase() &&
        index > 0 &&
        !out.endsWith(" ")
      ) {
        out += " ";
      }
      out += character;
    }
    return out.charAt(0).toUpperCase() + out.slice(1);
  }

  function bindLabel(key: string, override: string | null): string {
    const base = override
      ? override
      : key
        ? titleCaseId(key)
        : "New bind";
    return base.endsWith(":") || base.endsWith(";") ? base : `${base}:`;
  }

  function descriptionText(row: ScanRecord): string {
    return textOf(fieldRecord(row, "description")).trim();
  }

  function previewText(row: ScanRecord): string {
    const parts: string[] = [];
    for (const name of schema.preview) {
      const text = textOf(fieldRecord(row, name));
      if (kindOf(name) === "list") {
        parts.push(...csvList(text).map((item) => item.toUpperCase()));
      } else if (text.trim()) {
        parts.push(text.trim());
      }
    }
    return parts.join("+");
  }

  function emptyDraft(kind: BindKind): ControlValue {
    if (kind === "list") return "";
    if (kind === "bool") return false;
    if (kind === "int" || kind === "float") return 0;
    return "";
  }

  function fieldDraft(record: ScanRecord | null, kind: BindKind): ControlValue {
    if (!record) return emptyDraft(kind);
    const value = values[record.path];
    if (kind === "list") return csvList(valueText(value)).join(", ");
    if (kind === "bool") return Boolean(value);
    if (kind === "int" || kind === "float") {
      const number = Number(value);
      return Number.isFinite(number) ? number : 0;
    }
    return valueText(value);
  }

  function startEdit(row: ScanRecord) {
    error = "";
    editingPath = row.path;
    draftName = isList ? "" : keyOf(row);
    const nextDraft: Record<string, ControlValue> = {};
    const nextOriginal: Record<string, ControlValue> = {};
    for (const field of schema.fields) {
      const record = fieldRecord(row, field.name);
      const value = fieldDraft(record, field.kind);
      nextDraft[field.name] = value;
      nextOriginal[field.name] = value;
    }
    const nextExtras: Record<string, ControlValue> = {};
    const nextOriginalExtras: Record<string, ControlValue> = {};
    for (const record of unknownFields(row)) {
      const value = values[record.path] ?? "";
      nextExtras[fieldName(record)] = value;
      nextOriginalExtras[fieldName(record)] = Array.isArray(value)
        ? [...(value as string[])]
        : value;
    }
    draft = nextDraft;
    original = nextOriginal;
    draftExtras = nextExtras;
    originalExtras = nextOriginalExtras;
  }

  function stopEdit() {
    editingPath = null;
    error = "";
  }

  function sameList(left: ControlValue, right: ControlValue): boolean {
    return (
      csvList(valueText(left)).join("\u0000") ===
      csvList(valueText(right)).join("\u0000")
    );
  }

  function changed(name: string): boolean {
    const kind = kindOf(name);
    const next = draft[name];
    const previous = original[name];
    if (kind === "list") return !sameList(next, previous);
    if (kind === "int" || kind === "float") {
      return Number(next) !== Number(previous);
    }
    if (kind === "bool") return Boolean(next) !== Boolean(previous);
    return valueText(next) !== valueText(previous);
  }

  function extrasChanged(record: ScanRecord): boolean {
    const name = fieldName(record);
    const next = draftExtras[name];
    const previous = originalExtras[name];
    if (Array.isArray(next) || Array.isArray(previous)) {
      return !sameList(next, previous);
    }
    return next !== previous;
  }

  function typedValue(kind: BindKind, value: ControlValue): ControlValue {
    if (kind === "list") return csvList(valueText(value));
    if (kind === "bool") return Boolean(value);
    if (kind === "int") return Math.trunc(Number(value) || 0);
    if (kind === "float") {
      const number = Number(value);
      return Number.isFinite(number) ? number : 0;
    }
    return valueText(value);
  }

  function quoteNix(text: string): string {
    const escaped = text
      .replaceAll("\\", "\\\\")
      .replaceAll('"', '\\"')
      .replaceAll("${", "\\${");
    return `"${escaped}"`;
  }

  function literalOf(kind: BindKind, value: ControlValue): string {
    if (kind === "list") {
      return `[ ${csvList(valueText(value)).map(quoteNix).join(" ")} ]`;
    }
    if (kind === "bool") return value ? "true" : "false";
    if (kind === "int") return String(Math.trunc(Number(value) || 0));
    if (kind === "float") {
      const number = Number(value);
      const finite = Number.isFinite(number) ? number : 0;
      return String(finite).includes(".") ? String(finite) : `${finite}.0`;
    }
    return quoteNix(valueText(value));
  }

  function uniqueKey(list: ScanRecord[]): string {
    const existing = list.map((row) => keyOf(row));
    let index = 1;
    while (true) {
      const candidate = index === 1 ? "bind" : `bind${index}`;
      if (!existing.includes(candidate)) return candidate;
      index += 1;
    }
  }

  async function addRow() {
    busy = true;
    error = "";
    try {
      await rpc(
        "structure.add",
        {
          path: parent.path,
          key: isList ? "" : uniqueKey(rows),
          mode: null,
          defaults: {},
        },
        csrf,
      );
      await onstructurechange();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot add the bind";
    } finally {
      busy = false;
    }
  }

  async function removeRow(row: ScanRecord) {
    if (!confirm(`Remove ${bindLabel(keyOf(row), labelOverride(row))}?`)) return;
    busy = true;
    error = "";
    try {
      await rpc(
        "structure.remove",
        { parent: parent.path, child: row.path },
        csrf,
      );
      await onstructurechange();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot remove the bind";
    } finally {
      busy = false;
    }
  }

  async function applyRow(row: ScanRecord) {
    busy = true;
    error = "";
    try {
      const name = draftName.trim();
      if (!isList && !name) throw new Error("A name is required");
      const renamed = !isList && name !== keyOf(row);
      if (renamed) {
        await rpc(
          "structure.rename",
          { parent: parent.path, child: row.path, key: name },
          csrf,
        );
        await onstructurechange();
        await tick();
      }
      const targetRow = renamed
        ? (rows.find((candidate) => keyOf(candidate) === name) ?? row)
        : row;
      let added = false;
      for (const field of schema.fields) {
        if (!changed(field.name)) continue;
        if (fieldRecord(targetRow, field.name)) continue;
        await rpc(
          "structure.addField",
          {
            parent: targetRow.path,
            field: field.name,
            literal: literalOf(field.kind, draft[field.name]),
          },
          csrf,
        );
        added = true;
      }
      if (added) {
        await onstructurechange();
        await tick();
      }
      for (const field of schema.fields) {
        const record = fieldRecord(targetRow, field.name);
        if (!record || !changed(field.name)) continue;
        onchange(record.path, typedValue(field.kind, draft[field.name]));
      }
      for (const record of unknownFields(targetRow)) {
        if (!extrasChanged(record)) continue;
        onchange(record.path, draftExtras[fieldName(record)]);
      }
      editingPath = null;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot apply the bind";
    } finally {
      busy = false;
    }
  }
</script>

<div class="binds-editor">
  {#if error}
    <p class="error">{error}</p>
  {/if}
  {#each rows as row (row.path)}
    <section class="bind-row" data-path={row.path}>
      <div class="bind-main">
        <div class="bind-title">
          <strong>{bindLabel(keyOf(row), labelOverride(row))}</strong>
          {#if descriptionText(row)}
            <span class="muted small">{descriptionText(row)}</span>
          {/if}
        </div>
        <span class="bind-preview">{previewText(row)}</span>
        <button
          class="ghost tiny"
          type="button"
          disabled={busy}
          onclick={() => startEdit(row)}
        >
          Edit
        </button>
        <button
          class="ghost tiny"
          type="button"
          disabled={busy}
          onclick={() => removeRow(row)}
        >
          Remove
        </button>
      </div>
      {#if editingPath === row.path}
        <div class="bind-form">
          {#if descriptionText(row)}
            <p class="muted small bind-description">{descriptionText(row)}</p>
          {/if}
          {#if !isList}
            <label class="bind-field">
              <span>name</span>
              <input type="text" bind:value={draftName} disabled={busy} />
            </label>
          {/if}
          {#each schema.fields as field (field.name)}
            <label class="bind-field">
              <span>{field.name}</span>
              {#if field.kind === "list"}
                <input
                  type="text"
                  placeholder="comma separated"
                  value={valueText(draft[field.name])}
                  disabled={busy}
                  oninput={(event) =>
                    (draft = {
                      ...draft,
                      [field.name]: event.currentTarget.value,
                    })}
                />
              {:else if field.kind === "bool"}
                <input
                  type="checkbox"
                  checked={Boolean(draft[field.name])}
                  disabled={busy}
                  onchange={(event) =>
                    (draft = {
                      ...draft,
                      [field.name]: event.currentTarget.checked,
                    })}
                />
              {:else if field.kind === "int" || field.kind === "float"}
                <input
                  type="number"
                  step={field.kind === "float" ? "0.1" : "1"}
                  value={Number(draft[field.name] ?? 0)}
                  disabled={busy}
                  oninput={(event) =>
                    (draft = {
                      ...draft,
                      [field.name]: Number(event.currentTarget.value),
                    })}
                />
              {:else}
                <input
                  type="text"
                  value={valueText(draft[field.name])}
                  disabled={busy}
                  oninput={(event) =>
                    (draft = {
                      ...draft,
                      [field.name]: event.currentTarget.value,
                    })}
                />
              {/if}
              {#if FIELD_HELP[field.name]}
                <span class="muted small">{FIELD_HELP[field.name]}</span>
              {/if}
            </label>
          {/each}
          {#if unknownFields(row).length}
            <div class="bind-extras">
              <span>{EXTRAS_LABEL}</span>
              <p class="muted small">{EXTRAS_HELP}</p>
              {#each unknownFields(row) as extra (extra.path)}
                <label class="bind-field">
                  <span>{fieldName(extra)}</span>
                  {#if Array.isArray(draftExtras[fieldName(extra)])}
                    <input
                      type="text"
                      placeholder="comma separated"
                      value={valueText(draftExtras[fieldName(extra)])}
                      disabled={busy}
                      oninput={(event) =>
                        (draftExtras = {
                          ...draftExtras,
                          [fieldName(extra)]: csvList(
                            event.currentTarget.value,
                          ),
                        })}
                    />
                  {:else if typeof draftExtras[fieldName(extra)] === "boolean"}
                    <input
                      type="checkbox"
                      checked={Boolean(draftExtras[fieldName(extra)])}
                      disabled={busy}
                      onchange={(event) =>
                        (draftExtras = {
                          ...draftExtras,
                          [fieldName(extra)]: event.currentTarget.checked,
                        })}
                    />
                  {:else if typeof draftExtras[fieldName(extra)] === "number"}
                    <input
                      type="number"
                      value={Number(draftExtras[fieldName(extra)] ?? 0)}
                      disabled={busy}
                      oninput={(event) =>
                        (draftExtras = {
                          ...draftExtras,
                          [fieldName(extra)]: Number(event.currentTarget.value),
                        })}
                    />
                  {:else}
                    <input
                      type="text"
                      value={valueText(draftExtras[fieldName(extra)])}
                      disabled={busy}
                      oninput={(event) =>
                        (draftExtras = {
                          ...draftExtras,
                          [fieldName(extra)]: event.currentTarget.value,
                        })}
                    />
                  {/if}
                </label>
              {/each}
            </div>
          {/if}
          <div class="bind-actions">
            <button
              class="ghost"
              type="button"
              disabled={busy}
              onclick={stopEdit}
            >
              Cancel
            </button>
            <button
              class="ghost"
              type="button"
              disabled={busy}
              onclick={() => applyRow(row)}
            >
              Apply
            </button>
          </div>
        </div>
      {/if}
    </section>
  {/each}
  {#if rows.length === 0}
    <p class="muted small">No binds in this list.</p>
  {/if}
  <button class="ghost" type="button" disabled={busy} onclick={addRow}>
    Add bind
  </button>
</div>

<style>
  .binds-editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .bind-row {
    border-bottom: 1px solid var(--border);
    padding: 4px 0 8px;
  }

  .bind-main {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .bind-title {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .bind-title strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bind-preview {
    background: var(--surface-2);
    border-radius: 8px;
    padding: 2px 8px;
    font-size: 0.85rem;
    white-space: nowrap;
  }

  .bind-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 8px;
    padding: 8px 12px;
    border-left: 2px solid var(--accent);
    background: var(--surface);
  }

  .bind-description {
    margin: 0;
  }

  .bind-field {
    display: grid;
    grid-template-columns: 8rem minmax(0, 1fr);
    align-items: center;
    gap: 4px 8px;
  }

  .bind-field > span:first-child {
    font-size: 0.85rem;
    color: var(--text-strong);
  }

  .bind-field > .small {
    grid-column: 2;
  }

  .bind-extras {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px dashed var(--border);
  }

  .bind-extras > p {
    margin: 0;
  }

  .bind-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
