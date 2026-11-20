<script lang="ts">
  import { onMount } from "svelte";
  import { schemeById, schemeGroups } from "../theme";

  export let id: string;
  export let value: string;
  export let disabled = false;
  export let onchange: (value: string) => void;

  let filter = "";
  let previewId = value;
  let expanded = new Set<string>();

  $: groups = schemeGroups();
  $: current = schemeById(previewId || value);
  $: selected = schemeById(value);
  $: if (value) previewId = value;
  $: term = filter.trim().toLowerCase();

  onMount(() => {
    const group = groups.find((candidate) =>
      candidate.metas.some((meta) => meta.scheme.id === current.id),
    );
    if (group?.category) expanded = new Set([group.category]);
  });

  const paletteKeys = [
    "base00",
    "base01",
    "base02",
    "base03",
    "base04",
    "base05",
    "base06",
    "base07",
    "base08",
    "base09",
    "base0A",
    "base0B",
    "base0C",
    "base0D",
    "base0E",
    "base0F",
  ];

  function strip(scheme = current): string[] {
    return paletteKeys.map((key) => scheme.palette[key] ?? "#000000");
  }

  function matches(name: string): boolean {
    return !term || name.toLowerCase().includes(term);
  }

  function groupOpen(category: string | null, hits: boolean): boolean {
    if (category == null) return true;
    if (term) return hits;
    return expanded.has(category);
  }

  function toggle(category: string) {
    if (term) return;
    const next = new Set(expanded);
    if (next.has(category)) next.delete(category);
    else next.add(category);
    expanded = next;
  }

  function choose(schemeId: string) {
    previewId = schemeId;
    onchange(schemeId);
  }
</script>

<div class="theme-picker" data-theme-picker={id}>
  <div class="theme-list-pane">
    <input
      class="search-input small"
      type="search"
      bind:value={filter}
      placeholder="Search palettes"
      aria-label="Search palettes"
    />
    <div class="theme-list" role="listbox" aria-label="Color palettes">
      {#each groups as group, groupIndex (groupIndex)}
        {@const hits = group.metas.some((meta) => matches(meta.scheme.id))}
        {#if hits}
          {#if group.category}
            <button
              type="button"
              class="theme-group"
              aria-expanded={groupOpen(group.category, hits)}
              onclick={() => toggle(group.category ?? "")}
            >
              <span>{groupOpen(group.category, hits) ? "▾" : "▸"}</span>
              {group.label}
            </button>
          {/if}
          {#if groupOpen(group.category, hits)}
            {#each group.metas as meta (meta.scheme.id)}
              {#if matches(meta.scheme.id)}
                <button
                  type="button"
                  class="theme-row"
                  class:selected={meta.scheme.id === selected.id}
                  role="option"
                  aria-selected={meta.scheme.id === selected.id}
                  title={meta.scheme.id}
                  {disabled}
                  onclick={() => choose(meta.scheme.id)}
                >
                  <span class="theme-strip" aria-hidden="true">
                    {#each strip(meta.scheme) as color, index (index)}
                      <span style={`background: ${color}`}></span>
                    {/each}
                  </span>
                  <span class="theme-row-label">{meta.label}</span>
                </button>
              {/if}
            {/each}
          {/if}
        {/if}
      {/each}
    </div>
  </div>

  <div class="theme-preview">
    <div class="theme-preview-head">
      <strong>{current.name}</strong>
      <span class="muted small">{current.variant} · {current.author}</span>
    </div>
    <pre
      class="theme-preview-code"
      style={`background: ${current.palette.base00}; color: ${current.palette.base05};`}
    ><span style={`color: ${current.palette.base03};`}># base16 scheme preview</span>
<span style={`color: ${current.palette.base05};`}>{`{ config, pkgs, ... }:`}</span>
<span style={`color: ${current.palette.base05};`}>{`{`}</span>
  <span style={`color: ${current.palette.base0D};`}>stylix</span><span style={`color: ${current.palette.base05};`}>{` = {`}</span>
    <span style={`color: ${current.palette.base0D};`}>enable</span><span style={`color: ${current.palette.base05};`}>{` = `}</span><span style={`color: ${current.palette.base09};`}>true</span><span style={`color: ${current.palette.base05};`}>{`;`}</span>
    <span style={`color: ${current.palette.base0D};`}>base16Scheme</span><span style={`color: ${current.palette.base05};`}>{` = `}</span><span style={`color: ${current.palette.base0B};`}>{`"${current.id}"`}</span><span style={`color: ${current.palette.base05};`}>{`;`}</span>
  <span style={`color: ${current.palette.base05};`}>{`};`}</span>
<span style={`color: ${current.palette.base05};`}>{`}`}</span></pre>
    <div class="theme-preview-strip" aria-hidden="true">
      {#each strip(current) as color, index (index)}
        <span style={`background: ${color}`} title={color}></span>
      {/each}
    </div>
  </div>
</div>
