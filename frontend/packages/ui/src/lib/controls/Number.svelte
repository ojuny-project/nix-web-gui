<script lang="ts">
  export let id: string;
  export let value: number;
  export let min: number | undefined = undefined;
  export let max: number | undefined = undefined;
  export let step: number | undefined = undefined;
  export let digits: number | undefined = undefined;
  export let unit: string | undefined = undefined;
  export let range = false;
  export let disabled = false;
  export let onchange: (value: number) => void;

  function parse(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const next = Number(input.value);
    if (Number.isFinite(next)) onchange(next);
  }
</script>

<div class="number-field">
  <input
    {id}
    type={range ? "range" : "number"}
    {value}
    {min}
    {max}
    step={step ?? (digits === 0 ? 1 : "any")}
    {disabled}
    oninput={parse}
  />
  {#if unit}
    <span class="unit">{unit}</span>
  {/if}
</div>
