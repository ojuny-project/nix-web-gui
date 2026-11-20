<script lang="ts">
  import { onMount } from "svelte";
  import { rpc, type SecureBootState, type SecureBootStatus } from "../../api";

  export let csrf: string;
  export let configured: boolean;
  export let file: string;
  export let origin: string;
  export let root: string;

  type Stage =
    | "not-uefi"
    | "keys"
    | "config"
    | "rebuild"
    | "clear-keys"
    | "enroll"
    | "enable-secureboot"
    | "verify"
    | "done";

  type FirmwareMode = "unknown" | "disabled" | "setup" | "user";

  type Action = {
    label: string;
    hint: string;
    needsPassword: boolean;
    disabled: boolean;
    run: () => void;
  };

  const STAGES: Array<{ id: Stage; title: string }> = [
    { id: "keys", title: "Prepare keys" },
    { id: "config", title: "Write configuration" },
    { id: "rebuild", title: "Rebuild NixOS" },
    { id: "clear-keys", title: "Clear Secure Boot keys in firmware" },
    { id: "enroll", title: "Enroll keys" },
    { id: "enable-secureboot", title: "Enable Secure Boot in firmware" },
    { id: "verify", title: "Verify boot files" },
    { id: "done", title: "Secure Boot is active" },
  ];

  const KEY_ACTIONS: Array<{ value: string; label: string }> = [
    { value: "keep", label: "Keep existing keys" },
    { value: "create", label: "Generate new keys" },
    { value: "migrate", label: "Migrate /etc/secureboot" },
  ];

  let status: SecureBootStatus | null = null;
  let busy = "";
  let error = "";
  let note = "";
  let output: string[] = [];
  let configBlock = "";
  let password = "";
  let keyAction = "keep";
  let microsoft = true;
  let firmwareBuiltin = false;
  let configWritten = false;
  let mounted = false;
  let lastConfigured = configured;

  $: state = status?.state ?? null;
  $: stage = state ? stageOf(state, configured) : null;
  $: position = stage ? stageOrder(stage) : -1;
  $: warnings = state ? warningsFor(state, configured) : [];
  $: action = stage ? nextAction(stage) : null;
  $: if (state) adjustKeyAction(state);
  $: if (mounted && lastConfigured !== configured) {
    lastConfigured = configured;
    void load();
  }

  onMount(() => {
    mounted = true;
    lastConfigured = configured;
    void load();
    window.addEventListener("nix-web-gui:progress", onProgress);
    return () => {
      mounted = false;
      window.removeEventListener("nix-web-gui:progress", onProgress);
    };
  });

  function onProgress(event: Event) {
    const detail = (event as CustomEvent).detail as {
      params?: { job?: string; line?: string };
    };
    if (detail?.params?.job !== "secureboot-rebuild") return;
    output = [...output, detail.params.line ?? ""].slice(-500);
  }

  function stageOf(current: SecureBootState, isConfigured: boolean): Stage {
    if (!current.efi) return "not-uefi";
    if (!current.keys_present) return "keys";
    if (!isConfigured) return "config";
    if (!current.lanzaboote_active) return "rebuild";
    if (setupMode(current)) return "enroll";
    if (secureBootEnabled(current)) {
      return current.unsigned_files > 0 ? "verify" : "done";
    }
    if (current.keys_enrolled === true) return "enable-secureboot";
    return "clear-keys";
  }

  function stageOrder(current: Stage): number {
    return STAGES.findIndex((item) => item.id === current);
  }

  function firmwareMode(current: SecureBootState): FirmwareMode {
    const raw = (current.firmware_mode ?? "").toLowerCase();
    if (raw.includes("setup")) return "setup";
    if (raw.includes("enabled")) return "user";
    if (raw.includes("disabled")) return "disabled";
    return "unknown";
  }

  function setupMode(current: SecureBootState): boolean {
    return (
      current.setup_mode_enabled ?? firmwareMode(current) === "setup"
    );
  }

  function secureBootEnabled(current: SecureBootState): boolean {
    return firmwareMode(current) === "user";
  }

  function firmwareModeLabel(current: SecureBootState): string {
    switch (firmwareMode(current)) {
      case "setup":
        return "enabled (setup mode)";
      case "user":
        return "enabled (user mode)";
      case "disabled":
        return "disabled";
      default:
        return "unknown";
    }
  }

  function badge(current: SecureBootState, isConfigured: boolean): string {
    if (!current.efi) return "UEFI required";
    if (!isConfigured) return "Disabled in the configuration";
    if (secureBootEnabled(current) && current.lanzaboote_active) {
      return current.unsigned_files > 0
        ? "Enforced, unsigned files found"
        : "Enabled and enforced";
    }
    if (current.lanzaboote_active) {
      return setupMode(current)
        ? "Ready to enroll keys"
        : "Installed, Secure Boot disabled in firmware";
    }
    return setupMode(current)
      ? "Setup mode, rebuild required"
      : "Configured, rebuild required";
  }

  function warningsFor(
    current: SecureBootState,
    isConfigured: boolean,
  ): string[] {
    const messages: string[] = [];
    if (isConfigured && !current.keys_present) {
      messages.push(
        "No Secure Boot key bundle exists. Open Set up Secure Boot and create keys before saving or rebuilding.",
      );
    }
    if (
      isConfigured &&
      current.keys_present &&
      current.keys_enrolled !== true &&
      !setupMode(current) &&
      !secureBootEnabled(current)
    ) {
      messages.push(
        "The firmware does not have this key bundle enrolled. Reset it to Setup Mode and enroll the current bundle.",
      );
    }
    if (isConfigured && setupMode(current)) {
      messages.push(
        "The firmware is in Setup Mode. Enroll the current keys, then enable Secure Boot.",
      );
    }
    if (isConfigured && secureBootEnabled(current) && !current.lanzaboote_active) {
      messages.push(
        "Secure Boot is enforced but Lanzaboote is not installed yet. Do not reboot before the rebuild completes.",
      );
    }
    if (!isConfigured && secureBootEnabled(current)) {
      messages.push(
        "Secure Boot is enforced. Disable it in the firmware before turning Lanzaboote off.",
      );
    }
    if (
      current.windows_present &&
      !current.vendor_keys.includes("microsoft")
    ) {
      messages.push(
        "Windows is installed but the Microsoft keys are not enrolled. Enroll them to keep Windows bootable.",
      );
    }
    if (current.bitlocker_present) {
      messages.push(
        "BitLocker is present. Confirm the recovery key in Windows before changing Secure Boot keys.",
      );
    }
    return messages;
  }

  function keyActionLabel(): string {
    return (
      KEY_ACTIONS.find((item) => item.value === keyAction)?.label ??
      "Keep existing keys"
    );
  }

  function keyBundleLabel(current: SecureBootState): string {
    let text = current.keys_present
      ? `present in ${current.key_bundle}`
      : `no bundle in ${current.key_bundle}`;
    if (current.owner_guid) {
      text += ` · owner ${current.owner_guid.slice(0, 8)}`;
    }
    return text;
  }

  function enrollmentLabel(current: SecureBootState): string {
    if (current.keys_enrolled === true) return "enrolled from this bundle";
    if (current.keys_enrolled === false) return "not enrolled from this bundle";
    return "unknown; enroll to record it";
  }

  function humanBytes(bytes: number | null): string {
    if (bytes === null) return "unknown";
    const units = ["B", "KiB", "MiB", "GiB"];
    let size = bytes;
    let unit = 0;
    while (size >= 1024 && unit < units.length - 1) {
      size /= 1024;
      unit += 1;
    }
    return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
  }

  function nextAction(current: Stage): Action {
    switch (current) {
      case "not-uefi":
        return {
          label: "UEFI required",
          hint: "Secure Boot needs UEFI. Switch the firmware from legacy boot first.",
          needsPassword: false,
          disabled: true,
          run: () => {},
        };
      case "keys":
        return {
          label: keyActionLabel(),
          hint: "Create a key bundle in /var/lib/sbctl, or migrate /etc/secureboot. Existing keys are never overwritten.",
          needsPassword: true,
          disabled: false,
          run: () => void prepareKeys(),
        };
      case "config":
        return {
          label: "Write configuration",
          hint: "Add the Lanzaboote input and the bootloader assignments to the module that owns the toggle. Then enable the Lanzaboote switch above and save the page.",
          needsPassword: false,
          disabled: configWritten,
          run: () => void writeConfiguration(),
        };
      case "rebuild":
        return {
          label: "Rebuild now",
          hint: "Apply the configuration so Lanzaboote installs signed boot files. For Secure Boot, keep the rebuild command on nixos-rebuild boot if you do not want the running system switched before enrollment.",
          needsPassword: true,
          disabled: false,
          run: () => void rebuildNow(),
        };
      case "clear-keys":
        return {
          label: "Reboot to firmware",
          hint: "The firmware holds a different key bundle. Reboot, reset it to Setup Mode yourself, then return here.",
          needsPassword: true,
          disabled: false,
          run: () => void rebootToFirmware(),
        };
      case "enroll":
        return {
          label: "Enroll keys",
          hint: "Setup Mode is enabled. Enroll the local bundle. The enrollment marker is written only after sbctl reports success.",
          needsPassword: true,
          disabled: false,
          run: () => void prepareKeys(),
        };
      case "enable-secureboot":
        return {
          label: "Reboot to firmware",
          hint: "The keys are enrolled. Reboot and enable Secure Boot yourself. Do not reset to Setup Mode, that clears the keys.",
          needsPassword: true,
          disabled: false,
          run: () => void rebootToFirmware(),
        };
      case "verify":
        return {
          label: "Rebuild and re-check",
          hint: "Some boot files are still unsigned. Rebuild, reboot, then refresh this state.",
          needsPassword: true,
          disabled: false,
          run: () => void rebuildNow(),
        };
      case "done":
        return {
          label: "",
          hint: "The firmware enforces Secure Boot and Lanzaboote signed the boot files.",
          needsPassword: false,
          disabled: true,
          run: () => {},
        };
    }
  }

  function adjustKeyAction(current: SecureBootState) {
    if (!current.keys_present && keyAction === "keep") keyAction = "create";
    if (current.keys_present && keyAction === "create") keyAction = "keep";
  }

  function validationError(current: SecureBootState): string {
    if (!current.efi) {
      return "This machine is not booted in UEFI mode";
    }
    if (keyAction === "create" && current.keys_present) {
      return `Keys already exist in ${current.key_bundle}. Back them up and remove them first, or keep them.`;
    }
    if (!microsoft && current.vendor_keys.length > 0) {
      return "Without the Microsoft keys, Windows and some option ROMs may stop booting";
    }
    return "";
  }

  function requirePassword(): boolean {
    if (password.trim() !== "") return true;
    error =
      "Enter the device password. Secure Boot setup, rebuild, and reboot re-authenticate through PAM.";
    return false;
  }

  function describeError(cause: unknown, fallback: string): string {
    return cause instanceof Error ? cause.message : fallback;
  }

  async function load() {
    busy = "status";
    error = "";
    try {
      status = await rpc<SecureBootStatus>(
        "secureboot.status",
        { configured },
        csrf,
      );
    } catch (cause) {
      error = describeError(cause, "Cannot read Secure Boot state");
    } finally {
      busy = "";
    }
  }

  async function showConfigBlock() {
    error = "";
    try {
      const result = await rpc<{ block: string }>(
        "secureboot.config",
        {},
        csrf,
      );
      configBlock = result.block;
    } catch (cause) {
      error = describeError(cause, "Cannot read the config block");
    }
  }

  async function prepareKeys() {
    if (!state) return;
    const invalid = validationError(state);
    if (invalid) {
      error = invalid;
      return;
    }
    if (
      keyAction === "create" &&
      !confirm("Generate a new key bundle in /var/lib/sbctl?")
    ) {
      return;
    }
    if (!requirePassword()) return;
    busy = "setup";
    error = "";
    note = "";
    try {
      const result = await rpc<{
        output: string[];
        stage: { title: string };
      }>(
        "secureboot.setup",
        { keyAction, microsoft, firmwareBuiltin, password },
        csrf,
      );
      output = result.output.slice(-500);
      note = `Next: ${result.stage.title}`;
      await load();
    } catch (cause) {
      error = describeError(cause, "Cannot prepare or enroll keys");
    } finally {
      busy = "";
    }
  }

  async function writeConfiguration() {
    busy = "config";
    error = "";
    note = "";
    try {
      const result = await rpc<{ written: string[] }>(
        "secureboot.writeConfig",
        { file, origin },
        csrf,
      );
      configWritten = true;
      output = result.written.length
        ? result.written.map((line) => `wrote: ${line}`)
        : ["The module already declares every Lanzaboote setting."];
      note =
        "Enable the Lanzaboote switch above and save the page. The switch writes boot.lanzaboote.enable, which this step does not touch.";
      await load();
    } catch (cause) {
      error = describeError(cause, "Cannot write the configuration");
    } finally {
      busy = "";
    }
  }

  async function rebuildNow() {
    if (!requirePassword()) return;
    if (
      !confirm(
        "Run the configured rebuild command now? It runs as the web service account.",
      )
    ) {
      return;
    }
    busy = "rebuild";
    error = "";
    note = "";
    output = [];
    try {
      const result = await rpc<{ output: string }>(
        "secureboot.rebuild",
        { root, password },
        csrf,
      );
      if (result.output.trim()) {
        output = result.output.split("\n").slice(-500);
      }
      note = "Rebuild finished. Reboot before checking Secure Boot enforcement.";
      await load();
    } catch (cause) {
      error = describeError(cause, "Rebuild failed");
    } finally {
      busy = "";
    }
  }

  async function rebootToFirmware() {
    if (!requirePassword()) return;
    if (
      !confirm(
        "Reboot now? This button only reboots. Firmware settings stay manual.",
      )
    ) {
      return;
    }
    busy = "reboot";
    error = "";
    note = "";
    try {
      await rpc("secureboot.reboot", { confirm: true, password }, csrf);
      note =
        "Reboot requested. Open the firmware setup menu from the boot screen, change Secure Boot there yourself, then boot back into NixOS.";
    } catch (cause) {
      error = describeError(cause, "Reboot failed");
    } finally {
      busy = "";
    }
  }

  function busyLabel(): string {
    switch (busy) {
      case "setup":
        return "Running...";
      case "config":
        return "Writing...";
      case "rebuild":
        return "Rebuilding...";
      case "reboot":
        return "Rebooting...";
      case "status":
        return "Reading...";
      default:
        return "";
    }
  }
