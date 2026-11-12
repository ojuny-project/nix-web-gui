<script lang="ts">
  import Field from "./Field.svelte";
  import Color from "../controls/Color.svelte";
  import NumberInput from "../controls/Number.svelte";
  import Radio from "../controls/Radio.svelte";
  import Select from "../controls/Select.svelte";
  import Switch from "../controls/Switch.svelte";
  import Text from "../controls/Text.svelte";
  import Textarea from "../controls/Textarea.svelte";
  import type { ChangeHandler, ControlDescriptor, ControlValue } from "../types";

  export let descriptor: ControlDescriptor;
  export let value: ControlValue;
  export let onchange: ChangeHandler;

  $: id = `control-${descriptor.path.replaceAll(".", "-")}`;
  $: disabled = descriptor.readonly === true;

  function change(next: unknown) {
    if (!disabled) onchange(descriptor.path, next);
  }
</script>

<Field label={descriptor.label} description={descriptor.description} htmlFor={id}>
  {#if descriptor.widget === "switch"}
    <Switch {id} checked={Boolean(value)} {disabled} onchange={change} />
  {:else if descriptor.widget === "radio"}
    <Radio {id} choices={descriptor.choices ?? []} value={String(value ?? "")} {disabled} onchange={change} />
  {:else if descriptor.widget === "dropdown" || descriptor.widget === "theme" || descriptor.widget === "facter"}
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
  {:else if descriptor.widget === "color"}
    <Color {id} value={String(value ?? "#000000")} {disabled} onchange={change} />
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
