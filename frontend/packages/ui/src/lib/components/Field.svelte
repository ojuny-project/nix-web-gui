<script lang="ts">
  export let label: string;
  export let description: string | null | undefined = undefined;
  export let origin: string | null | undefined = undefined;
  export let htmlFor: string | undefined = undefined;
  export let expandDetails = false;

  let open = expandDetails;
  $: if (expandDetails) open = true;
  $: hasDetails = Boolean(description || origin);
</script>

<div class="field">
  <label class="field-label" for={htmlFor}>{label}</label>
  {#if hasDetails}
    <button
      type="button"
      class="details-toggle"
      aria-expanded={open}
      onclick={() => (open = !open)}
    >
      {open ? "Hide details" : "Details"}
    </button>
    {#if open}
      {#if description}
        <p class="field-help">{description}</p>
      {/if}
      {#if origin}
        <p class="field-origin">Imported from <code>{origin}</code></p>
      {/if}
    {/if}
  {/if}
  <div class="field-control">
    <slot />
  </div>
</div>