</script>

<section class="secureboot-flow">
  <header class="flow-head">
    <h3>{stage ? STAGES.find((item) => item.id === stage)?.title : "Secure Boot"}</h3>
    <p class="muted">
      {status?.stage.detail ??
        "Reading firmware, keys, and boot loader state."}
    </p>
    {#if state}
      <p class="flow-badge">{badge(state, configured)}</p>
    {/if}
  </header>

  {#if stage === "not-uefi"}
    <p class="flow-blocked">
      Secure Boot is unavailable on a legacy BIOS boot. Reinstall or switch the
      firmware to UEFI with systemd-boot before using this flow.
    </p>
  {/if}

  <ol class="flow-stages">
    {#each STAGES as item, index (item.id)}
      <li
        class:current={item.id === stage}
        class:done={position >= 0 && index < position}
      >
        <span class="flow-mark">
          {position >= 0 && index < position
            ? "ok"
            : item.id === stage
              ? "next"
              : "todo"}
        </span>
        <span>{item.title}</span>
      </li>
    {/each}
  </ol>

  {#if state}
    <dl class="flow-facts">
      <div>
        <dt>Firmware</dt>
        <dd>
          {state.efi ? "UEFI" : "legacy BIOS"}{state.firmware
            ? ` · ${state.firmware}`
            : ""}
        </dd>
      </div>
      <div>
        <dt>Secure Boot</dt>
        <dd>{firmwareModeLabel(state)}</dd>
      </div>
      <div>
        <dt>Setup mode</dt>
        <dd>{setupMode(state) ? "yes" : "no"}</dd>
      </div>
      <div>
        <dt>Key bundle</dt>
        <dd class="mono">{keyBundleLabel(state)}</dd>
      </div>
      <div>
        <dt>Enrollment</dt>
        <dd>{enrollmentLabel(state)}</dd>
      </div>
      <div>
        <dt>Boot loader</dt>
        <dd>
          {state.lanzaboote_active
            ? `Lanzaboote · ${state.signed_files} signed, ${state.unsigned_files} unsigned`
            : "Lanzaboote not detected in the running system"}
        </dd>
      </div>
      <div>
        <dt>EFI system partition</dt>
        <dd class="mono">
          {state.esp_path ?? "/boot"} · {humanBytes(state.esp_free_bytes)} free
        </dd>
      </div>
      {#if state.windows_present}
        <div>
          <dt>Windows</dt>
          <dd>boot manager found on the ESP</dd>
        </div>
      {/if}
      {#if state.bitlocker_present}
        <div>
          <dt>BitLocker</dt>
          <dd>volume found, save the recovery key first</dd>
        </div>
      {/if}
    </dl>
  {/if}

  {#if warnings.length > 0 || (state?.errors.length ?? 0) > 0}
    <ul class="flow-warnings">
      {#each warnings as message, index (index)}
        <li>{message}</li>
      {/each}
      {#each state?.errors ?? [] as message, index (index)}
        <li>{message}</li>
      {/each}
    </ul>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}
  {#if note}
    <p class="flow-note">{note}</p>
  {/if}

  {#if action}
    <div class="flow-action">
      <h4>Next action</h4>
      <p class="muted small">{action.hint}</p>
      <div class="flow-controls">
        {#if action.needsPassword}
          <label class="flow-password">
            <span>Device password</span>
            <input
              type="password"
              bind:value={password}
              autocomplete="current-password"
              placeholder="Required for setup, rebuild, and reboot"
            />
          </label>
        {/if}
        {#if action.label}
          <button
            type="button"
            onclick={action.run}
            disabled={action.disabled || busy !== ""}
          >
            {busy && busy !== "status" ? busyLabel() : action.label}
          </button>
        {/if}
        <button
          type="button"
          class="ghost"
          onclick={load}
          disabled={busy !== ""}
        >
          {busy === "status" ? "Reading..." : "Refresh state"}
        </button>
        <button type="button" class="ghost" onclick={showConfigBlock}>
          Show config block
        </button>
      </div>
    </div>
  {/if}

  {#if stage === "keys" || stage === "enroll"}
    <details class="flow-options" open={stage === "keys"}>
      <summary>Setup options</summary>
      <label class="flow-field">
        <span>Keys</span>
        <select bind:value={keyAction}>
          {#each KEY_ACTIONS as option (option.value)}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>
      <label class="flow-field inline">
        <input type="checkbox" bind:checked={microsoft} />
        <span>Include Microsoft keys, keeps Windows and option ROMs bootable</span>
      </label>
      <label class="flow-field inline">
        <input type="checkbox" bind:checked={firmwareBuiltin} />
        <span>Include firmware built-in keys, needed on Framework and similar</span>
      </label>
      <p class="muted small">
        Enrollment runs only when the firmware reports Setup Mode. The
        enrollment marker is written after sbctl succeeds, so a failed run does
        not advance the flow.
      </p>
    </details>
  {/if}

  {#if stage === "clear-keys"}
    <div class="flow-firmware">
      <h4>Firmware: reset to Setup Mode</h4>
      <ol>
        <li>Reboot and open the firmware settings from the boot menu.</li>
        <li>Open Security, then Secure Boot, then Key Management.</li>
        <li>Select Reset to Setup Mode, or delete the Platform Key.</li>
        <li>
          Do not select Clear All Secure Boot Keys when a reset option exists.
          It can drop the Forbidden Signature Database (dbx).
        </li>
        <li>Save and exit, then boot back into NixOS.</li>
      </ol>
      <p class="muted small">
        ThinkPad: Security, Secure Boot, Reset to Setup Mode. Framework:
        Administer Secure Boot, delete the PK, KEK and DB entries, do not use
        Erase all Secure Boot Settings. Other boards: erase the existing
        Platform Key.
      </p>
    </div>
  {:else if stage === "enable-secureboot"}
    <div class="flow-firmware">
      <h4>Firmware: enable Secure Boot</h4>
      <ol>
        <li>Reboot and open the firmware settings from the boot menu.</li>
        <li>Open Security, then Secure Boot.</li>
        <li>Enable Secure Boot.</li>
        <li>
          Do not reset to Setup Mode, that clears the keys you just enrolled.
        </li>
        <li>Save and exit, then boot back into NixOS.</li>
      </ol>
      <p class="muted small">
        On Framework, also enable Enforce Secure Boot in Administer Secure Boot.
        These steps are manual. This page cannot change firmware settings.
      </p>
    </div>
  {:else if stage === "enroll"}
    <div class="flow-firmware">
      <h4>Firmware: enrollment happens from NixOS</h4>
      <p class="muted small">
        The firmware already reports Setup Mode. Keep it there until enrollment
        reports success, then use the next stage to enable Secure Boot.
      </p>
    </div>
  {:else if stage === "verify"}
    <div class="flow-firmware">
      <h4>Unsigned boot files</h4>
      <p class="muted small">
        Kernel and initrd copies under /boot/EFI/nixos are expected unsigned.
        Lanzaboote embeds their hash in the signed stub. Anything else reported
        unsigned needs another rebuild.
      </p>
    </div>
  {/if}

  {#if configBlock}
    <pre class="flow-log">{configBlock}</pre>
  {/if}
  {#if output.length > 0}
    <pre class="flow-log">{output.join("\n")}</pre>
  {/if}

  <p class="muted small">
    Firmware steps stay manual: reset to Setup Mode before enrolling, then
    enable Secure Boot without resetting the keys. Key creation, enrollment, the
    Lanzaboote configuration, and the rebuild run from the web service account
    and require the device password again.
  </p>
</section>

<style>
  .secureboot-flow {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 12px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
  }

  .flow-head h3 {
    margin: 0 0 4px;
  }

  .flow-head p {
    margin: 0;
  }

  .flow-badge {
    display: inline-block;
    margin-top: 8px;
    padding: 2px 8px;
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--accent);
    font-size: 0.85rem;
  }

  .flow-blocked {
    margin: 0;
    padding: 8px 12px;
    border-left: 3px solid var(--danger);
    background: var(--surface-2);
  }

  .flow-stages {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr));
    gap: 4px 12px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .flow-stages li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 0;
    color: var(--muted);
    font-size: 0.9rem;
  }

  .flow-stages li.current {
    color: var(--text-strong);
    font-weight: 600;
  }

  .flow-stages li.done {
    color: var(--text);
  }

  .flow-mark {
    display: inline-block;
    min-width: 3.2rem;
    padding: 0 6px;
    border-radius: 6px;
    background: var(--surface-2);
    color: var(--muted);
    font-size: 0.7rem;
    letter-spacing: 0.04em;
    text-align: center;
    text-transform: uppercase;
  }

  .flow-stages li.current .flow-mark {
    background: color-mix(in srgb, var(--accent) 25%, transparent);
    color: var(--accent);
  }

  .flow-stages li.done .flow-mark {
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    color: var(--text);
  }

  .flow-facts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
    gap: 4px 16px;
    margin: 0;
  }

  .flow-facts div {
    display: flex;
    gap: 8px;
    padding: 2px 0;
    border-bottom: 1px dashed var(--border);
  }

  .flow-facts dt {
    flex: 0 0 9rem;
    color: var(--muted);
    font-size: 0.9rem;
  }

  .flow-facts dd {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
    font-size: 0.9rem;
  }

  .flow-warnings {
    margin: 0;
    padding: 8px 12px 8px 28px;
    border-left: 3px solid var(--danger);
    background: var(--surface-2);
    font-size: 0.9rem;
  }

  .flow-note {
    margin: 0;
    padding: 8px 12px;
    border-left: 3px solid var(--accent);
    background: var(--surface-2);
    font-size: 0.9rem;
  }

  .flow-action {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
  }

  .flow-action h4 {
    margin: 0;
  }

  .flow-controls {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 8px;
  }

  .flow-password {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.85rem;
  }

  .flow-password input {
    min-width: 16rem;
  }

  .flow-options {
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
  }

  .flow-options summary {
    cursor: pointer;
    color: var(--text-strong);
  }

  .flow-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 8px;
    font-size: 0.9rem;
  }

  .flow-field.inline {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }

  .flow-firmware {
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
  }

  .flow-firmware h4 {
    margin: 0 0 6px;
  }

  .flow-firmware ol {
    margin: 0;
    padding-left: 20px;
    font-size: 0.9rem;
  }

  .flow-firmware p {
    margin: 6px 0 0;
  }

  .flow-log {
    max-height: 22rem;
    overflow: auto;
    margin: 0;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
    font-family: ui-monospace, "JetBrains Mono", monospace;
    font-size: 0.8rem;
    white-space: pre-wrap;
  }
</style>
