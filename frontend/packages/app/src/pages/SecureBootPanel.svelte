<script lang="ts">
  import { onMount } from "svelte";
  import { Panel } from "@nix-web-gui/ui";
  import { rpc, type SecureBootStatus } from "../api";

  export let csrf: string;
  export let configured: boolean;

  let status: SecureBootStatus | null = null;
  let busy = false;
  let error = "";

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
    <p class="muted small">
      Setup actions that create keys, enroll keys, rebuild, or reboot are not
      exposed yet. They need the privileged re-authentication flow.
    </p>
  {/if}
</Panel>
