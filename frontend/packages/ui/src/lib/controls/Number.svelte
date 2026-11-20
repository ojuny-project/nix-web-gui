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

  $: numeric = Number.isFinite(value) ? value : (min ?? 0);
  $: display = digits != null ? numeric.toFixed(digits) : String(numeric);
  $: sliderStep = step ?? (range ? "any" : digits === 0 ? 1 : "any");

  function parse(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const next = Number(input.value);
    if (Number.isFinite(next)) onchange(next);
  }
</script>

<div class="number-field" class:range>
  <input
    {id}
    type={range ? "range" : "number"}
    value={numeric}
    {min}
    {max}
    step={sliderStep}
    {disabled}
    oninput={parse}
  />
  {#if range}
    <output for={id}>{display}{unit ? ` ${unit}` : ""}</output>
  {:else if unit}
    <span class="unit">{unit}</span>
  {/if}
</div>
