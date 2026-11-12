<script lang="ts">
  import { onMount } from "svelte";
  import {
    Control,
    Panel,
    schemeById,
    type ControlDescriptor,
    type ControlValue,
  } from "@nix-web-gui/ui";
  import {
    getState,
    logout,
    rpc,
    type PreviewResult,
    type ScanMenu,
    type ScanRecord,
    type ScanResult,
    type User,
  } from "../api";

  export let user: User;
  export let onLogout: () => void;

  let root = "";
  let menus: ScanMenu[] = [];
  let selected = 0;
  let view: "settings" | "flake" = "settings";
  let records: ScanRecord[] = [];
  let values: Record<string, ControlValue> = {};
  let dirty: Set<string> = new Set();
  let advanced = false;
  let busy = false;
  let status = "";
  let error = "";
  let diff = "";
  let showDiff = false;
  let themeName = "everforest-dark-hard";
  let engine = "loading";

  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  function sortMenus(input: ScanMenu[]) {
    return [...input].sort(
      (a, b) => a.order - b.order || a.displayname.localeCompare(b.displayname),
    );
  }

  $: sortedMenus = sortMenus(menus);
  $: visibleRecords = records.filter((record) => !record.hidden || advanced);

  onMount(() => {
    const syncTheme = () => {
      themeName = document.documentElement.dataset.scheme ?? themeName;
    };
    syncTheme();
    window.addEventListener("nix-web-gui:theme-change", syncTheme);

    rpc<{ engine: string }>("system.capabilities", {}, user.csrf)
      .then((capabilities) => {
        engine = capabilities.engine;
      })
      .catch((cause) => {
        engine = cause instanceof Error ? cause.message : "unavailable";
      });

    getState()
      .then((state) => {
        const initial = state.flake ?? localStorage.getItem("nix-web-gui.root");
        if (initial) {
          root = initial;
          openRoot(initial);
        }
      })
      .catch((cause) => {
        error = cause instanceof Error ? cause.message : "Cannot read server state";
      });

    return () => window.removeEventListener("nix-web-gui:theme-change", syncTheme);
  });

  async function openRoot(path: string) {
    if (!path.trim()) return;
    busy = true;
    error = "";
    status = "Opening...";
    try {
      const opened = await rpc<{ menus: number; records: number }>(
        "session.open",
        { root: path },
        user.csrf,
      );
      const scanned = await rpc<ScanResult>("scan", {}, user.csrf);
      menus = scanned.menus;
      selected = 0;
      view = "settings";
      loadPage();
      localStorage.setItem("nix-web-gui.root", path);
      status = `Opened ${opened.menus} menus, ${opened.records} records`;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot open the flake";
    } finally {
      busy = false;
    }
  }

  function loadPage() {
    for (const timer of timers.values()) clearTimeout(timer);
    timers.clear();
    const menu = sortMenus(menus)[selected];
    records = menu?.records ?? [];
    values = Object.fromEntries(
      records.map((record) => [record.path, displayValue(record)]),
    );
    dirty = new Set();
    showDiff = false;
  }

  function selectMenu(index: number) {
    selected = index;
    view = "settings";
    loadPage();
  }

  function displayValue(record: ScanRecord): ControlValue {
    const raw = record.value.trim();
    if (record.widget === "switch") return raw === "true";
    if (record.widget === "spin" || record.widget === "slider") {
      const number = Number(raw);
      return Number.isFinite(number) ? number : 0;
    }
    if (raw.startsWith('"') && raw.endsWith('"')) {
      return raw
        .slice(1, -1)
        .replaceAll('\\"', '"')
        .replaceAll("\\\\", "\\")
        .replaceAll("\\${", "${");
    }
    return raw;
  }

  function toNix(record: ScanRecord, value: ControlValue): string {
    if (record.widget === "switch") return value ? "true" : "false";
    if (record.widget === "spin" || record.widget === "slider") {
      const number = Number(value);
      return Number.isFinite(number) ? String(number) : "0";
    }
    if (record.widget === "path") return String(value ?? "");
    return JSON.stringify(String(value ?? ""));
  }

  function descriptor(record: ScanRecord): ControlDescriptor {
    return {
      path: record.path,
      widget: (record.widget ?? "text") as ControlDescriptor["widget"],
      label: record.label ?? record.path.split(".").at(-1) ?? record.path,
      description: record.description,
      choices: record.choices,
      min: record.min,
      max: record.max,
      step: record.step,
      digits: record.digits,
      unit: record.unit,
      placeholder: record.placeholder,
      rows: record.rows,
      readonly: record.readonly,
      advanced: record.advanced,
      hidden: record.hidden,
      group: record.group,
      order: record.order,
      groupOrder: record.groupOrder,
    };
  }

  function onChange(path: string, value: ControlValue) {
    values = { ...values, [path]: value };
    dirty = new Set(dirty).add(path);

    const record = records.find((candidate) => candidate.path === path);
    if (!record || record.readonly) return;

    const literal = toNix(record, value);
    const previous = timers.get(path);
    if (previous) clearTimeout(previous);
    timers.set(
      path,
      setTimeout(() => {
        rpc("control.set", { path, value: literal }, user.csrf).catch((cause) => {
          error = cause instanceof Error ? cause.message : "Cannot stage the value";
        });
      }, 250),
    );
  }

  async function preview() {
    error = "";
    try {
      const result = await rpc<PreviewResult>("edits.preview", {}, user.csrf);
      diff = result.edits.length
        ? result.edits
            .map(
              (edit) =>
                `--- ${edit.file}\n- ${edit.before ?? ""}\n+ ${edit.after}`,
            )
            .join("\n\n")
        : "No staged changes.";
      showDiff = true;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot preview changes";
    }
  }

  async function save() {
    busy = true;
    error = "";
    status = "Saving...";
    try {
      const result = await rpc<{ files: number }>("edits.apply", {}, user.csrf);
      status = `Saved ${result.files} file(s)`;
      const scanned = await rpc<ScanResult>("scan", {}, user.csrf);
      menus = scanned.menus;
      loadPage();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Save failed";
    } finally {
      busy = false;
    }
  }

  async function discard() {
    if (dirty.size && !confirm("Discard staged changes?")) return;
    try {
      await rpc("session.discard", {}, user.csrf);
      const scanned = await rpc<ScanResult>("scan", {}, user.csrf);
      menus = scanned.menus;
      loadPage();
      status = "Changes discarded";
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot reload";
    }
  }

  async function signOut() {
    await logout();
    onLogout();
  }
