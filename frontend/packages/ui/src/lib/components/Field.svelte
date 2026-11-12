<script lang="ts">
  export let label: string;
  export let description: string | null | undefined = undefined;
  export let htmlFor: string | undefined = undefined;
</script>

<div class="field">
  <label class="field-label" for={htmlFor}>{label}</label>
  {#if description}
    <p class="field-help">{description}</p>
  {/if}
  <div class="field-control">
    <slot />
  </div>
</div>
