<script lang="ts">
  import Field from "./Field.svelte";
  import { OJUNY_WIDGETS, OJUNY_WIDGET_KINDS } from "../features";
  import Color from "../controls/Color.svelte";
  import ListInput from "../controls/List.svelte";
  import NumberInput from "../controls/Number.svelte";
  import Radio from "../controls/Radio.svelte";
  import Select from "../controls/Select.svelte";
  import Switch from "../controls/Switch.svelte";
  import Text from "../controls/Text.svelte";
  import TableInput from "../controls/Table.svelte";
  import Textarea from "../controls/Textarea.svelte";
  import ThemePicker from "../controls/ThemePicker.svelte";
  import type {
    ChangeHandler,
    ControlDescriptor,
    ControlValue,
    TableRow,
  } from "../types";

  export let descriptor: ControlDescriptor;
  export let value: ControlValue;
  export let onchange: ChangeHandler;
  export let expandDetails = false;
  export let compact = false;
  export let onIconPick:
    | ((current: string) => Promise<string | null>)
    | undefined = undefined;
  export let onIconPreview:
    | ((value: string) => Promise<string | null>)
    | undefined = undefined;
  export let onIconDelete:
    | ((value: string) => Promise<void>)
    | undefined = undefined;
  export let csvList = false;

  $: id = `control-${descriptor.path.replaceAll(".", "-")}`;
  $: disabled = descriptor.readonly === true;
  $: tableRows = ((value as { rows?: TableRow[] })?.rows ?? []) as TableRow[];

  const rawWidgets = new Set([
    "raw",
    "submodule",
    "horizontal-submodule",
    "table-attrset",
    "table-list",
    "keybinds",
    "monitors",
    "users",
    "secureboot",
    "secure-boot",
    "menu",
    "plugins",
    "bar",
    "dock",
    "grid-configuration",
    "docker",
  ]);

  function change(next: unknown) {
    if (!disabled) onchange(descriptor.path, next);
  }
</script>

<div style={`margin-left: ${(descriptor.depth ?? 0) * 12}px`}>
<Field
  label={descriptor.label}
  description={descriptor.description}
  origin={descriptor.origin}
  {expandDetails}
  {compact}
  htmlFor={id}
>
  {#if descriptor.widget === "switch"}
    <Switch {id} checked={Boolean(value)} {disabled} onchange={change} />
  {:else if descriptor.widget === "radio"}
    <Radio {id} choices={descriptor.choices ?? []} value={String(value ?? "")} {disabled} onchange={change} />
  {:else if descriptor.widget === "theme"}
    <ThemePicker {id} value={String(value ?? "")} {disabled} onchange={change} />
  {:else if descriptor.widget === "dropdown" || descriptor.widget === "facter" || descriptor.widget === "font" || descriptor.widget === "icon-theme" || descriptor.widget === "cursor-theme"}
    <Select {id} choices={descriptor.choices ?? []} value={String(value ?? "")} {disabled} onchange={change} />
  {:else if descriptor.widget === "spin" || descriptor.widget === "slider"}
    <NumberInput
      {id}
      value={Number(value ?? 0)}
      min={descriptor.min ?? undefined}
      max={descriptor.max ?? undefined}
      step={descriptor.step ?? undefined}
      digits={descriptor.digits ?? undefined}
      unit={descriptor.unit ?? undefined}
      range={descriptor.widget === "slider"}
      {disabled}
      onchange={change}
    />
  {:else if descriptor.widget === "multiline" || descriptor.widget === "raw"}
    <Textarea
      {id}
      value={String(value ?? "")}
      placeholder={descriptor.placeholder ?? undefined}
      rows={descriptor.rows ?? 6}
      {disabled}
      onchange={change}
    />
  {:else if descriptor.widget === "list" || descriptor.widget === "packages" || descriptor.widget === "nixos-modules" || descriptor.widget === "hm-modules"}
    {#if csvList}
      <input
        {id}
        type="text"
        class="csv-input"
        value={(Array.isArray(value) ? (value as string[]) : []).join(", ")}
        {disabled}
        placeholder="comma separated"
        oninput={(event) =>
          change(
            event.currentTarget.value
              .split(",")
              .map((item) => item.trim())
              .filter(Boolean),
          )}
      />
    {:else}
      <ListInput
        {id}
        items={Array.isArray(value) ? (value as string[]) : []}
        packages={descriptor.widget === "packages"}
        {disabled}
        onchange={change}
      />
    {/if}
  {:else if descriptor.widget === "color"}
    <Color {id} value={String(value ?? "#000000")} {disabled} onchange={change} />
  {:else if !OJUNY_WIDGETS && OJUNY_WIDGET_KINDS.has(descriptor.widget)}
    <p class="muted">
      This Ojuny widget is not available in the minimal build.
    </p>
  {:else if (descriptor.widget === "table-attrset" || descriptor.widget === "table-list") && (descriptor.table?.columns.length ?? 0) > 0}
    <TableInput
      {id}
      columns={descriptor.table?.columns ?? []}
      columnTypes={descriptor.table?.columnTypes ?? []}
      choices={descriptor.table?.choices ?? descriptor.choices ?? []}
      list={descriptor.widget === "table-list"}
      keyEditable={descriptor.table?.keyEditable ?? false}
      firstValueEditable={descriptor.table?.firstValueEditable ?? false}
      rows={tableRows}
      {disabled}
      onchange={change}
      {onIconPick}
      {onIconPreview}
      {onIconDelete}
    />
  {:else if rawWidgets.has(descriptor.widget)}
    <Textarea
      {id}
      value={String(value ?? "")}
      placeholder={descriptor.placeholder ?? "Raw Nix expression"}
      rows={descriptor.rows ?? 6}
      {disabled}
      onchange={change}
    />
  {:else}
    <Text
      {id}
      value={String(value ?? "")}
      placeholder={descriptor.placeholder ?? undefined}
      {disabled}
      onchange={change}
    />
  {/if}
</Field>
</div>
