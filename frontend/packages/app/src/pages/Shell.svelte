<script lang="ts">
  import { onMount } from "svelte";
  import {
    Control,
    OJUNY_WIDGETS,
    OJUNY_WIDGET_KINDS,
    Panel,
    applyTheme,
    schemeById,
    type ControlDescriptor,
    type ControlValue,
  } from "@nix-web-gui/ui";
  import CardCollection from "./CardCollection.svelte";
  import MediaPicker from "./MediaPicker.svelte";
  import SecureBootPanel from "./SecureBootPanel.svelte";
  import {
    getState,
    logout,
    rpc,
    type PreviewResult,
    type FileTemplate,
    type FlakeInputJson,
    type FlakeInputsResult,
    type GitStatus,
    type InputCheck,
    type WebConfig,
    type ScanInsert,
    type ScanMenu,
    type ScanRecord,
    type ScanResult,
    type ScanSnippet,
    type StyleEntry,
    type User,
  } from "../api";

  export let user: User;
  export let onLogout: () => void;
  export let popout: string | null = null;

  let root = "";
  let search = "";
  let menus: ScanMenu[] = [];
  let hosts: string[] = [];
  let hostname = "";
  let recentFlakes: string[] = [];
  let systemOnly = false;
  let selectedHost = "";
  let selected = 0;
  let view: "settings" | "flake" | "git" = "settings";
  let popoutMode = false;
  let records: ScanRecord[] = [];
  let values: Record<string, ControlValue> = {};
  let originalValues: Record<string, ControlValue> = {};
  let dirty: Set<string> = new Set();
  let highlightPath = "";
  let collapsed = new Set<string>();
  let advanced = false;
  let busy = false;
  let status = "";
  let error = "";
  let diff = "";
  let showDiff = false;
  let rebuildOutput = "";
  let rebuildRunning = false;
  let cancelRequested = false;
  let themeName = "everforest-dark-hard";
  let engine = "loading";
  let insert: { menuFile: string; marker: ScanInsert; snippet: ScanSnippet } | null =
    null;
  let insertValues: Record<string, string> = {};
  let insertBusy = false;
  let flakeData: FlakeInputsResult | null = null;
  let flakeBusy = false;
  let flakeCheckBusy = false;
  let updateChecks: Record<string, InputCheck> = {};
  let inputName = "";
  let inputUrl = "";
  let inputDescription = "";
  let editingName: string | null = null;
  let gitStatus: GitStatus | null = null;
  let gitBusy = false;
  let gitDiff = "";
  let gitDiffBusy = false;
  let commitMessage = "";
  let liveApply = true;
  let autoGitSync = false;
  let expandDetailsDefault = false;
  let config: WebConfig | null = null;
  let showConfig = false;
  let configBusy = false;
  let rebuildCommand = "";
  let terminalCommand = "";
  let configFlakePath = "";
  let showAddFile = false;
  let fileTemplates: FileTemplate[] = [];
  let addFileKind = "nixos";
  let addFilePath = "";
  let addFileModuleName = "";
  let addFileTemplate: number | null = null;
  let addFilePlaceholders: Record<string, string> = {};
  let addFileBusy = false;
  let showAddOption = false;
  let optionPath = "";
  let optionWidget = "text";
  let optionLabel = "";
  let optionValue = "";
  let optionBusy = false;
  let structureTarget: ScanRecord | null = null;
  let structureKey = "";
  let structureMode: { width: number; height: number; refresh: number } | null =
    null;
  let structureBusy = false;
  let detectedOutputs: Array<{
    name: string;
    make?: string | null;
    model?: string | null;
    enabled: boolean;
    width?: number | null;
    height?: number | null;
    refresh?: number | null;
  }> = [];

  const optionWidgets = [
    "switch",
    "dropdown",
    "radio",
    "spin",
    "slider",
    "text",
    "multiline",
    "path",
    "package",
    "packages",
    "list",
    "color",
    "submodule",
    "users",
    "facter",
    "docker",
    "raw",
  ];

  const timers = new Map<string, ReturnType<typeof setTimeout>>();
  const pendingApplies = new Map<string, string>();
  let applyRunning = false;
  let pageToken = 0;
  let fontEntries: StyleEntry[] = [];
  let iconEntries: StyleEntry[] = [];
  let cursorEntries: StyleEntry[] = [];

  function selectStartupPage(pattern: string) {
    try {
      const expression = new RegExp(pattern, "i");
      const index = sortMenus(menus).findIndex(
        (menu) =>
          expression.test(menu.displayname) || expression.test(menu.file),
      );
      if (index >= 0) {
        selected = index;
        loadPage();
      }
    } catch {
      // Keep the first menu on an invalid page pattern.
    }
  }

  function toggleCategory(category: string) {
    const next = new Set(collapsed);
    if (next.has(category)) next.delete(category);
    else next.add(category);
    collapsed = next;
  }

  function sortMenus(input: ScanMenu[]) {
    return [...input].sort(
      (a, b) => a.order - b.order || a.displayname.localeCompare(b.displayname),
    );
  }

  $: sortedMenus = sortMenus(menus);
  $: ojunyPaths = OJUNY_WIDGETS
    ? []
    : records
        .filter((record) => OJUNY_WIDGET_KINDS.has(record.widget ?? ""))
        .map((record) => record.path);
  $: cardPaths = new Set(
    records
      .filter(
        (record) =>
          (record.children?.length ?? 0) > 0 && record.widget !== "submodule",
      )
      .flatMap((record) => descendantPaths(record)),
  );
  $: visibleRecords = records.filter((record) => {
    if (!OJUNY_WIDGETS && ojunyPaths.some((path) => record.path.startsWith(`${path}.`))) {
      return false;
    }
    if (cardPaths.has(record.path)) return false;
    if (disabledByAncestor(record)) return false;
    if (record.hidden && !advanced) return false;
    if (!visibleWhenHolds(record)) return false;
    if (!searchTerm) return true;
    return subtreeMatches(record, searchTerm);
  });
  $: currentMenu = sortedMenus[selected];
  $: orderedRecords = [...visibleRecords].sort(
    (left, right) =>
      (left.groupOrder ?? 1e9) - (right.groupOrder ?? 1e9) ||
      (left.group ?? "").localeCompare(right.group ?? "") ||
      (left.order ?? 1e9) - (right.order ?? 1e9) ||
      left.path.localeCompare(right.path),
  );
  $: groupedRecords = (() => {
    const normal = orderedRecords.filter(
      (record) => !record.advanced && !record.hidden,
    );
    const advancedRecords = orderedRecords.filter(
      (record) => record.advanced || record.hidden,
    );
    const groups: Array<{ name: string; records: ScanRecord[] }> = [];
    for (const record of normal) {
      const name = record.group ?? "";
      const last = groups.at(-1);
      if (last && last.name === name) {
        last.records.push(record);
      } else {
        groups.push({ name, records: [record] });
      }
    }
    if (advanced && advancedRecords.length) {
      groups.push({ name: "Advanced", records: advancedRecords });
    }
    return groups;
  })();
  $: searchTerm = search.trim().toLowerCase();
  $: filteredMenus = sortedMenus.filter((menu) => {
    if (!searchTerm) return true;
    if (
      menu.displayname.toLowerCase().includes(searchTerm) ||
      (menu.description ?? "").toLowerCase().includes(searchTerm)
    ) {
      return true;
    }
    return menu.records.some(
      (record) =>
        record.path.toLowerCase().includes(searchTerm) ||
        (record.label ?? "").toLowerCase().includes(searchTerm) ||
        (record.description ?? "").toLowerCase().includes(searchTerm),
    );
  });
  $: menuGroups = Object.entries(
    filteredMenus.reduce<Record<string, ScanMenu[]>>((groups, menu) => {
      const category = menu.category ?? "";
      (groups[category] ??= []).push(menu);
      return groups;
    }, {}),
  ).sort(([left], [right]) => left.localeCompare(right)) as Array<
    [string, ScanMenu[]]
  >;
  $: searchMatches = searchTerm
    ? sortedMenus
        .flatMap((menu) =>
          menu.records
            .filter(
              (record) =>
                (!record.hidden || advanced) &&
                matchesRecord(record, searchTerm),
            )
            .map((record) => ({ menu, record })),
        )
        .slice(0, 40)
    : [];

  function descendantPaths(record: ScanRecord): string[] {
    return (record.children ?? []).flatMap((child) => [
      child.path,
      ...descendantPaths(child),
    ]);
  }

  function subtreeMatches(record: ScanRecord, term: string): boolean {
    if (matchesRecord(record, term)) return true;
    return (record.children ?? []).some((child) => subtreeMatches(child, term));
  }

  function disabledByAncestor(record: ScanRecord): boolean {
    return records.some((candidate) => {
      if (!candidate.disabledFeatures?.length) return false;
      const prefix = `${candidate.path}.`;
      if (!record.path.startsWith(prefix)) return false;
      const relative = record.path.slice(prefix.length);
      return candidate.disabledFeatures.some(
        (feature) => relative === feature || relative.startsWith(`${feature}.`),
      );
    });
  }

  function cardRecordVisible(record: ScanRecord): boolean {
    if (record.hidden && !advanced) return false;
    if (disabledByAncestor(record)) return false;
    return visibleWhenHolds(record);
  }

  function matchesRecord(record: ScanRecord, term: string): boolean {
    return (
      record.path.toLowerCase().includes(term) ||
      (record.label ?? "").toLowerCase().includes(term) ||
      (record.description ?? "").toLowerCase().includes(term)
    );
  }

  onMount(() => {
    const syncTheme = () => {
      themeName = document.documentElement.dataset.scheme ?? themeName;
    };
    syncTheme();
    window.addEventListener("nix-web-gui:theme-change", syncTheme);
    window.addEventListener("keydown", onKeydown);
    window.addEventListener("beforeunload", onBeforeUnload);
    window.addEventListener("nix-web-gui:progress", onProgress);
    popoutMode = Boolean(popout);

    loadCatalogs();
    rpc<WebConfig>("config.get", {}, user.csrf)
      .then((loaded) => {
        liveApply = loaded.live_apply;
        autoGitSync = loaded.auto_git_sync;
        expandDetailsDefault = loaded.expand_details_by_default;
      })
      .catch(() => {
        // Keep the defaults when config is unavailable.
      });
    rpc<{ engine: string }>("system.capabilities", {}, user.csrf)
      .then((capabilities) => {
        engine = capabilities.engine;
      })
      .catch((cause) => {
        engine = cause instanceof Error ? cause.message : "unavailable";
      });

    getState()
      .then(async (state) => {
        hostname = state.hostname ?? "";
        recentFlakes = state.recentFlakes ?? [];
        const initial =
          state.flake ??
          state.configFlake ??
          localStorage.getItem("nix-web-gui.root");
        if (initial) {
          root = initial;
          await openRoot(initial);
          if (popout) selectStartupPage(popout);
          else if (state.page) selectStartupPage(state.page);
        }
      })
      .catch((cause) => {
        error = cause instanceof Error ? cause.message : "Cannot read server state";
      });

    return () => {
      window.removeEventListener("nix-web-gui:theme-change", syncTheme);
      window.removeEventListener("keydown", onKeydown);
      window.removeEventListener("beforeunload", onBeforeUnload);
      window.removeEventListener("nix-web-gui:progress", onProgress);
    };
  });

  function onProgress(event: Event) {
    const detail = (event as CustomEvent).detail as {
      params?: { job?: string; line?: string };
    };
    if (detail?.params?.job === "update") {
      status = `Checked ${detail.params.line ?? ""}`;
      return;
    }
    if (
      detail?.params?.job === "scan" ||
      detail?.params?.job === "save" ||
      detail?.params?.job === "apply"
    ) {
      status = detail.params.line ?? detail.params.job;
      return;
    }
    if (detail?.params?.job !== "rebuild") return;
    const next = [...rebuildOutput.split("\n"), detail.params.line ?? ""]
      .filter((line, index, lines) => line !== "" || index === lines.length - 1)
      .slice(-500);
    rebuildOutput = next.join("\n");
    status = "Rebuilding...";
  }

  function onBeforeUnload(event: BeforeUnloadEvent) {
    if (dirty.size === 0) return;
    event.preventDefault();
    event.returnValue = "";
  }

  function onKeydown(event: KeyboardEvent) {
    const modifier = event.ctrlKey || event.metaKey;
    if (modifier && event.key.toLowerCase() === "s") {
      event.preventDefault();
      if (event.shiftKey) {
        saveAndRebuild();
      } else {
        save();
      }
    } else if (modifier && event.key.toLowerCase() === "f") {
      event.preventDefault();
      document.querySelector<HTMLInputElement>(".search-input")?.focus();
    } else if (modifier && event.key.toLowerCase() === "r") {
      event.preventDefault();
      discard();
    } else if (event.key === "Escape") {
      showAddFile = false;
      showAddOption = false;
      showConfig = false;
      structureTarget = null;
      insert = null;
    }
  }

  async function loadCatalogs() {
    try {
      const [fonts, icons, cursors] = await Promise.all([
        rpc<StyleEntry[]>("catalog.styles", { kind: "fonts" }, user.csrf),
        rpc<StyleEntry[]>("catalog.styles", { kind: "icons" }, user.csrf),
        rpc<StyleEntry[]>("catalog.styles", { kind: "cursors" }, user.csrf),
      ]);
      fontEntries = fonts;
      iconEntries = icons;
      cursorEntries = cursors;
    } catch {
      // Keep text inputs when the catalog is unavailable.
    }
  }

  async function openFlakeInputs() {
    view = "flake";
    await loadFlakeInputs();
  }

  async function loadFlakeInputs() {
    flakeBusy = true;
    error = "";
    try {
      flakeData = await rpc<FlakeInputsResult>("flake.inputs", {}, user.csrf);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot load flake inputs";
    } finally {
      flakeBusy = false;
    }
  }

  async function addMissingInputs() {
    flakeBusy = true;
    error = "";
    try {
      const result = await rpc<{ added: string[] }>(
        "flake.input.addAll",
        {},
        user.csrf,
      );
      status = result.added.length
        ? `Declared ${result.added.join(", ")}`
        : "No missing inputs";
      await loadFlakeInputs();
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Cannot declare missing inputs";
    } finally {
      flakeBusy = false;
    }
  }

  async function reopenAfterCancel(message: string) {
    if (!root) return;
    dirty = new Set();
    try {
      await rpc("session.open", { root }, user.csrf);
      const scanned = await rpc<ScanResult>("scan", {}, user.csrf);
      menus = scanned.menus;
      loadPage();
      status = message;
    } catch {
      status = `${message}; reopen the flake to continue`;
    }
  }

  async function checkInputUpdates(names: string[]) {
    flakeCheckBusy = true;
    cancelRequested = false;
    error = "";
    try {
      const result = await rpc<InputCheck[]>(
        "flake.update.check",
        { names },
        user.csrf,
      );
      updateChecks = {
        ...updateChecks,
        ...Object.fromEntries(result.map((check) => [check.name, check])),
      };
      status = "Checked input revisions";
    } catch (cause) {
      if (cancelRequested) {
        await reopenAfterCancel("Update check cancelled");
      } else {
        error =
          cause instanceof Error ? cause.message : "Cannot check input revisions";
      }
    } finally {
      flakeCheckBusy = false;
    }
  }

  async function updateInputs(names: string[]) {
    flakeBusy = true;
    error = "";
    try {
      const result = await rpc<{ updated: number }>(
        "flake.update",
        { names },
        user.csrf,
      );
      status = `Updated ${result.updated} input(s)`;
      await loadFlakeInputs();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot update inputs";
    } finally {
      flakeBusy = false;
    }
  }

  function checkLabel(name: string): string {
    const check = updateChecks[name];
    if (!check) return "";
    if (check.error) return check.error;
    if (check.state === "update-available") return "update available";
    return check.state ?? "";
  }

  async function openAddFile() {
    view = "settings";
    error = "";
    try {
      fileTemplates = await rpc<FileTemplate[]>("file.templates", {}, user.csrf);
      addFileKind = "nixos";
      addFileTemplate = null;
      addFileModuleName = "";
      addFilePath = `${root}/modules/module.nix`;
      addFilePlaceholders = {};
      showAddFile = true;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot read templates";
    }
  }

  function onFileKindChange() {
    if (addFileKind.startsWith("template:")) {
      const index = Number(addFileKind.slice("template:".length));
      const template = fileTemplates[index];
      if (!template) return;
      addFileTemplate = template.index;
      addFilePath = `${root}/${template.output}/${template.filename}`;
      addFilePlaceholders = Object.fromEntries(
        template.placeholders.map((placeholder) => [
          placeholder.name,
          placeholder.default,
        ]),
      );
      return;
    }
    addFileTemplate = null;
    addFilePath = `${root}/modules/module.nix`;
    addFilePlaceholders = {};
  }

  async function saveAddFile(event: SubmitEvent) {
    event.preventDefault();
    addFileBusy = true;
    error = "";
    try {
      const result = await rpc<{ path: string; reminder: string }>(
        "file.add",
        {
          path: addFilePath,
          kind: addFileKind.startsWith("template:") ? "nixos" : addFileKind,
          moduleName: addFileModuleName,
          template: addFileTemplate,
          placeholders: addFilePlaceholders,
        },
        user.csrf,
      );
      showAddFile = false;
      status = `Created ${result.path}${result.reminder ? ` · ${result.reminder}` : ""}`;
      const scanned = await rpc<ScanResult>("scan", {}, user.csrf);
      menus = scanned.menus;
      loadPage();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot create file";
    } finally {
      addFileBusy = false;
    }
  }

  function openAddOption() {
    showAddOption = true;
    optionPath = "";
    optionWidget = "text";
    optionLabel = "";
    optionValue = "";
    error = "";
  }

  async function saveAddOption(event: SubmitEvent) {
    event.preventDefault();
    if (!currentMenu) return;
    optionBusy = true;
    error = "";
    try {
      await rpc(
        "option.add",
        {
          file: currentMenu.file,
          origin: currentMenu.origin ?? "",
          path: optionPath,
          widget: optionWidget,
          label: optionLabel,
          value: optionValue,
        },
        user.csrf,
      );
      showAddOption = false;
      status = `Added ${optionPath}`;
      const scanned = await rpc<ScanResult>("scan", {}, user.csrf);
      menus = scanned.menus;
      loadPage();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot add option";
    } finally {
      optionBusy = false;
    }
  }

  function promptOpenFlake() {
    const next = prompt("Flake or configuration directory", root);
    if (next && next.trim()) openRoot(next.trim());
  }

  async function reloadFromDisk() {
    if (dirty.size > 0 && !confirm("Discard staged changes and reload from disk?")) {
      return;
    }
    try {
      await rpc("session.reload", {}, user.csrf);
      const scanned = await rpc<ScanResult>("scan", {}, user.csrf);
      menus = scanned.menus;
      hosts = scanned.hosts ?? [];
      loadPage();
      status = "Reloaded from disk";
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot reload";
    }
  }

  async function openConfig() {
    view = "settings";
    error = "";
    try {
      config = await rpc<WebConfig>("config.get", {}, user.csrf);
      rebuildCommand = config.rebuild_command ?? "";
      terminalCommand = config.terminal_command ?? "";
      configFlakePath = config.flake_path ?? "";
      showConfig = true;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot read config";
    }
  }

  async function saveConfig(event: SubmitEvent) {
    event.preventDefault();
    if (!config) return;
    configBusy = true;
    error = "";
    try {
      const next: WebConfig = {
        ...config,
        flake_path: configFlakePath.trim() || null,
        rebuild_command: rebuildCommand.trim() || null,
        terminal_command: terminalCommand.trim() || null,
      };
      await rpc("config.set", next, user.csrf);
      config = next;
      liveApply = next.live_apply;
      autoGitSync = next.auto_git_sync;
      expandDetailsDefault = next.expand_details_by_default;
      showConfig = false;
      status = "Config saved";
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot save config";
    } finally {
      configBusy = false;
    }
  }

  async function refreshGit() {
    if (!root) return;
    try {
      gitStatus = await rpc<GitStatus>("git.status", {}, user.csrf);
    } catch {
      gitStatus = null;
    }
  }

  async function gitSync() {
    if (!root) {
      error = "Open a flake first";
      return;
    }
    gitBusy = true;
    error = "";
    try {
      const current = await rpc<GitStatus>("git.status", {}, user.csrf);
      if (current.dirty) {
        const message = prompt("Commit message", "Update settings");
        if (!message) return;
        await rpc("git.sync", { message }, user.csrf);
        status = `Committed and pushed ${current.branch}`;
      } else {
        await rpc("git.pull", {}, user.csrf);
        status = `Pulled ${current.branch}`;
      }
      await refreshGit();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Git sync failed";
    } finally {
      gitBusy = false;
    }
  }

  async function openGit() {
    view = "git";
    gitDiff = "";
    await refreshGit();
  }

  async function gitCommit() {
    if (!root) return;
    gitBusy = true;
    error = "";
    try {
      const result = await rpc<{ commit: string }>(
        "git.commit",
        { message: commitMessage || "Update settings" },
        user.csrf,
      );
      commitMessage = "";
      status = `Committed ${result.commit}`;
      await refreshGit();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Commit failed";
    } finally {
      gitBusy = false;
    }
  }

  async function gitPush() {
    if (!root) return;
    gitBusy = true;
    error = "";
    try {
      const result = await rpc<{ pushed: string }>("git.push", {}, user.csrf);
      status = `Pushed to ${result.pushed}`;
      await refreshGit();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Push failed";
    } finally {
      gitBusy = false;
    }
  }

  async function gitPull() {
    if (!root) return;
    gitBusy = true;
    error = "";
    try {
      await rpc("git.pull", {}, user.csrf);
      status = "Pulled";
      await refreshGit();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Pull failed";
    } finally {
      gitBusy = false;
    }
  }

  async function loadGitDiff() {
    gitDiffBusy = true;
    error = "";
    try {
      const result = await rpc<{ diff: string }>("git.diff", {}, user.csrf);
      gitDiff = result.diff.trim() ? result.diff : "No working-tree changes.";
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot read the diff";
    } finally {
      gitDiffBusy = false;
    }
  }

  function newInput() {
    editingName = null;
    inputName = "";
    inputUrl = "";
    inputDescription = "";
  }

  function editInput(input: FlakeInputJson) {
    editingName = input.name;
    inputName = input.name;
    inputUrl = input.url ?? "";
    inputDescription = input.description ?? "";
  }

  async function saveInput(event: SubmitEvent) {
    event.preventDefault();
    if (!inputName.trim() || !inputUrl.trim()) {
      error = "Input name and URL are required";
      return;
    }
    flakeBusy = true;
    error = "";
    try {
      await rpc(
        "flake.input.upsert",
        {
          name: inputName,
          url: inputUrl,
          flake: true,
          description: inputDescription.trim() || null,
        },
        user.csrf,
      );
      status = `Saved ${inputName}`;
      newInput();
      await loadFlakeInputs();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot save input";
    } finally {
      flakeBusy = false;
    }
  }

  async function deleteInput(name: string) {
    if (!confirm(`Delete input ${name}?`)) return;
    flakeBusy = true;
    error = "";
    try {
      await rpc("flake.input.delete", { name }, user.csrf);
      status = `Deleted ${name}`;
      await loadFlakeInputs();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot delete input";
    } finally {
      flakeBusy = false;
    }
  }

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
      if (autoGitSync) {
        try {
          await rpc("git.pull", {}, user.csrf);
        } catch (cause) {
          status =
            cause instanceof Error ? `Git pull failed: ${cause.message}` : "Git pull failed";
        }
      }
      const scanned = await rpc<ScanResult>("scan", {}, user.csrf);
      menus = scanned.menus;
      hosts = scanned.hosts ?? [];
      selectedHost = hosts.includes(hostname) ? hostname : hosts[0] ?? "";
      selected = 0;
      view = "settings";
      loadPage();
      localStorage.setItem("nix-web-gui.root", path);
      status = `Opened ${opened.menus} menus, ${opened.records} records`;
      refreshGit();
      rpc("config.remember", { flake: path }, user.csrf).catch(() => {
        // A read-only config directory must not block opening a flake.
      });
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot open the flake";
    } finally {
      busy = false;
    }
  }

  async function applyHostFilter() {
    if (!root) return;
    error = "";
    try {
      const scanned = await rpc<ScanResult>(
        "scan",
        systemOnly && selectedHost ? { host: selectedHost } : {},
        user.csrf,
      );
      menus = scanned.menus;
      selected = 0;
      loadPage();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot filter by host";
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
    originalValues = { ...values };
    dirty = new Set();
    showDiff = false;
    pageToken += 1;
    describePage(pageToken, menu?.file);
  }

  function flattenControls(
    controls: ScanRecord[],
    depth = 0,
  ): ScanRecord[] {
    return controls.flatMap((control) => {
      const current = { ...control, depth };
      const children = control.children?.length
        ? flattenControls(control.children, depth + 1)
        : [];
      return [current, ...children];
    });
  }

  function describePage(token: number, file: string | undefined) {
    if (!file) return;
    rpc<{ controls: ScanRecord[] }>("page.describe", { file }, user.csrf)
      .then((described) => {
        if (token !== pageToken) return;
        if (dirty.size > 0) {
          const byPath = new Map(
            described.controls.map((control) => [control.path, control]),
          );
          records = records.map((record) => ({
            ...record,
            ...byPath.get(record.path),
          }));
          return;
        }
        records = flattenControls(described.controls);
        values = Object.fromEntries(
          records.map((record) => [record.path, displayValue(record)]),
        );
        originalValues = { ...values };
      })
      .catch(() => {
        // Keep the scanned shape when metadata is unavailable.
      });
  }

  function selectMenu(menu: ScanMenu) {
    selected = sortedMenus.findIndex((candidate) => candidate.file === menu.file);
    view = "settings";
    loadPage();
  }

  function goToMatch(menu: ScanMenu, record: ScanRecord) {
    selectMenu(menu);
    highlightPath = record.path;
    setTimeout(() => {
      document
        .getElementById(`control-${record.path.replaceAll(".", "-")}`)
        ?.scrollIntoView({ block: "center", behavior: "smooth" });
    }, 150);
    setTimeout(() => {
      if (highlightPath === record.path) highlightPath = "";
    }, 2400);
  }

  async function resetOption(record: ScanRecord) {
    try {
      await rpc("control.reset", { path: record.path }, user.csrf);
      const original = originalValues[record.path];
      values = {
        ...values,
        [record.path]: original ?? displayValue(record),
      };
      const next = new Set(dirty);
      next.delete(record.path);
      dirty = next;
      status = `Reset ${record.label ?? record.path}`;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot reset the value";
    }
  }

  const addableWidgets = new Set([
    "users",
    "keybinds",
    "monitors",
    "docker",
    "menu",
    "submodule",
    "horizontal-submodule",
    "grid-configuration",
  ]);

  function parentOf(record: ScanRecord): ScanRecord | null {
    return (
      records.find((candidate) =>
        candidate.children?.some((child) => child.path === record.path),
      ) ?? null
    );
  }

  function styleEntriesFor(record: ScanRecord): StyleEntry[] {
    if (record.widget === "icon-theme") {
      return record.variant
        ? iconEntries.filter((entry) => entry.variant === record.variant)
        : iconEntries;
    }
    if (record.widget === "cursor-theme") return cursorEntries;
    return [];
  }

  function stylePreview(entry: StyleEntry): string | null {
    return entry.preview ?? entry.previews?.[0] ?? null;
  }

  function keybindAncestor(record: ScanRecord): ScanRecord | null {
    return (
      records.find(
        (candidate) =>
          candidate.widget === "keybinds" &&
          (record.path === candidate.path ||
            record.path.startsWith(`${candidate.path}.`)),
      ) ?? null
    );
  }

  function isKeybindCsv(record: ScanRecord): boolean {
    const keybinds = keybindAncestor(record);
    if (!keybinds) return false;
    const field = record.path.split(".").at(-1) ?? "";
    const fields = keybinds.bindFields ?? ["modifier", "keys"];
    const index = fields.indexOf(field);
    const type =
      keybinds.bindFieldTypes?.[index] ??
      (field === "modifier" || field === "keys" ? "list" : "str");
    return type.toLowerCase() === "list";
  }

  function bindPreviewValue(record: ScanRecord, field: string): string {
    const child = (record.children ?? []).find(
      (candidate) => candidate.path.split(".").at(-1) === field,
    );
    if (!child) return "";
    const value = values[child.path];
    if (Array.isArray(value)) return value.join("+");
    if (typeof value === "boolean") return value ? "on" : "off";
    return String(value ?? "");
  }

  function cardRowTitle(record: ScanRecord): string {
    const fallback = record.label ?? record.path.split(".").at(-1) ?? record.path;
    const keybinds = keybindAncestor(record);
    const preview = keybinds?.bindPreview ?? [];
    if (preview.length) {
      const text = preview
        .map((field) => bindPreviewValue(record, field))
        .filter(Boolean)
        .join(" ");
      if (text) return `${fallback}: ${text}`;
    }
    return fallback;
  }

  function cardRowSort(left: ScanRecord, right: ScanRecord): number {
    const keybinds = keybindAncestor(left) ?? keybindAncestor(right);
    const fields = keybinds?.bindSort ?? [];
    for (const field of fields) {
      const order = bindPreviewValue(left, field).localeCompare(
        bindPreviewValue(right, field),
      );
      if (order !== 0) return order;
    }
    return 0;
  }

  function canAddRow(record: ScanRecord): boolean {
    const structuredList =
      record.widget === "list" && (record.children?.length ?? 0) > 0;
    return (
      (addableWidgets.has(record.widget ?? "") || structuredList) &&
      dirty.size === 0
    );
  }

  function canRenameRow(record: ScanRecord): boolean {
    const parent = parentOf(record);
    return Boolean(
      parent &&
        addableWidgets.has(parent.widget ?? "") &&
        parent.widget !== "submodule" &&
        parent.widget !== "horizontal-submodule" &&
        dirty.size === 0,
    );
  }

  function canMoveRow(record: ScanRecord): boolean {
    const parent = parentOf(record);
    return Boolean(parent && parent.widget === "list" && dirty.size === 0);
  }

  function canRemoveRow(record: ScanRecord): boolean {
    const parent = parentOf(record);
    return Boolean(
      parent &&
        (addableWidgets.has(parent.widget ?? "") || parent.widget === "list") &&
        dirty.size === 0,
    );
  }

  async function openStructureAdd(record: ScanRecord) {
    structureTarget = record;
    structureKey = "";
    structureMode = null;
    detectedOutputs = [];
    error = "";
    if (record.widget === "monitors") {
      try {
        const detected = await rpc<{
          outputs: typeof detectedOutputs;
        }>("monitor.detect", {}, user.csrf);
        detectedOutputs = detected.outputs;
        if (detectedOutputs.length > 0) chooseOutput(detectedOutputs[0]);
      } catch {
        // Manual output names remain available when detection fails.
      }
    }
    if (record.widget === "users") structureKey = "newuser";
  }

  function chooseOutput(output: (typeof detectedOutputs)[number]) {
    structureKey = output.name;
    structureMode = {
      width: output.width ?? 1920,
      height: output.height ?? 1080,
      refresh: output.refresh ?? 60,
    };
  }

  async function addStructureRow(event: SubmitEvent) {
    event.preventDefault();
    if (!structureTarget) return;
    structureBusy = true;
    error = "";
    try {
      await rpc(
        "structure.add",
        {
          path: structureTarget.path,
          key: structureKey,
          mode: structureMode,
        },
        user.csrf,
      );
      status = `Added ${structureKey || "row"}`;
      structureTarget = null;
      await reloadStructure();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot add the row";
    } finally {
      structureBusy = false;
    }
  }

  async function renameStructureRow(record: ScanRecord) {
    const parent = parentOf(record);
    if (!parent) return;
    const current = record.path.split(".").at(-1) ?? "";
    const next = prompt("New row name", current);
    if (!next || next === current) return;
    error = "";
    try {
      await rpc(
        "structure.rename",
        { parent: parent.path, child: record.path, key: next },
        user.csrf,
      );
      status = `Renamed ${current} to ${next}`;
      await reloadStructure();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot rename the row";
    }
  }

  async function moveStructureRow(record: ScanRecord, offset: number) {
    const parent = parentOf(record);
    if (!parent) return;
    error = "";
    try {
      await rpc(
        "structure.move",
        { parent: parent.path, child: record.path, offset },
        user.csrf,
      );
      await reloadStructure();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot move the row";
    }
  }

  async function removeStructureRow(record: ScanRecord) {
    const parent = parentOf(record);
    if (!parent) return;
    if (!confirm(`Remove ${record.label ?? record.path}?`)) return;
    error = "";
    try {
      await rpc(
        "structure.remove",
        { parent: parent.path, child: record.path },
        user.csrf,
      );
      status = `Removed ${record.label ?? record.path}`;
      await reloadStructure();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot remove the row";
    }
  }

  async function reloadStructure() {
    const scanned = await rpc<ScanResult>("scan", {}, user.csrf);
    menus = scanned.menus;
    const file = currentMenu?.file;
    const index = sortMenus(menus).findIndex((menu) => menu.file === file);
    selected = index >= 0 ? index : 0;
    loadPage();
  }

  function displayValue(record: ScanRecord): ControlValue {
    if (
      record.widget === "list" ||
      record.widget === "packages" ||
      record.widget === "nixos-modules" ||
      record.widget === "hm-modules"
    ) {
      return record.list ?? [];
    }
    if (record.widget === "table-attrset" || record.widget === "table-list") {
      return {
        rows: (record.table?.rows ?? []).map((row) => ({
          key: row.key,
          fields: { ...row.fields },
        })),
      };
    }
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

  function literalOf(value: ControlValue): string {
    if (typeof value === "boolean") return value ? "true" : "false";
    if (typeof value === "number") return String(value);
    if (Array.isArray(value)) return "(list)";
    return JSON.stringify(String(value ?? ""));
  }

  function visibleWhenHolds(record: ScanRecord): boolean {
    const condition = record.visibleWhen;
    if (!condition) return true;
    if (!records.some((candidate) => candidate.path === condition.path)) {
      return true;
    }
    const current = literalOf(values[condition.path]).trim();
    if (condition.equals != null) {
      return current === condition.equals.trim();
    }
    if (condition.equalsAny?.length) {
      return condition.equalsAny.some((value) => value.trim() === current);
    }
    if (condition.notEquals != null) {
      return current !== condition.notEquals.trim();
    }
    return !["false", "null", "0", "0.0"].includes(current);
  }

  function withCurrent(choices: string[], current: string): string[] {
    return current && !choices.includes(current) ? [current, ...choices] : choices;
  }

  function descriptor(record: ScanRecord): ControlDescriptor {
    const widget = (record.widget ?? "text") as ControlDescriptor["widget"];
    const value = String(displayValue(record) ?? "");
    const result: ControlDescriptor = {
      path: record.path,
      widget,
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
      children: record.children?.map(descriptor),
      depth: record.depth ?? 0,
      table: record.table,
      origin: record.origin,
      visibleWhen: record.visibleWhen,
    };
    if (widget === "font") {
      const entries = record.monospace
        ? fontEntries.filter((entry) => entry.monospace)
        : fontEntries;
      result.choices = withCurrent(
        entries.map((entry) => entry.value),
        value,
      );
    }
    if (widget === "icon-theme") {
      const entries = record.variant
        ? iconEntries.filter((entry) => entry.variant === record.variant)
        : iconEntries;
      result.choices = withCurrent(
        entries.map((entry) => entry.value),
        value,
      );
    }
    if (widget === "cursor-theme") {
      result.choices = withCurrent(
        cursorEntries.map((entry) => entry.value),
        value,
      );
    }
    return result;
  }

  function scheduleApply(path: string, literal: string) {
    pendingApplies.set(path, literal);
    void flushApplies();
  }

  async function flushApplies() {
    if (applyRunning) return;
    applyRunning = true;
    try {
      while (pendingApplies.size > 0) {
        const [path, literal] = pendingApplies.entries().next().value as [
          string,
          string,
        ];
        pendingApplies.delete(path);
        try {
          await rpc("apply.run", { path, value: literal }, user.csrf);
          status = `Applied ${path}`;
        } catch (cause) {
          error =
            cause instanceof Error ? cause.message : "Cannot apply the value";
        }
      }
    } finally {
      applyRunning = false;
    }
  }

  function onChange(path: string, value: ControlValue) {
    values = { ...values, [path]: value };
    dirty = new Set(dirty).add(path);

    const record = records.find((candidate) => candidate.path === path);
    if (!record || record.readonly) return;
    if (record.widget === "theme" && typeof value === "string" && value) {
      applyTheme(value);
    }

    const previous = timers.get(path);
    if (previous) clearTimeout(previous);
    timers.set(
      path,
      setTimeout(() => {
        void (async () => {
          try {
            const staged = await rpc<{
              value: string;
              packagePath?: string;
              packageValue?: string;
            }>(
              "control.setTyped",
              { path, widget: record.widget ?? "text", value },
              user.csrf,
            );
            if (
              staged.packagePath &&
              staged.packageValue &&
              staged.packagePath in values
            ) {
              values = {
                ...values,
                [staged.packagePath]: staged.packageValue,
              };
              dirty = new Set(dirty).add(staged.packagePath);
            }
            if (record.applyCommand && liveApply) {
              scheduleApply(path, staged.value);
            }
          } catch (cause) {
            error =
              cause instanceof Error ? cause.message : "Cannot stage the value";
          }
        })();
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

  function openInsert(
    menuFile: string,
    marker: ScanInsert,
    snippet: ScanSnippet,
  ) {
    insert = { menuFile, marker, snippet };
    insertValues = Object.fromEntries(
      snippet.placeholders.map((placeholder) => [
        placeholder.name,
        placeholder.default,
      ]),
    );
  }

  async function submitInsert(event: SubmitEvent) {
    event.preventDefault();
    if (!insert) return;
    insertBusy = true;
    error = "";
    try {
      await rpc(
        "insert.apply",
        {
          file: insert.menuFile,
          name: insert.marker.name,
          offset: insert.marker.offset,
          placeholders: insertValues,
        },
        user.csrf,
      );
      insert = null;
      const scanned = await rpc<ScanResult>("scan", {}, user.csrf);
      menus = scanned.menus;
      loadPage();
      status = "Snippet inserted";
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot insert snippet";
    } finally {
      insertBusy = false;
    }
  }

  async function save(): Promise<boolean> {
    busy = true;
    error = "";
    status = "Saving...";
    try {
      const result = await rpc<{ files: number }>("edits.apply", {}, user.csrf);
      status = `Saved ${result.files} file(s)`;
      const scanned = await rpc<ScanResult>("scan", {}, user.csrf);
      menus = scanned.menus;
      loadPage();
      if (autoGitSync) {
        try {
          const current = await rpc<GitStatus>("git.status", {}, user.csrf);
          gitStatus = current;
          if (current.dirty && current.state === "current") {
            const message = prompt("Commit message", "Update settings");
            if (message) {
              await rpc("git.sync", { message }, user.csrf);
              status = `Saved and pushed ${current.branch}`;
            }
          } else if (current.state !== "current") {
            status = `Saved; git is ${current.state}`;
          }
          await refreshGit();
        } catch (cause) {
          error =
            cause instanceof Error
              ? `Saved, but git sync failed: ${cause.message}`
              : "Saved, but git sync failed";
        }
      }
      return true;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Save failed";
      return false;
    } finally {
      busy = false;
    }
  }

  async function pickIcon(
    record: ScanRecord,
    current: string,
  ): Promise<string | null> {
    return new Promise((resolve) => {
      const input = document.createElement("input");
      input.type = "file";
      input.accept = "image/png,image/jpeg,image/webp,image/svg+xml,image/gif";
      input.onchange = async () => {
        const file = input.files?.[0];
        if (!file) {
          resolve(null);
          return;
        }
        try {
          const buffer = await file.arrayBuffer();
          const result = await rpc<{ value: string }>(
            "preview.upload",
            {
              path: record.path,
              name: file.name,
              data: toBase64(buffer),
            },
            user.csrf,
          );
          resolve(result.value);
        } catch (cause) {
          error = cause instanceof Error ? cause.message : "Cannot upload the icon";
          resolve(null);
        }
      };
      input.click();
      void current;
    });
  }

  async function previewIcon(
    record: ScanRecord,
    value: string,
  ): Promise<string | null> {
    try {
      const preview = await rpc<{ mime: string; data: string }>(
        "preview.icon",
        { path: record.path, name: value },
        user.csrf,
      );
      return `data:${preview.mime};base64,${preview.data}`;
    } catch {
      return null;
    }
  }

  function toBase64(buffer: ArrayBuffer): string {
    const bytes = new Uint8Array(buffer);
    let binary = "";
    const chunk = 0x8000;
    for (let index = 0; index < bytes.length; index += chunk) {
      binary += String.fromCharCode(...bytes.subarray(index, index + chunk));
    }
    return btoa(binary);
  }

  async function setPassword(record: ScanRecord) {
    const password = prompt("New password");
    if (!password) return;
    const confirmation = prompt("Confirm password");
    if (confirmation !== password) {
      error = "Passwords do not match";
      return;
    }
    error = "";
    try {
      const result = await rpc<{ hash: string }>(
        "users.hashPassword",
        { password },
        user.csrf,
      );
      await rpc(
        "control.setTyped",
        {
          path: record.path,
          widget: record.widget ?? "text",
          value: result.hash,
        },
        user.csrf,
      );
      values = { ...values, [record.path]: result.hash };
      dirty = new Set(dirty).add(record.path);
      status = "Password hash staged";
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Cannot generate password hash";
    }
  }

  async function saveAndRebuild() {
    if (!(await save())) return;
    busy = true;
    rebuildRunning = true;
    cancelRequested = false;
    error = "";
    status = "Rebuilding...";
    try {
      const result = await rpc<{ output: string }>("rebuild.run", {}, user.csrf);
      rebuildOutput = result.output.trim();
      status = "Rebuild finished";
    } catch (cause) {
      if (!cancelRequested) {
        error = cause instanceof Error ? cause.message : "Rebuild failed";
      } else {
        await reopenAfterCancel("Rebuild cancelled; the session was reopened");
      }
    } finally {
      busy = false;
      rebuildRunning = false;
    }
  }

  async function cancelJob() {
    cancelRequested = true;
    try {
      await rpc("job.cancel", {}, user.csrf);
      status = "Cancellation requested";
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot cancel the job";
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

<div class="shell" class:popout={popoutMode}>
  <header class="topbar">
    <div class="title">
      <span class="mark">N</span>
      <span>{root ? root.split("/").at(-1) : "Settings"}</span>
      {#if !popoutMode}
        <input
          class="search-input"
          type="search"
          bind:value={search}
          placeholder="Search settings"
          aria-label="Search settings"
        />
        {#if hosts.length}
          <label class="inline">
            <input
              type="checkbox"
              bind:checked={systemOnly}
              onchange={applyHostFilter}
            />
            <span>System only</span>
          </label>
          {#if systemOnly}
            <select bind:value={selectedHost} onchange={applyHostFilter}>
              {#each hosts as host (host)}
                <option value={host}>{host}</option>
              {/each}
            </select>
          {/if}
        {/if}
      {/if}
    </div>
    <div class="topbar-actions">
      <span class="chip">{themeName}</span>
      <span class="chip muted">{user.username}</span>
      <button class="ghost" onclick={signOut}>Sign out</button>
    </div>
  </header>

  {#if !popoutMode}
    <aside class="sidebar">
      <nav>
        <button class:active={view === "flake"} onclick={openFlakeInputs}>
          Flake Inputs
        </button>
        <button class:active={view === "git"} onclick={openGit}>Git</button>
        <button onclick={promptOpenFlake}>Open flake</button>
        <button onclick={openAddFile}>Add file</button>
        <button class:active={showConfig} onclick={openConfig}>Settings</button>
        <p class="nav-label">Menus</p>
        {#each menuGroups as [category, group] (category)}
          {#if category}
            <button
              class="nav-label nav-toggle"
              aria-expanded={!collapsed.has(category)}
              onclick={() => toggleCategory(category)}
            >
              {collapsed.has(category) ? "▸" : "▾"} {category}
            </button>
          {/if}
          {#if !category || !collapsed.has(category)}
            {#each group as menu (menu.file)}
              <button
                class:active={view === "settings" && currentMenu?.file === menu.file}
                onclick={() => selectMenu(menu)}
              >
                {#if menu.icon}
                  <span class="menu-icon" title={menu.icon}></span>
                {/if}
                {menu.displayname}
              </button>
            {/each}
          {/if}
        {/each}
        {#if sortedMenus.length === 0}
          <p class="muted small">Open a flake to list menus.</p>
        {/if}
      </nav>
    </aside>
  {/if}

  <main class="content">
    {#if !popoutMode && searchTerm && searchMatches.length}
      <Panel
        title={`Search results (${searchMatches.length})`}
        description="Matches across every menu."
      >
        <div class="search-results">
          {#each searchMatches as match (match.menu.file + ":" + match.record.path)}
            <button
              type="button"
              class="search-result"
              onclick={() => goToMatch(match.menu, match.record)}
            >
              <span>{match.record.label ?? match.record.path}</span>
              <span class="muted small">
                {match.menu.displayname} · {match.record.path}
              </span>
            </button>
          {/each}
        </div>
      </Panel>
    {/if}

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
            <input
              bind:value={root}
              list="recent-flakes"
              placeholder="/home/den/repos/nixos-configs/dendrite"
            />
            <datalist id="recent-flakes">
              {#each recentFlakes as recent (recent)}
                <option value={recent}></option>
              {/each}
            </datalist>
          </label>
          <button type="submit" disabled={busy}>{busy ? "Opening..." : "Open"}</button>
        </form>
      </Panel>
    {:else if view === "flake"}
      <div class="toolbar">
        <span class="spacer"></span>
        {#if flakeCheckBusy}
          <button class="ghost" onclick={cancelJob}>Cancel check</button>
        {/if}
      </div>
      <Panel title="nixpkgs" description="The pinned revision comes from flake.lock.">
        {#if flakeData?.nixpkgs}
          <div class="input-row">
            <div>
              <strong>{flakeData.nixpkgs.name}</strong>
              <p class="muted small mono">{flakeData.nixpkgs.url}</p>
            </div>
            <div class="row-actions">
              {#if flakeData.locked?.rev}
                <span class="chip muted mono">{flakeData.locked.rev.slice(0, 12)}</span>
              {/if}
              <button
                class="ghost"
                onclick={() => checkInputUpdates(["nixpkgs"])}
                disabled={flakeCheckBusy}
              >
                Check
              </button>
              <button
                class="ghost"
                onclick={() => updateInputs(["nixpkgs"])}
                disabled={flakeBusy}
              >
                Refresh
              </button>
              <button
                class="ghost"
                onclick={() => flakeData?.nixpkgs && editInput(flakeData.nixpkgs)}
              >
                Edit
              </button>
            </div>
          </div>
        {:else}
          <p class="muted">No nixpkgs input declared.</p>
        {/if}
      </Panel>

      {#if flakeData?.missing?.length}
        <Panel
          title="Missing inputs"
          description="Declared by an @inputs marker but absent from flake.nix."
        >
          <ul class="plain-list">
            {#each flakeData.missing as input (input.name)}
              <li>
                <strong>{input.name}</strong>
                <span class="muted small mono">{input.url}</span>
              </li>
            {/each}
          </ul>
          <div class="toolbar">
            <span class="spacer"></span>
            <button onclick={addMissingInputs} disabled={flakeBusy}>
              Declare all
            </button>
          </div>
        </Panel>
      {/if}

      <Panel title="Inputs" description="Top-level flake inputs.">
        {#each (flakeData?.inputs ?? []).filter((input) => input.name !== "nixpkgs") as input (input.name)}
          <div class="input-row">
            <div>
              <strong>{input.name}</strong>
              <p class="muted small mono">{input.url ?? "no URL"}</p>
              {#if input.description}
                <p class="muted small">{input.description}</p>
              {/if}
            </div>
            <div class="row-actions">
              {#if checkLabel(input.name)}
                <span class="chip muted">{checkLabel(input.name)}</span>
              {/if}
              <button
                class="ghost"
                onclick={() => checkInputUpdates([input.name])}
                disabled={flakeCheckBusy}
              >
                Check
              </button>
              <button
                class="ghost"
                onclick={() => updateInputs([input.name])}
                disabled={flakeBusy}
              >
                Update
              </button>
              <button class="ghost" onclick={() => editInput(input)}>Edit</button>
              <button class="ghost" onclick={() => deleteInput(input.name)}>
                Delete
              </button>
            </div>
          </div>
        {/each}

        <form class="input-form" onsubmit={saveInput}>
          <h3>{editingName ? `Edit ${editingName}` : "Add input"}</h3>
          <label>
            <span>Name</span>
            <input bind:value={inputName} disabled={editingName !== null} />
          </label>
          <label>
            <span>URL</span>
            <input bind:value={inputUrl} placeholder="github:owner/repo" />
          </label>
          <label>
            <span>Description</span>
            <input bind:value={inputDescription} />
          </label>
          <div class="toolbar">
            <span class="spacer"></span>
            {#if editingName}
              <button type="button" class="ghost" onclick={newInput}>Cancel</button>
            {/if}
            <button type="submit" disabled={flakeBusy}>
              {flakeBusy ? "Saving..." : "Save"}
            </button>
          </div>
        </form>
      </Panel>
    {:else if view === "git"}
      <Panel
        title="Git"
        description="Commit locally or push to the configured upstream."
      >
        {#if gitStatus}
          <div class="git-summary">
            <span class="chip mono">{gitStatus.branch}</span>
            <span class="chip muted">{gitStatus.state}</span>
            <span class="chip muted">{gitStatus.dirty ? "dirty" : "clean"}</span>
            {#if gitStatus.upstream}
              <span class="chip muted mono">{gitStatus.upstream}</span>
            {/if}
            {#if gitStatus.ahead}
              <span class="chip">{gitStatus.ahead} ahead</span>
            {/if}
            {#if gitStatus.behind}
              <span class="chip">{gitStatus.behind} behind</span>
            {/if}
          </div>
        {:else}
          <p class="muted">No git repository information. Open a flake first.</p>
        {/if}
        <div class="toolbar">
          <button class="ghost" onclick={refreshGit} disabled={gitBusy}>
            Refresh
          </button>
          <button class="ghost" onclick={gitPull} disabled={gitBusy}>Pull</button>
          <button class="ghost" onclick={gitPush} disabled={gitBusy}>Push</button>
          <span class="spacer"></span>
          <button class="ghost" onclick={gitSync} disabled={gitBusy}>Sync</button>
        </div>
        <form
          class="input-form"
          onsubmit={(event) => {
            event.preventDefault();
            gitCommit();
          }}
        >
          <label>
            <span>Commit message</span>
            <input bind:value={commitMessage} placeholder="Update settings" />
          </label>
          <div class="toolbar">
            <span class="spacer"></span>
            <button type="submit" disabled={gitBusy || !gitStatus?.dirty}>
              Commit
            </button>
          </div>
        </form>
        <div class="toolbar">
          <button class="ghost" onclick={loadGitDiff} disabled={gitDiffBusy}>
            {gitDiffBusy ? "Loading..." : "Show diff"}
          </button>
        </div>
        {#if gitDiff}
          <pre class="diff">{gitDiff}</pre>
        {/if}
      </Panel>
    {:else}
      <Panel
        title={sortedMenus[selected]?.displayname ?? "Settings"}
        description={sortedMenus[selected]?.description}
      >
        <div class="toolbar">
          <label class="inline">
            <input type="checkbox" bind:checked={advanced} />
            <span>Advanced</span>
          </label>
          <span class="spacer"></span>
          <button class="ghost" onclick={reloadFromDisk}>Reload</button>
          <button class="ghost" onclick={openAddOption}>Add option</button>
          <button class="ghost" onclick={discard}>Discard</button>
          <button class="ghost" onclick={preview}>Diff</button>
          {#if rebuildRunning}
            <button class="ghost" onclick={cancelJob}>Cancel</button>
          {:else}
            <button class="ghost" onclick={saveAndRebuild} disabled={busy || dirty.size === 0}>
              Save &amp; rebuild
            </button>
          {/if}
          <button onclick={save} disabled={busy || dirty.size === 0}>
            {busy ? "Saving..." : "Save"}
          </button>
        </div>

        {#if visibleRecords.length === 0}
          <p class="muted">This menu has no controls.</p>
        {/if}

        {#each groupedRecords as group (group.name)}
          {#if group.name}
            <h3 class="group-heading">{group.name}</h3>
          {/if}
          {#each group.records as record (record.path)}
            <div
              class="control-record"
              class:highlight={highlightPath === record.path}
            >
              {#if !OJUNY_WIDGETS && OJUNY_WIDGET_KINDS.has(record.widget ?? "")}
                <p class="muted">
                  This Ojuny widget is not available in the minimal build.
                </p>
              {:else if record.children?.length && record.widget === "submodule"}
                <h3
                  class="submodule-heading"
                  style={`margin-left: ${(record.depth ?? 0) * 12}px`}
                >
                  {record.label ?? record.path}
                </h3>
              {:else if record.children?.length}
                <CardCollection
                  parent={record}
                  allRecords={records}
                  values={values}
                  descriptor={descriptor}
                  onchange={onChange}
                  isVisible={cardRecordVisible}
                  csrf={user.csrf}
                  expandDetails={expandDetailsDefault}
                  maxColumns={record.maxColumns ?? 1}
                  canAdd={canAddRow(record)}
                  canMove={canMoveRow}
                  canRename={canRenameRow}
                  canRemove={canRemoveRow}
                  onadd={() => openStructureAdd(record)}
                  onmove={moveStructureRow}
                  onrename={renameStructureRow}
                  onremove={removeStructureRow}
                  onpassword={setPassword}
                  onIconPick={pickIcon}
                  onIconPreview={previewIcon}
                  isCsv={isKeybindCsv}
                  rowTitle={cardRowTitle}
                  sortRows={cardRowSort}
                />
              {:else if record.widget === "secureboot"}
                <Control
                  descriptor={descriptor(record)}
                  value={values[record.path]}
                  onchange={onChange}
                  expandDetails={expandDetailsDefault}
                />
                <SecureBootPanel
                  csrf={user.csrf}
                  configured={Boolean(values[record.path])}
                  file={record.file}
                  origin={record.origin ?? ""}
                  {root}
                />
              {:else}
                <Control
                  descriptor={descriptor(record)}
                  value={values[record.path]}
                  onchange={onChange}
                  expandDetails={expandDetailsDefault}
                  onIconPick={(current) => pickIcon(record, current)}
                  onIconPreview={(value) => previewIcon(record, value)}
                  csvList={isKeybindCsv(record)}
                />
                {#if record.widget === "font"}
                  <p
                    class="font-preview"
                    style={`font-family: "${String(values[record.path] ?? "").replaceAll('"', "")}", sans-serif`}
                  >
                    The quick brown fox jumps over the lazy dog · 0123456789
                  </p>
                {/if}
                {#if record.widget === "icon-theme" || record.widget === "cursor-theme"}
                  {@const previews = styleEntriesFor(record)
                    .filter((entry) => stylePreview(entry))
                    .slice(0, 12)}
                  {#if previews.length}
                    <div class="style-previews">
                      {#each previews as entry (entry.value)}
                        <button
                          type="button"
                          class="style-preview"
                          class:selected={values[record.path] === entry.value}
                          title={entry.label}
                          onclick={() => onChange(record.path, entry.value)}
                        >
                          <img src={stylePreview(entry) ?? ""} alt={entry.label} />
                          <span>{entry.label}</span>
                        </button>
                      {/each}
                    </div>
                  {/if}
                {/if}
                {#if record.widget === "wallpaper" || record.widget === "file" || record.widget === "icon"}
                  <MediaPicker
                    {record}
                    csrf={user.csrf}
                    onchange={onChange}
                  />
                {/if}
                {#if record.path.endsWith(".hashedPassword")}
                  <div class="inline-actions">
                    <button class="ghost" onclick={() => setPassword(record)}>
                      Set password
                    </button>
                  </div>
                {/if}
                {#if dirty.has(record.path) && !record.readonly}
                  <div class="inline-actions">
                    <button class="ghost" onclick={() => resetOption(record)}>
                      Reset
                    </button>
                  </div>
                {/if}
                {#if canRenameRow(record) || canRemoveRow(record) || canMoveRow(record)}
                  <div class="inline-actions">
                    {#if canMoveRow(record)}
                      <button
                        class="ghost"
                        onclick={() => moveStructureRow(record, -1)}
                      >
                        Move up
                      </button>
                      <button
                        class="ghost"
                        onclick={() => moveStructureRow(record, 1)}
                      >
                        Move down
                      </button>
                    {/if}
                    {#if canRenameRow(record)}
                      <button
                        class="ghost"
                        onclick={() => renameStructureRow(record)}
                      >
                        Rename
                      </button>
                    {/if}
                    {#if canRemoveRow(record)}
                      <button
                        class="ghost"
                        onclick={() => removeStructureRow(record)}
                      >
                        Remove row
                      </button>
                    {/if}
                  </div>
                {/if}
              {/if}
            </div>
          {/each}
        {/each}

        {#if currentMenu?.inserts?.length}
          <div class="inserts">
            {#each currentMenu.inserts as marker (marker.offset)}
              {@const snippet = currentMenu.snippets?.find(
                (candidate) => candidate.name === marker.name,
              )}
              {#if snippet}
                <button
                  class="ghost"
                  onclick={() => openInsert(currentMenu.file, marker, snippet)}
                >
                  Add {marker.label ?? snippet.label}
                </button>
              {/if}
            {/each}
          </div>
        {/if}
      </Panel>

      {#if showAddFile}
        <Panel title="Add file" description="Create a module or render a template.">
          <form class="input-form" onsubmit={saveAddFile}>
            <label>
              <span>Kind</span>
              <select bind:value={addFileKind} onchange={onFileKindChange}>
                <option value="raw">Raw function</option>
                <option value="nixos">NixOS module</option>
                <option value="home">Home-manager module</option>
                {#each fileTemplates as template (template.index)}
                  <option value={`template:${template.index}`}>
                    Template: {template.label}
                  </option>
                {/each}
              </select>
            </label>
            <label>
              <span>File path</span>
              <input bind:value={addFilePath} />
            </label>
            <label>
              <span>Module name</span>
              <input bind:value={addFileModuleName} placeholder="flake.modules.<kind>.<name>" />
            </label>
            {#each Object.entries(addFilePlaceholders) as [name, value] (name)}
              <label>
                <span>{name}</span>
                <input bind:value={addFilePlaceholders[name]} />
              </label>
            {/each}
            <div class="toolbar">
              <span class="spacer"></span>
              <button type="button" class="ghost" onclick={() => (showAddFile = false)}>
                Cancel
              </button>
              <button type="submit" disabled={addFileBusy}>
                {addFileBusy ? "Creating..." : "Create"}
              </button>
            </div>
          </form>
        </Panel>
      {/if}

      {#if showAddOption}
        <Panel title="Add option" description="Insert an @option block into the current page.">
          <form class="input-form" onsubmit={saveAddOption}>
            <label>
              <span>Option path</span>
              <input bind:value={optionPath} placeholder="services.pipewire.enable" />
            </label>
            <label>
              <span>Widget</span>
              <select bind:value={optionWidget}>
                {#each optionWidgets as widget (widget)}
                  <option value={widget}>{widget}</option>
                {/each}
              </select>
            </label>
            <label>
              <span>Label</span>
              <input bind:value={optionLabel} placeholder="Optional label" />
            </label>
            <label>
              <span>Initial value</span>
              <input bind:value={optionValue} placeholder="true, text, or an expression" />
            </label>
            <div class="toolbar">
              <span class="spacer"></span>
              <button type="button" class="ghost" onclick={() => (showAddOption = false)}>
                Cancel
              </button>
              <button type="submit" disabled={optionBusy}>
                {optionBusy ? "Adding..." : "Add option"}
              </button>
            </div>
          </form>
        </Panel>
      {/if}

      {#if structureTarget}
        <Panel
          title={`Add row to ${structureTarget.label ?? structureTarget.path}`}
          description="The row is written immediately; staged edits must be saved or discarded first."
        >
          <form class="input-form" onsubmit={addStructureRow}>
            <label>
              <span>
                {structureTarget.widget === "users"
                  ? "User name"
                  : structureTarget.widget === "monitors"
                    ? "Output name"
                    : structureTarget.widget === "keybinds"
                      ? "Bind name"
                      : "Row key"}
              </span>
              <input bind:value={structureKey} />
            </label>
            {#if structureTarget.widget === "grid-configuration" && structureTarget.gridRegistry?.length}
              <div class="toolbar">
                {#each structureTarget.gridRegistry as card (card)}
                  <button
                    type="button"
                    class="ghost"
                    class:active={structureKey === card}
                    onclick={() => (structureKey = card)}
                  >
                    {card}
                  </button>
                {/each}
              </div>
            {/if}
            {#if structureTarget.widget === "monitors"}
              {#if detectedOutputs.length > 0}
                <div class="toolbar">
                  {#each detectedOutputs as output (output.name)}
                    <button
                      type="button"
                      class="ghost"
                      class:active={output.name === structureKey}
                      onclick={() => chooseOutput(output)}
                    >
                      {output.name}
                      {output.width ? `${output.width}x${output.height}` : ""}
                    </button>
                  {/each}
                </div>
              {:else}
                <p class="muted small">
                  No compositor outputs detected. Enter the output name manually.
                </p>
              {/if}
              {#if structureMode}
                <div class="toolbar">
                  <label class="inline">
                    <span>Width</span>
                    <input
                      type="number"
                      bind:value={structureMode.width}
                    />
                  </label>
                  <label class="inline">
                    <span>Height</span>
                    <input
                      type="number"
                      bind:value={structureMode.height}
                    />
                  </label>
                  <label class="inline">
                    <span>Refresh</span>
                    <input
                      type="number"
                      bind:value={structureMode.refresh}
                    />
                  </label>
                </div>
              {/if}
            {/if}
            <div class="toolbar">
              <span class="spacer"></span>
              <button
                type="button"
                class="ghost"
                onclick={() => (structureTarget = null)}
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={structureBusy || !structureKey.trim()}
              >
                {structureBusy ? "Adding..." : "Add row"}
              </button>
            </div>
          </form>
        </Panel>
      {/if}

      {#if showConfig && config}
        <Panel title="Configure" description="The same config file as the desktop settings app.">
          <form class="input-form" onsubmit={saveConfig}>
            <label>
              <span>Flake path</span>
              <input
                bind:value={configFlakePath}
                list="config-recent-flakes"
                placeholder="/home/den/repos/nixos-configs/dendrite"
              />
              <datalist id="config-recent-flakes">
                {#each config.recent_flake_paths as recent (recent)}
                  <option value={recent}></option>
                {/each}
              </datalist>
            </label>
            <label>
              <span>Rebuild command</span>
              <input bind:value={rebuildCommand} placeholder="sudo nixos-rebuild switch --flake .#host" />
            </label>
            <label class="inline">
              <input type="checkbox" bind:checked={config.live_apply} />
              <span>Apply changes immediately</span>
            </label>
            <label class="inline">
              <input type="checkbox" bind:checked={config.auto_git_sync} />
              <span>Sync git on open and save</span>
            </label>
            <label class="inline">
              <input type="checkbox" bind:checked={config.expand_details_by_default} />
              <span>Expand setting details</span>
            </label>
            <label class="inline">
              <input type="checkbox" bind:checked={config.reload_mango_on_rebuild} />
              <span>Reload mango after rebuild</span>
            </label>
            <label class="inline">
              <input type="checkbox" bind:checked={config.reload_dms_on_rebuild} />
              <span>Reload DMS after rebuild</span>
            </label>
            <div class="toolbar">
              <span class="spacer"></span>
              <button type="button" class="ghost" onclick={() => (showConfig = false)}>
                Cancel
              </button>
              <button type="submit" disabled={configBusy}>
                {configBusy ? "Saving..." : "Save config"}
              </button>
            </div>
          </form>
        </Panel>
      {/if}

      {#if insert}
        <Panel
          title={`Add ${insert.marker.label ?? insert.snippet.label}`}
          description="The snippet is inserted above its @insert marker."
        >
          <form class="insert-form" onsubmit={submitInsert}>
            {#each insert.snippet.placeholders as placeholder (placeholder.name)}
              <label>
                <span>{placeholder.label}</span>
                <input
                  bind:value={insertValues[placeholder.name]}
                  placeholder={placeholder.hint}
                />
              </label>
            {/each}
            <div class="toolbar">
              <span class="spacer"></span>
              <button type="button" class="ghost" onclick={() => (insert = null)}>
                Cancel
              </button>
              <button type="submit" disabled={insertBusy}>
                {insertBusy ? "Inserting..." : "Insert"}
              </button>
            </div>
          </form>
        </Panel>
      {/if}

      {#if showDiff}
        <Panel title="Pending changes" description="Nothing is written until Save.">
          <pre class="diff">{diff}</pre>
        </Panel>
      {/if}

      {#if rebuildOutput}
        <Panel title="Rebuild output" description="Captured when the command finished.">
          <pre class="diff">{rebuildOutput}</pre>
        </Panel>
      {/if}
    {/if}

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
  </main>

  {#if !popoutMode}
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
  {/if}
</div>