</script>

<div class="shell">
  <header class="topbar">
    <div class="title">
      <span class="mark">N</span>
      <span>{root ? root.split("/").at(-1) : "Settings"}</span>
    </div>
    <div class="topbar-actions">
      <span class="chip">{themeName}</span>
      <span class="chip muted">{user.username}</span>
      <button class="ghost" onclick={signOut}>Sign out</button>
    </div>
  </header>

  <aside class="sidebar">
    <nav>
      <button class:active={view === "flake"} onclick={() => (view = "flake")}>
        Flake Inputs
      </button>
      <p class="nav-label">Menus</p>
      {#each sortedMenus as menu, index (menu.file)}
        <button
          class:active={view === "settings" && selected === index}
          onclick={() => selectMenu(index)}
        >
          {menu.displayname}
        </button>
      {/each}
      {#if sortedMenus.length === 0}
        <p class="muted small">Open a flake to list menus.</p>
      {/if}
    </nav>
  </aside>

  <main class="content">
    {#if menus.length === 0}
      <Panel title="Open a flake" description="The directory is scanned for @menu and @option markers.">
        <form
          class="open-form"
          onsubmit={(event) => {
            event.preventDefault();
            openRoot(root);
          }}
        >
          <label>
            <span>Flake or configuration directory</span>
            <input bind:value={root} placeholder="/home/den/repos/nixos-configs/dendrite" />
          </label>
          <button type="submit" disabled={busy}>{busy ? "Opening..." : "Open"}</button>
        </form>
      </Panel>
    {:else if view === "flake"}
      <Panel title="Flake Inputs" description="The nixpkgs card and input editor are the next engine slice.">
        <p class="muted">No input data yet.</p>
      </Panel>
    {:else}
      <Panel title={sortedMenus[selected]?.displayname ?? "Settings"} description={sortedMenus[selected]?.description}>
        <div class="toolbar">
          <label class="inline">
            <input type="checkbox" bind:checked={advanced} />
            <span>Advanced</span>
          </label>
          <span class="spacer"></span>
          <button class="ghost" onclick={discard}>Discard</button>
          <button class="ghost" onclick={preview}>Diff</button>
          <button onclick={save} disabled={busy || dirty.size === 0}>
            {busy ? "Saving..." : "Save"}
          </button>
        </div>

        {#if visibleRecords.length === 0}
          <p class="muted">This menu has no controls.</p>
        {/if}

        {#each visibleRecords as record (record.path)}
          <Control
            descriptor={descriptor(record)}
            value={values[record.path]}
            onchange={onChange}
          />
        {/each}
      </Panel>

      {#if showDiff}
        <Panel title="Pending changes" description="Nothing is written until Save.">
          <pre class="diff">{diff}</pre>
        </Panel>
      {/if}
    {/if}

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
  </main>

  <footer class="bottombar">
    <div class="left">
      <span class="muted small">{status}</span>
    </div>
    <div class="right">
      <span class="chip muted">{dirty.size} staged</span>
      <span class="chip muted">{engine}</span>
      <span class="chip">{schemeById(themeName).name}</span>
    </div>
  </footer>
</div>
