<script lang="ts">
  import type { ControlValue } from "@nix-web-gui/ui";
  import { rpc, type ScanRecord } from "../../api";

  type MenuProvider = {
    id: string;
    label: string;
    provider?: string;
    fields?: string[];
  };

  type FieldSpec = {
    name: string;
    label: string;
  };

  export let parent: ScanRecord;
  export let allRecords: ScanRecord[];
  export let values: Record<string, ControlValue>;
  export let onchange: (path: string, value: ControlValue) => void;
  export let csrf: string;
  export let providers: MenuProvider[] = [];
  export let fieldHelp: (name: string) => string | null = () => null;
  export let onstructurechange: () => Promise<void> = async () => {};

  const APPS_PROVIDER = "apps";
  const NEW_SECTION = "__new__";

  const MAIN_FIELDS: FieldSpec[] = [
    { name: "provider", label: "Provider" },
    { name: "target", label: "Target" },
    { name: "action", label: "Action" },
    { name: "title", label: "Title" },
    { name: "icon", label: "Icon" },
    { name: "iconFont", label: "Icon font" },
  ];

  const CONDITION_FIELDS: FieldSpec[] = [
    { name: "checked", label: "Checked when" },
    { name: "when", label: "Visible when" },
    { name: "disabled", label: "Disabled when" },
  ];

  const FALLBACK_HELP: Record<string, string> = {
    provider:
      "Contents source: a provider like apps, custom (run a command), or Open Application (open a path).",
    target: "URL or path opened when the entry is activated.",
    action: "Shell command run when the entry is activated.",
    title: "Header text while this submenu is open; defaults to the label.",
    icon: "Material Symbols name, for example school, wifi or power_settings_new.",
    iconFont: "Font family for the icon glyph when it is not the menu font.",
    checked: "Command whose exit status decides the toggle state.",
    when: "Command; the entry is shown only while it succeeds.",
    disabled: "Shell command; the row is dimmed and unselectable while it succeeds.",
  };

  const ICON_CHOICES = [
    "apps",
    "school",
    "bolt",
    "settings",
    "power_settings_new",
    "keyboard",
    "menu_book",
    "home",
    "search",
    "screenshot_region",
    "edit",
    "document_scanner",
    "qr_code_scanner",
    "emoji_emotions",
    "content_paste",
    "edit_note",
    "toggle_on",
    "coffee",
    "nightlight",
    "battery_std",
    "lock",
    "bedtime",
    "logout",
    "restart_alt",
    "wifi",
    "tune",
    "science",
    "volume_up",
    "volume_down",
    "mic",
    "headphones",
    "music_note",
    "movie",
    "folder",
    "terminal",
    "code",
    "build",
    "download",
    "upload",
    "delete",
    "refresh",
    "star",
    "favorite",
    "info",
    "help",
    "warning",
    "check_circle",
    "cancel",
    "close",
    "add",
    "remove",
    "arrow_back",
    "arrow_forward",
    "chevron_right",
    "expand_more",
    "more_vert",
    "open_in_new",
    "link",
    "image",
    "palette",
    "dark_mode",
    "light_mode",
    "schedule",
    "calendar_month",
    "notifications",
    "do_not_disturb_on",
    "bluetooth",
    "vpn_key",
    "print",
    "display_settings",
  ];

  const PROVIDER_ICONS: Record<string, string> = {
    "Open Application": "open_in_new",
    "Run Command": "terminal",
    "Open Webpage": "link",
    "Apps List": "apps",
  };

  let selectedPath = "";
  let busy = false;
  let error = "";
  let notice = "";

  let addOpen = false;
  let addSection = "";
  let addNewSection = "";
  let addChild = "";
  let addProviderLabel = "Run Command";
  let addLabel = "";
  let addTarget = "";
  let addAction = "";
  let addIcon = "";

  $: rows = parent.children ?? [];
  $: sections = groupRows(rows);
  $: addSectionIds = sections.map((section) => section.id);
  $: addProviderFields = providerFields(addProviderLabel);

  function titleCaseId(id: string): string {
    const expanded = id.replaceAll("_", " ");
    let out = "";
    for (let index = 0; index < expanded.length; index += 1) {
      const character = expanded[index];
      if (
        character === character.toUpperCase() &&
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

  function decodeLiteral(raw: string): string {
    const text = raw.trim();
    if (text.length >= 2 && text.startsWith('"') && text.endsWith('"')) {
      return text
        .slice(1, -1)
        .replaceAll('\\"', '"')
        .replaceAll("\\\\", "\\");
    }
    return text;
  }

  function relativeKey(row: ScanRecord): string {
    return row.path.slice(parent.path.length + 1);
  }

  function sectionOf(row: ScanRecord): string {
    return relativeKey(row).split(".")[0];
  }

  function childKeyOf(row: ScanRecord): string {
    return relativeKey(row).split(".").slice(1).join(".");
  }

  function isTopLevel(row: ScanRecord): boolean {
    return !relativeKey(row).includes(".");
  }

  function sectionRows(id: string): ScanRecord[] {
    return rows.filter((row) => sectionOf(row) === id);
  }

  function groupRows(input: ScanRecord[]): Array<{
    id: string;
    label: string;
    rows: ScanRecord[];
  }> {
    const order: string[] = [];
    const grouped = new Map<string, ScanRecord[]>();
    for (const row of input) {
      const id = sectionOf(row);
      if (!grouped.has(id)) {
        grouped.set(id, []);
        order.push(id);
      }
      grouped.get(id)!.push(row);
    }
    return order.map((id) => ({
      id,
      label: titleCaseId(id),
      rows: grouped.get(id) ?? [],
    }));
  }

  function fieldRecord(row: ScanRecord, name: string): ScanRecord | undefined {
    const path = `${row.path}.${name}`;
    return (
      (row.children ?? []).find((child) => child.path === path) ??
      allRecords.find((record) => record.path === path)
    );
  }

  function fieldText(row: ScanRecord, name: string): string {
    const record = fieldRecord(row, name);
    if (!record) return "";
    const value = values[record.path];
    if (Array.isArray(value)) return value.map(String).join(", ");
    if (typeof value === "string") return value;
    if (typeof value === "number" || typeof value === "boolean") {
      return String(value);
    }
    return decodeLiteral(record.value);
  }

  function providerValue(provider: MenuProvider): string {
    return provider.provider ?? provider.id;
  }

  function providerForLabel(label: string): MenuProvider | undefined {
    return providers.find(
      (provider) =>
        provider.label === label || providerValue(provider) === label,
    );
  }

  function providerValueForLabel(label: string): string {
    return providerForLabel(label)?.provider ?? label;
  }

  function providerFields(label: string): string[] {
    return providerForLabel(label)?.fields ?? [];
  }

  function providerLabelFor(row: ScanRecord): string {
    const raw = fieldText(row, "provider").trim();
    if (raw) {
      const known = providers.find(
        (provider) => providerValue(provider) === raw,
      );
      return known?.label ?? raw;
    }
    if (fieldText(row, "target").trim()) {
      return (
        providers.find((provider) =>
          (provider.fields ?? []).includes("target"),
        )?.label ?? "Open Application"
      );
    }
    if (fieldText(row, "action").trim()) {
      return (
        providers.find((provider) =>
          (provider.fields ?? []).includes("action"),
        )?.label ?? "Run Command"
      );
    }
    return "Run Command";
  }

  function providerRawValue(row: ScanRecord): string {
    return fieldText(row, "provider").trim();
  }

  function providerOptions(
    topLevel: boolean,
    current: string,
  ): MenuProvider[] {
    const available = providers.filter(
      (provider) => topLevel || providerValue(provider) !== APPS_PROVIDER,
    );
    const known = available.some(
      (provider) =>
        provider.label === current || providerValue(provider) === current,
    );
    if (current && !known && (topLevel || current !== APPS_PROVIDER)) {
      return [...available, { id: current, label: current }];
    }
    return available;
  }

  function providerSelection(row: ScanRecord): string {
    const options = providerOptions(isTopLevel(row), providerRawValue(row));
    const label = providerLabelFor(row);
    return options.some((provider) => provider.label === label)
      ? label
      : (options[0]?.label ?? label);
  }

  function visibleFields(row: ScanRecord): FieldSpec[] {
    const fields = providerFields(providerLabelFor(row));
    return MAIN_FIELDS.filter(
      (field) =>
        (field.name !== "target" && field.name !== "action") ||
        fields.includes(field.name),
    );
  }

  function helpFor(name: string): string | null {
    return fieldHelp(name) ?? FALLBACK_HELP[name] ?? null;
  }

  function rowLabel(row: ScanRecord): string {
    return fieldText(row, "label").trim() || relativeKey(row);
  }

  function rowKind(row: ScanRecord): string {
    if (fieldText(row, "action").trim()) return "shell";
    if (fieldText(row, "target").trim()) return "link";
    if (providerRawValue(row)) return "provider";
    return "group";
  }

  function rowSummary(row: ScanRecord): string {
    const action = fieldText(row, "action").trim();
    if (action) return action;
    const target = fieldText(row, "target").trim();
    if (target) return target;
    if (providerRawValue(row)) return providerValueForLabel(providerLabelFor(row));
    return "submenu";
  }

  function rowChecked(row: ScanRecord): boolean {
    return fieldText(row, "checked").trim().length > 0;
  }

  function rowDisabled(row: ScanRecord): boolean {
    return fieldText(row, "disabled").trim().length > 0;
  }

  function rowIcon(row: ScanRecord): string {
    return fieldText(row, "icon").trim();
  }

  function providerWarning(row: ScanRecord): string {
    if (providerRawValue(row) !== APPS_PROVIDER) return "";
    if (!isTopLevel(row)) {
      return "Apps List is only valid on a top-level menu. Choose another provider.";
    }
    if (sectionRows(sectionOf(row)).length > 1) {
      return "Remove this menu's child options before choosing Apps List.";
    }
    return "";
  }

  function sectionAcceptsChildren(id: string): boolean {
    const topLevel = rows.find((row) => relativeKey(row) === id);
    return !topLevel || providerRawValue(topLevel) !== APPS_PROVIDER;
  }

  function literalOf(value: string): string {
    return JSON.stringify(value);
  }

  function listLiteralOf(items: string[]): string {
    return `[ ${items.map(literalOf).join(" ")} ]`;
  }

  function aliasesText(row: ScanRecord): string {
    const record = fieldRecord(row, "aliases");
    if (!record) return "";
    const value = values[record.path];
    if (Array.isArray(value)) return value.map(String).join(", ");
    if (typeof value === "string") return value;
    const parsed = record.list ?? [];
    if (parsed.length) return parsed.join(", ");
    return "";
  }

  async function setField(
    row: ScanRecord,
    name: string,
    value: string,
  ): Promise<void> {
    busy = true;
    error = "";
    notice = "";
    try {
      const record = fieldRecord(row, name);
      if (record) {
        onchange(record.path, value);
        return;
      }
      await rpc(
        "structure.addField",
        { parent: row.path, field: name, literal: literalOf(value) },
        csrf,
      );
      await onstructurechange();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot edit the field";
    } finally {
      busy = false;
    }
  }

  async function changeProvider(row: ScanRecord, label: string): Promise<void> {
    const value = providerValueForLabel(label);
    if (value === APPS_PROVIDER) {
      if (!isTopLevel(row)) {
        notice =
          "Apps List is only valid on a top-level menu. Choose another provider.";
        return;
      }
      if (sectionRows(sectionOf(row)).length > 1) {
        notice = "Remove this menu's child options before choosing Apps List.";
        return;
      }
    }
    if (!value && !fieldRecord(row, "provider")) return;
    await setField(row, "provider", value);
  }

  async function changeAliases(row: ScanRecord, text: string): Promise<void> {
    const items = text
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean);
    busy = true;
    error = "";
    notice = "";
    try {
      const record = fieldRecord(row, "aliases");
      if (record) {
        onchange(record.path, items);
        return;
      }
      if (items.length === 0) return;
      await rpc(
        "structure.addField",
        { parent: row.path, field: "aliases", literal: listLiteralOf(items) },
        csrf,
      );
      await onstructurechange();
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Cannot edit the aliases";
    } finally {
      busy = false;
    }
  }

  async function renameRow(
    row: ScanRecord,
    nextKey?: string,
  ): Promise<void> {
    if (isTopLevel(row)) {
      notice = "Section rows have no key field; add a new section instead.";
      return;
    }
    const current = childKeyOf(row);
    const next = nextKey ?? prompt("New row name", current);
    if (!next || next === current) return;
    const key = `${sectionOf(row)}.${next.trim()}`;
    busy = true;
    error = "";
    notice = "";
    try {
      await rpc(
        "structure.rename",
        { parent: parent.path, child: row.path, key },
        csrf,
      );
      await onstructurechange();
      selectedPath = "";
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot rename the row";
    } finally {
      busy = false;
    }
  }

  async function moveRow(row: ScanRecord, offset: number): Promise<void> {
    busy = true;
    error = "";
    notice = "";
    try {
      await rpc(
        "structure.move",
        { parent: parent.path, child: row.path, offset },
        csrf,
      );
      await onstructurechange();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot move the row";
    } finally {
      busy = false;
    }
  }

  async function removeRow(row: ScanRecord): Promise<void> {
    if (!confirm(`Remove ${rowLabel(row)}?`)) return;
    busy = true;
    error = "";
    notice = "";
    try {
      await rpc(
        "structure.remove",
        { parent: parent.path, child: row.path },
        csrf,
      );
      if (selectedPath === row.path) selectedPath = "";
      await onstructurechange();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot remove the row";
    } finally {
      busy = false;
    }
  }

  function openAdd(): void {
    addOpen = true;
    addSection = addSectionIds[0] ?? NEW_SECTION;
    addNewSection = "";
    addChild = "";
    addProviderLabel = "Run Command";
    addLabel = "";
    addTarget = "";
    addAction = "";
    addIcon = "terminal";
    error = "";
    notice = "";
  }

  function seedProvider(label: string): void {
    addProviderLabel = label;
    addLabel = label;
    addTarget = "";
    addAction = "";
    addIcon = PROVIDER_ICONS[label] ?? "";
  }

  function updateAddChild(value: string): void {
    addChild = value;
    if (value.trim() && addProviderLabel === "Apps List") {
      seedProvider("Run Command");
    }
  }

  function addTargetSection(): string {
    return addSection === NEW_SECTION ? addNewSection.trim() : addSection;
  }

  async function addRow(): Promise<void> {
    const section = addTargetSection();
    const child = addChild.trim();
    if (!section) {
      error = "A section id is required.";
      return;
    }
    if (section.includes(".") || section.includes('"')) {
      error = "A section id cannot contain a dot or a quote.";
      return;
    }
    if (child && !sectionAcceptsChildren(section)) {
      error = "Apps List rows cannot have child entries.";
      return;
    }
    if (!child && rows.some((row) => relativeKey(row) === section)) {
      error =
        "This section already exists. Set a child id or pick New section.";
      return;
    }
    const value = providerValueForLabel(addProviderLabel);
    if (child && value === APPS_PROVIDER) {
      error = "Apps List is only valid on a top-level menu.";
      return;
    }
    const fields = providerFields(addProviderLabel);
    const key = child ? `${section}.${child}` : section;
    const defaults: Record<string, string> = {
      label: literalOf(addLabel.trim() || titleCaseId(child || section)),
    };
    if (value) defaults.provider = literalOf(value);
    if (fields.includes("target") && addTarget.trim()) {
      defaults.target = literalOf(addTarget.trim());
    }
    if (fields.includes("action") && addAction.trim()) {
      defaults.action = literalOf(addAction.trim());
    }
    defaults.icon = literalOf(addIcon.trim());
    busy = true;
    error = "";
    notice = "";
    try {
      await rpc(
        "structure.add",
        { path: parent.path, key, mode: null, defaults },
        csrf,
      );
      addOpen = false;
      await onstructurechange();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot add the row";
    } finally {
      busy = false;
    }
  }
</script>

<div class="menu-editor">
  {#if error}
    <p class="banner error">{error}</p>
  {/if}
  {#if notice}
    <p class="banner notice">{notice}</p>
  {/if}

  <div class="toolbar">
    <span class="count">{rows.length} entries</span>
    <span class="spacer"></span>
    <button type="button" class="ghost" onclick={openAdd} disabled={busy}>
      Add entry
    </button>
  </div>

  {#each sections as section (section.id)}
    <section class="menu-section">
      <h3 class="section-heading">
        <span>{section.label}</span>
        <span class="section-id">{section.id}</span>
      </h3>
      {#each section.rows as row (row.path)}
        <article
          class="menu-row"
          class:selected={selectedPath === row.path}
          data-path={row.path}
        >
          <header class="row-main">
            <span class="row-icon" title={rowIcon(row) || "No icon"}>
              {#if rowIcon(row)}
                {rowIcon(row)}
              {:else}
                {rowLabel(row).charAt(0).toUpperCase()}
              {/if}
            </span>

            {#if !isTopLevel(row)}
              <input
                class="key-input"
                type="text"
                aria-label="Entry key"
                value={childKeyOf(row)}
                onchange={(event) => {
                  const next = event.currentTarget.value.trim();
                  if (next && next !== childKeyOf(row)) {
                    renameRow(row, next);
                  } else {
                    event.currentTarget.value = childKeyOf(row);
                  }
                }}
              />
            {/if}

            <input
              class="label-input"
              type="text"
              aria-label="Entry label"
              value={fieldText(row, "label")}
              onchange={(event) =>
                setField(row, "label", event.currentTarget.value)}
            />

            <input
              class="aliases-input"
              type="text"
              aria-label="Entry aliases"
              placeholder="aliases"
              value={aliasesText(row)}
              onchange={(event) => changeAliases(row, event.currentTarget.value)}
            />

            <span class="kind">{rowKind(row)}</span>
            {#if rowChecked(row)}<span class="state">checked</span>{/if}
            {#if rowDisabled(row)}<span class="state">disabled</span>{/if}
            <span class="summary" title={rowSummary(row)}>{rowSummary(row)}</span>

            <button
              type="button"
              class="ghost tiny"
              aria-expanded={selectedPath === row.path}
              onclick={() =>
                (selectedPath = selectedPath === row.path ? "" : row.path)}
            >
              {selectedPath === row.path ? "Close" : "Edit"}
            </button>
            <button
              type="button"
              class="ghost tiny"
              onclick={() => removeRow(row)}
              disabled={busy}
            >
              Remove
            </button>
          </header>

          {#if selectedPath === row.path}
            <div class="row-details">
              {#if providerWarning(row)}
                <p class="banner error">{providerWarning(row)}</p>
              {/if}

              <div class="field-grid">
                {#each visibleFields(row) as field (field.name)}
                  <div class="field">
                    <label class="field-label">
                      <span>{field.label}</span>
                      {#if field.name === "provider"}
                        <select
                          value={providerSelection(row)}
                          onchange={(event) =>
                            changeProvider(row, event.currentTarget.value)}
                          disabled={busy}
                        >
                          {#each providerOptions(
                            isTopLevel(row),
                            providerRawValue(row),
                          ) as provider (provider.label)}
                            <option value={provider.label}>{provider.label}</option>
                          {/each}
                        </select>
                      {:else if field.name === "icon"}
                        <span class="field-row">
                          <input
                            type="text"
                            value={fieldText(row, "icon")}
                            onchange={(event) =>
                              setField(row, "icon", event.currentTarget.value)}
                          />
                          <select
                            aria-label="Common icons"
                            value=""
                            onchange={(event) => {
                              if (event.currentTarget.value) {
                                setField(
                                  row,
                                  "icon",
                                  event.currentTarget.value,
                                );
                              }
                              event.currentTarget.value = "";
                            }}
                          >
                            <option value="">Common icons…</option>
                            {#each ICON_CHOICES as icon (icon)}
                              <option value={icon}>{icon}</option>
                            {/each}
                          </select>
                        </span>
                      {:else}
                        <input
                          type="text"
                          class={field.name === "action" ? "mono" : ""}
                          value={fieldText(row, field.name)}
                          onchange={(event) =>
                            setField(
                              row,
                              field.name,
                              event.currentTarget.value,
                            )}
                          disabled={busy}
                        />
                      {/if}
                    </label>
                    {#if helpFor(field.name)}
                      <p class="field-help">{helpFor(field.name)}</p>
                    {/if}
                  </div>
                {/each}
              </div>

              <details class="advanced">
                <summary>Advanced conditions</summary>
                <div class="field-grid">
                  {#each CONDITION_FIELDS as field (field.name)}
                    <div class="field">
                      <label class="field-label">
                        <span>{field.label}</span>
                        <input
                          type="text"
                          class="mono"
                          value={fieldText(row, field.name)}
                          onchange={(event) =>
                            setField(
                              row,
                              field.name,
                              event.currentTarget.value,
                            )}
                          disabled={busy}
                        />
                      </label>
                      {#if helpFor(field.name)}
                        <p class="field-help">{helpFor(field.name)}</p>
                      {/if}
                    </div>
                  {/each}
                </div>
              </details>

              <div class="row-actions">
                <span class="spacer"></span>
                <button
                  type="button"
                  class="ghost tiny"
                  onclick={() => moveRow(row, -1)}
                  disabled={busy}
                >
                  Up
                </button>
                <button
                  type="button"
                  class="ghost tiny"
                  onclick={() => moveRow(row, 1)}
                  disabled={busy}
                >
                  Down
                </button>
                <button
                  type="button"
                  class="ghost tiny"
                  onclick={() => renameRow(row)}
                  disabled={busy || isTopLevel(row)}
                  title={isTopLevel(row)
                    ? "Section rows have no key field"
                    : "Rename this entry"}
                >
                  Rename
                </button>
              </div>
            </div>
          {/if}
        </article>
      {/each}
    </section>
  {/each}

  {#if addOpen}
    <section class="add-panel">
      <h3>Add menu entry</h3>
      <p class="field-help">
        Pick a provider to seed the label, provider, target or action, and icon.
      </p>
      <div class="provider-assistant">
        {#each providerOptions(!addChild.trim(), "") as provider (provider.label)}
          <button
            type="button"
            class="ghost tiny"
            class:active={addProviderLabel === provider.label}
            onclick={() => seedProvider(provider.label)}
          >
            {provider.label}
          </button>
        {/each}
      </div>

      <div class="field-grid">
        <div class="field">
          <label class="field-label">
            <span>Section</span>
            <select
              value={addSection}
              onchange={(event) => (addSection = event.currentTarget.value)}
            >
              {#each sections as section (section.id)}
                <option value={section.id}>{section.label}</option>
              {/each}
              <option value={NEW_SECTION}>New section…</option>
            </select>
          </label>
        </div>
        {#if addSection === NEW_SECTION}
          <div class="field">
            <label class="field-label">
              <span>Section id</span>
              <input
                type="text"
                placeholder="section-id"
                value={addNewSection}
                oninput={(event) =>
                  (addNewSection = event.currentTarget.value)}
              />
            </label>
          </div>
        {/if}
        <div class="field">
          <label class="field-label">
            <span>Child id</span>
            <input
              type="text"
              placeholder="optional child-id"
              value={addChild}
              oninput={(event) => updateAddChild(event.currentTarget.value)}
              disabled={addSection !== NEW_SECTION &&
                !sectionAcceptsChildren(addSection)}
            />
          </label>
          {#if addSection !== NEW_SECTION && !sectionAcceptsChildren(addSection)}
            <p class="field-help">
              Apps List rows cannot have child entries.
            </p>
          {/if}
        </div>
        <div class="field">
          <label class="field-label">
            <span>Label</span>
            <input
              type="text"
              value={addLabel}
              oninput={(event) => (addLabel = event.currentTarget.value)}
            />
          </label>
        </div>
        {#if addProviderFields.includes("target")}
          <div class="field">
            <label class="field-label">
              <span>Target</span>
              <input
                type="text"
                value={addTarget}
                oninput={(event) => (addTarget = event.currentTarget.value)}
              />
            </label>
            {#if helpFor("target")}
              <p class="field-help">{helpFor("target")}</p>
            {/if}
          </div>
        {/if}
        {#if addProviderFields.includes("action")}
          <div class="field">
            <label class="field-label">
              <span>Action</span>
              <input
                type="text"
                class="mono"
                value={addAction}
                oninput={(event) => (addAction = event.currentTarget.value)}
              />
            </label>
            {#if helpFor("action")}
              <p class="field-help">{helpFor("action")}</p>
            {/if}
          </div>
        {/if}
        <div class="field">
          <label class="field-label">
            <span>Icon</span>
            <span class="field-row">
              <input
                type="text"
                value={addIcon}
                oninput={(event) => (addIcon = event.currentTarget.value)}
              />
              <select
                aria-label="Common icons"
                value=""
                onchange={(event) => {
                  if (event.currentTarget.value) {
                    addIcon = event.currentTarget.value;
                  }
                  event.currentTarget.value = "";
                }}
              >
                <option value="">Common icons…</option>
                {#each ICON_CHOICES as icon (icon)}
                  <option value={icon}>{icon}</option>
                {/each}
              </select>
            </span>
          </label>
          {#if helpFor("icon")}
            <p class="field-help">{helpFor("icon")}</p>
          {/if}
        </div>
      </div>

      <div class="add-actions">
        <span class="spacer"></span>
        <button
          type="button"
          class="ghost"
          onclick={() => (addOpen = false)}
          disabled={busy}
        >
          Cancel
        </button>
        <button type="button" onclick={addRow} disabled={busy}>
          {busy ? "Adding…" : "Add entry"}
        </button>
      </div>
    </section>
  {/if}
</div>

<style>
  .menu-editor {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .toolbar,
  .add-actions,
  .row-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .spacer {
    flex: 1;
  }

  .count,
  .section-id {
    color: var(--muted);
    font-size: 0.8rem;
  }

  .banner {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.5rem 0.7rem;
    font-size: 0.85rem;
  }

  .banner.error {
    border-color: var(--danger);
    color: var(--danger);
  }

  .banner.notice {
    border-color: var(--warning);
    color: var(--warning);
  }

  .menu-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .section-heading {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    font-size: 1rem;
  }

  .menu-row {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
  }

  .menu-row.selected {
    border-color: var(--accent);
  }

  .row-main {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem 0.6rem;
  }

  .row-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.8rem;
    height: 1.8rem;
    border-radius: 6px;
    background: var(--surface-2);
    color: var(--accent);
    font-size: 0.7rem;
    overflow: hidden;
    flex-shrink: 0;
  }

  .key-input {
    width: 8rem;
    flex-shrink: 0;
  }

  .label-input {
    flex: 1 1 10rem;
    min-width: 6rem;
  }

  .aliases-input {
    flex: 1 1 8rem;
    min-width: 5rem;
  }

  .kind,
  .state {
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.1rem 0.45rem;
    color: var(--muted);
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    flex-shrink: 0;
  }

  .state {
    color: var(--success);
    border-color: var(--success);
  }

  .summary {
    max-width: 14rem;
    color: var(--muted);
    font-family: "JetBrains Mono", "Cascadia Code", monospace;
    font-size: 0.75rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-details {
    border-top: 1px solid var(--border);
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .field-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
    gap: 0.75rem;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
  }

  .field-label {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    font-size: 0.8rem;
    color: var(--muted);
  }

  .field-row {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }

  .field-row input {
    flex: 1;
  }

  .field-row select {
    width: 9rem;
    flex-shrink: 0;
  }

  .field-help {
    color: var(--muted);
    font-size: 0.75rem;
  }

  .mono {
    font-family: "JetBrains Mono", "Cascadia Code", monospace;
  }

  .advanced summary {
    cursor: pointer;
    color: var(--muted);
    font-size: 0.85rem;
    margin-bottom: 0.5rem;
  }

  .tiny {
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
  }

  .add-panel {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .add-panel h3 {
    font-size: 1rem;
  }

  .provider-assistant {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }

  .provider-assistant button.active {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
