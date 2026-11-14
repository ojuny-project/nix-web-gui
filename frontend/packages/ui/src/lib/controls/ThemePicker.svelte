<script lang="ts">
  import { SCHEMES } from "../theme";

  export let id: string;
  export let value: string;
  export let disabled = false;
  export let onchange: (value: string) => void;

  let filter = "";
  $: term = filter.trim().toLowerCase();
  $: matches = term
    ? SCHEMES.filter(
        (scheme) =>
          scheme.id.toLowerCase().includes(term) ||
          scheme.name.toLowerCase().includes(term) ||
          scheme.author.toLowerCase().includes(term),
      )
    : SCHEMES;
  $: visible = matches.slice(0, 60);

  function swatches(scheme: (typeof SCHEMES)[number]): string[] {
    return ["base00", "base05", "base0d", "base0e", "base0b"].map(
      (key) => scheme.palette[key] ?? "#000000",
    );
  }
</script>

<div class="theme-picker" data-theme-picker={id}>
  <input
    class="search-input small"
    type="search"
    bind:value={filter}
    placeholder="Search themes"
    aria-label="Search themes"
  />
  <div class="theme-grid">
    {#each visible as scheme (scheme.id)}
      <button
        type="button"
        class="theme-swatch"
        class:selected={scheme.id === value}
        {disabled}
        title={`${scheme.name} by ${scheme.author}`}
        onclick={() => onchange(scheme.id)}
      >
        <span class="swatch-colors">
          {#each swatches(scheme) as color, index (index)}
            <span style={`background: ${color}`}></span>
          {/each}
        </span>
        <span class="swatch-name">{scheme.name}</span>
        <span class="muted small">{scheme.variant}</span>
      </button>
    {/each}
  </div>
  {#if matches.length > visible.length}
    <p class="muted small">
      Showing {visible.length} of {matches.length} themes. Refine the search.
    </p>
  {/if}
</div>
