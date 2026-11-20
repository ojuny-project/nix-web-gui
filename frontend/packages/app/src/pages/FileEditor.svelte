<script lang="ts">
  import { onMount } from "svelte";
  import { rpc } from "../api";

  export let file: string;
  export let csrf: string;
  export let onclose: () => void;
  export let onsaved: () => Promise<void>;

  let text = "";
  let original = "";
  let loading = true;
  let busy = false;
  let error = "";
  let textarea: HTMLTextAreaElement | null = null;
  let highlight: HTMLPreElement | null = null;
  let gutter: HTMLPreElement | null = null;

  $: dirty = text !== original;
  $: highlighted = highlightNix(text);
  $: lines = text.split("\n").length;

  onMount(load);

  async function load() {
    loading = true;
    error = "";
    try {
      const result = await rpc<{ text: string }>("file.read", { path: file }, csrf);
      text = result.text;
      original = result.text;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot read the file";
    } finally {
      loading = false;
    }
  }

  function syncScroll() {
    if (!textarea || !highlight) return;
    highlight.scrollTop = textarea.scrollTop;
    highlight.scrollLeft = textarea.scrollLeft;
    if (gutter) gutter.scrollTop = textarea.scrollTop;
  }

  function keydown(event: KeyboardEvent) {
    if (event.key !== "Tab") return;
    event.preventDefault();
    const target = event.currentTarget as HTMLTextAreaElement;
    const start = target.selectionStart;
    const end = target.selectionEnd;
    text = `${text.slice(0, start)}  ${text.slice(end)}`;
    requestAnimationFrame(() => {
      target.selectionStart = target.selectionEnd = start + 2;
    });
  }

  async function save() {
    busy = true;
    error = "";
    try {
      await rpc("file.write", { path: file, text }, csrf);
      original = text;
      await onsaved();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot save the file";
    } finally {
      busy = false;
    }
  }

  function escapeHtml(value: string): string {
    return value
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;");
  }

  function token(kind: string, value: string): string {
    return `<span class="tok-${kind}">${escapeHtml(value)}</span>`;
  }

  function highlightNix(source: string): string {
    let out = "";
    let index = 0;
    const keywords = new Set([
      "assert",
      "else",
      "if",
      "in",
      "inherit",
      "let",
      "or",
      "rec",
      "then",
      "with",
    ]);
    while (index < source.length) {
      const rest = source.slice(index);
      const character = source[index];
      if (character === "#") {
        const end = source.indexOf("\n", index);
        const value = end === -1 ? rest : source.slice(index, end);
        out += token("comment", value);
        index += value.length;
        continue;
      }
      if (rest.startsWith("''")) {
        const end = source.indexOf("''", index + 2);
        const value = end === -1 ? rest : source.slice(index, end + 2);
        out += token("string", value);
        index += value.length;
        continue;
      }
      if (character === '"') {
        let end = index + 1;
        while (end < source.length) {
          if (source[end] === "\\") {
            end += 2;
            continue;
          }
          if (source[end] === '"') {
            end += 1;
            break;
          }
          end += 1;
        }
        out += token("string", source.slice(index, end));
        index = end;
        continue;
      }
      const identifier = /^[A-Za-z_][A-Za-z0-9_'-]*/.exec(rest);
      if (identifier) {
        const value = identifier[0];
        out += keywords.has(value)
          ? token("keyword", value)
          : token("ident", value);
        index += value.length;
        continue;
      }
      const number = /^-?[0-9]+(?:\.[0-9]+)?/.exec(rest);
      if (number) {
        out += token("number", number[0]);
        index += number[0].length;
        continue;
      }
      if (
        rest.startsWith("./") ||
        rest.startsWith("../") ||
        rest.startsWith("~/") ||
        (character === "/" && source[index + 1] !== "/" && source[index + 1] !== "*")
      ) {
        const path = /^[^\s;,)\]}]+/.exec(rest)?.[0] ?? character;
        out += token("path", path);
        index += path.length;
        continue;
      }
      if (/[{}()\[\];:.,=<>+*/!&|@?]/.test(character)) {
        out += token("punctuation", character);
        index += 1;
        continue;
      }
      out += escapeHtml(character);
      index += 1;
    }
    return out;
  }
</script>

<div class="nix-editor">
  <div class="nix-editor-head">
    <div>
      <strong>{file.split("/").at(-1)}</strong>
      <p class="muted small mono">{file}</p>
    </div>
    <span class="spacer"></span>
    {#if dirty}<span class="chip">unsaved</span>{/if}
    <button class="ghost" onclick={onclose} disabled={busy}>Close</button>
    <button onclick={save} disabled={busy || loading || !dirty}>
      {busy ? "Saving..." : "Save file"}
    </button>
  </div>
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
  {#if loading}
    <p class="muted">Loading {file.split("/").at(-1)}…</p>
  {:else}
    <div class="nix-editor-body">
      <pre class="nix-gutter" bind:this={gutter} aria-hidden="true">{Array.from(
        { length: lines },
        (_, index) => index + 1,
      ).join("\n")}</pre>
      <div class="nix-code">
        <pre class="nix-highlight" bind:this={highlight} aria-hidden="true">{@html highlighted}</pre>
        <textarea
          bind:this={textarea}
          bind:value={text}
          spellcheck="false"
          autocapitalize="off"
          autocomplete="off"
          aria-label={`Edit ${file}`}
          onscroll={syncScroll}
          onkeydown={keydown}
        ></textarea>
      </div>
    </div>
  {/if}
</div>

<style>
  .nix-editor {
    display: grid;
    gap: 0.6rem;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--surface);
    padding: 1rem;
  }

  .nix-editor-head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .nix-editor-head p {
    margin: 0.15rem 0 0;
  }

  .spacer {
    flex: 1;
  }

  .nix-editor-body {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    min-height: 60vh;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--base00);
    color: var(--base05);
    overflow: hidden;
  }

  .nix-gutter {
    margin: 0;
    padding: 0.9rem 0.6rem;
    color: var(--base03);
    background: color-mix(in srgb, var(--base00) 88%, black);
    font-family: "JetBrains Mono", "Cascadia Code", monospace;
    font-size: 0.82rem;
    line-height: 1.55;
    text-align: right;
    overflow: hidden;
    user-select: none;
  }

  .nix-code {
    position: relative;
    min-height: 0;
    overflow: hidden;
  }

  .nix-highlight,
  .nix-code textarea {
    margin: 0;
    padding: 0.9rem 1rem;
    border: 0;
    font-family: "JetBrains Mono", "Cascadia Code", monospace;
    font-size: 0.82rem;
    line-height: 1.55;
    tab-size: 2;
    white-space: pre;
    overflow: auto;
  }

  .nix-highlight {
    position: absolute;
    inset: 0;
    pointer-events: none;
    color: var(--base05);
  }

  .nix-code textarea {
    position: relative;
    width: 100%;
    height: 60vh;
    resize: none;
    background: transparent;
    color: transparent;
    caret-color: var(--base05);
    outline: none;
  }

  .nix-code textarea::selection {
    background: color-mix(in srgb, var(--base0D) 35%, transparent);
  }

  :global(.tok-comment) {
    color: var(--base03);
  }

  :global(.tok-string) {
    color: var(--base0B);
  }

  :global(.tok-keyword) {
    color: var(--base0E);
  }

  :global(.tok-number) {
    color: var(--base09);
  }

  :global(.tok-path) {
    color: var(--base0C);
  }

  :global(.tok-ident) {
    color: var(--base05);
  }

  :global(.tok-punctuation) {
    color: var(--base04);
  }
</style>
