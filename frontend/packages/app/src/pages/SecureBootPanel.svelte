<script lang="ts">
  import { onMount } from "svelte";
  import { Panel } from "@nix-web-gui/ui";
  import { rpc, type SecureBootStatus } from "../api";

  export let csrf: string;
  export let configured: boolean;
  export let file: string;
  export let origin: string;
  export let root: string;

  let status: SecureBootStatus | null = null;
  let busy = false;
  let actionBusy = false;
  let error = "";
  let output: string[] = [];
  let keyAction = "keep";
  let microsoft = true;
  let firmwareBuiltin = false;
  let configBlock = "";

  onMount(load);

  async function load() {
    busy = true;
    error = "";
    try {
      status = await rpc<SecureBootStatus>(
        "secureboot.status",
        { configured },
        csrf,
      );
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot read Secure Boot";
    } finally {
      busy = false;
    }
  }

  async function prepare() {
    if (
      keyAction === "create" &&
      !confirm("Generate a new key bundle in /var/lib/sbctl?")
    ) {
      return;
    }
    actionBusy = true;
    error = "";
    try {
      const result = await rpc<{ output: string[] }>(
        "secureboot.setup",
        { keyAction, microsoft, firmwareBuiltin },
        csrf,
      );
      output = result.output;
      await load();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot prepare keys";
    } finally {
      actionBusy = false;
    }
  }

  async function writeConfig() {
    actionBusy = true;
    error = "";
    try {
      const result = await rpc<{ written: string[] }>(
        "secureboot.writeConfig",
        { file, origin },
        csrf,
      );
      output = result.written.length
        ? result.written.map((line) => `wrote: ${line}`)
        : ["The module already declares every Lanzaboote setting."];
      await load();
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Cannot write the configuration";
    } finally {
      actionBusy = false;
    }
  }

  async function rebuild() {
    if (!confirm("Run the configured rebuild command now?")) return;
    actionBusy = true;
    error = "";
    try {
      const result = await rpc<{ output: string }>(
        "secureboot.rebuild",
        { root },
        csrf,
      );
      output = [result.output];
      await load();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Rebuild failed";
    } finally {
      actionBusy = false;
    }
  }

  async function reboot() {
    if (!confirm("Reboot the machine now?")) return;
    actionBusy = true;
    error = "";
    try {
      await rpc("secureboot.reboot", { confirm: true }, csrf);
      output = ["Reboot requested."];
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Reboot failed";
    } finally {
      actionBusy = false;
    }
  }

  async function showConfig() {
    try {
      const result = await rpc<{ block: string }>("secureboot.config", {}, csrf);
      configBlock = result.block;
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Cannot read the config block";
    }
  }

  function yesNo(value: boolean | null): string {
    if (value === null) return "unknown";
    return value ? "yes" : "no";
  }

  function bytes(value: number | null): string {
    if (value === null) return "unknown";
    const units = ["B", "KiB", "MiB", "GiB"];
    let size = value;
    let unit = 0;
    while (size >= 1024 && unit < units.length - 1) {
      size /= 1024;
      unit += 1;
    }
    return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
  }
</script>

<Panel title="Secure Boot" description={status?.stage.detail ?? "Live firmware and key state."}>
  <div class="toolbar">
    <span class="spacer"></span>
    <button class="ghost" onclick={load} disabled={busy}>
      {busy ? "Reading..." : "Refresh"}
    </button>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if status}
    <h3>{status.stage.title}</h3>
    <dl class="facts">
      <div><dt>Firmware mode</dt><dd>{status.state.firmware_mode}</dd></div>
      <div><dt>UEFI</dt><dd>{yesNo(status.state.efi)}</dd></div>
      <div><dt>Setup mode</dt><dd>{yesNo(status.state.setup_mode_enabled)}</dd></div>
      <div><dt>sbctl</dt><dd>{yesNo(status.state.sbctl_installed)}</dd></div>
      <div><dt>Key bundle</dt><dd class="mono">{status.state.key_bundle}</dd></div>
      <div><dt>Keys present</dt><dd>{yesNo(status.state.keys_present)}</dd></div>
      <div><dt>Keys enrolled</dt><dd>{yesNo(status.state.keys_enrolled)}</dd></div>
      <div><dt>Lanzaboote active</dt><dd>{yesNo(status.state.lanzaboote_active)}</dd></div>
      <div><dt>Signed boot files</dt><dd>{status.state.signed_files}</dd></div>
      <div><dt>Unsigned boot files</dt><dd>{status.state.unsigned_files}</dd></div>
      <div><dt>Windows</dt><dd>{yesNo(status.state.windows_present)}</dd></div>
      <div><dt>BitLocker</dt><dd>{yesNo(status.state.bitlocker_present)}</dd></div>
      <div><dt>ESP free</dt><dd>{bytes(status.state.esp_free_bytes)}</dd></div>
    </dl>
    {#if status.state.errors.length}
      <h3>Warnings</h3>
      <ul>
        {#each status.state.errors as message (message)}
          <li class="error">{message}</li>
        {/each}
      </ul>
    {/if}

    <h3>Setup options</h3>
    <div class="input-form">
      <label>
        <span>Keys</span>
        <select bind:value={keyAction}>
          <option value="keep">Keep existing keys</option>
          <option value="create">Generate new keys</option>
          <option value="migrate">Migrate /etc/secureboot</option>
        </select>
      </label>
      <label class="inline">
        <input type="checkbox" bind:checked={microsoft} />
        <span>Include Microsoft keys</span>
      </label>
      <label class="inline">
        <input type="checkbox" bind:checked={firmwareBuiltin} />
        <span>Include firmware builtin keys</span>
      </label>
    </div>

    <div class="toolbar">
      <button class="ghost" onclick={showConfig}>Show config block</button>
      <button class="ghost" onclick={prepare} disabled={actionBusy}>
        Prepare or enroll keys
      </button>
      <button class="ghost" onclick={writeConfig} disabled={actionBusy}>
        Write configuration
      </button>
      <button class="ghost" onclick={rebuild} disabled={actionBusy}>
        Rebuild
      </button>
      <span class="spacer"></span>
      <button class="ghost" onclick={reboot} disabled={actionBusy}>
        Reboot
      </button>
    </div>

    {#if configBlock}
      <pre class="diff">{configBlock}</pre>
    {/if}
    {#if output.length}
      <pre class="diff">{output.join("\n")}</pre>
    {/if}

    <p class="muted small">
      Firmware steps stay manual: reset to Setup Mode before enrolling, then
      enable Secure Boot without resetting the keys. Key creation, enrollment,
      the Lanzaboote configuration, and the rebuild run from the root service
      account.
    </p>
  {/if}
</Panel>
