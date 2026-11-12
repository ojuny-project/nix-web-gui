<script lang="ts">
  export let title: string;
  export let description: string | null | undefined = undefined;
</script>

<section class="panel">
  <header class="panel-head">
    <h2>{title}</h2>
    {#if description}
      <p class="muted">{description}</p>
    {/if}
  </header>
  <div class="panel-body">
    <slot />
  </div>
</section>
