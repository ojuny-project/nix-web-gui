<script lang="ts">
  import { onMount } from "svelte";
  import { rpc, type ScanRecord } from "../api";

  export let parent: ScanRecord;
  export let csrf: string;
  export let onstructurechange: () => Promise<void>;
  export let onterminal: (command: string) => void = () => {};

  type Candidate = {
    name: string;
    path: string;
    uuid?: string | null;
    fstype: string;
    size?: string | null;
    model?: string | null;
    mountpoint: string;
    device: string;
    defaultOptions: string[];
    needsOwner: boolean;
    needsSubvolume: boolean;
  };
  type Disk = {
    name: string;
    path: string;
    size?: string | null;
    model?: string | null;
  };

  let partitions: Candidate[] = [];
  let disks: Disk[] = [];
  let unformatted: Disk[] = [];
  let partitionIndex = 0;
  let diskIndex = 0;
  let unformattedIndex = 0;
  let filesystem = "ext4";
  let owner = 0;
  let group = 0;
  let others = 0;
  let subvolume = "@data";
  let busy = false;
  let error = "";
  let note = "";

  const filesystems = ["ext4", "btrfs", "xfs", "f2fs", "vfat", "exfat", "ntfs"];

  async function load() {
    error = "";
    try {
      const result = await rpc<{
        partitions: Candidate[];
        disks: Disk[];
        unformatted: Disk[];
      }>("mounts.candidates", {}, csrf);
      partitions = result.partitions;
      disks = result.disks;
      unformatted = result.unformatted;
      if (partitionIndex >= partitions.length) partitionIndex = 0;
      if (diskIndex >= disks.length) diskIndex = 0;
      if (unformattedIndex >= unformatted.length) unformattedIndex = 0;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot read disks";
    }
  }

  onMount(load);

  function bits(index: number) {
    return index === 0 ? 6 : index === 1 ? 4 : 0;
  }

  function umask() {
    const mode = (bits(owner) << 6) | (bits(group) << 3) | bits(others);
    return (0o777 & ~mode).toString(8).padStart(3, "0");
  }

  function selected(): Candidate | undefined {
    return partitions[partitionIndex];
  }

  async function addOne(candidate: Candidate, withOwner: boolean, withSubvolume: boolean) {
    const options = [...candidate.defaultOptions, "nofail"];
    if (withOwner) options.push(`umask=0${umask()}`);
    if (withSubvolume && subvolume.trim()) options.push(`subvol=${subvolume.trim()}`);
    await rpc(
      "mounts.add",
      {
        path: parent.path,
        mountpoint: candidate.mountpoint,
        device: candidate.device,
        fsType: candidate.fstype,
        options,
      },
      csrf,
    );
  }

  async function addSelected() {
    const candidate = selected();
    if (!candidate) return;
    busy = true;
    error = "";
    try {
      await addOne(candidate, candidate.needsOwner, candidate.needsSubvolume);
      await onstructurechange();
      note = `Added ${candidate.mountpoint}`;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot add the mount";
    } finally {
      busy = false;
    }
  }

  async function automountAll() {
    busy = true;
    error = "";
    try {
      for (const candidate of partitions) {
        await addOne(candidate, candidate.needsOwner, candidate.needsSubvolume);
      }
      await onstructurechange();
      note = `Added ${partitions.length} mount${partitions.length === 1 ? "" : "s"}`;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot add all mounts";
    } finally {
      busy = false;
    }
  }

  function openCfdisk() {
    const disk = disks[diskIndex];
    if (disk) onterminal(`sudo cfdisk ${disk.path}`);
  }

  async function applyFilesystem() {
    const part = unformatted[unformattedIndex];
    if (!part) return;
    busy = true;
    error = "";
    try {
      const result = await rpc<{ command: string }>(
        "mounts.mkfs",
        { fsType: filesystem, device: part.path },
        csrf,
      );
      onterminal(`sudo ${result.command}`);
      note = `Started mkfs on ${part.path}. Refresh when it finishes.`;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Cannot start mkfs";
    } finally {
      busy = false;
    }
  }
</script>

<div class="mounts-panel">
  <div class="toolbar">
    <strong>Detected disks</strong>
    <span class="spacer"></span>
    <button class="ghost" onclick={load} disabled={busy}>Refresh</button>
  </div>

  <div class="mount-row">
    <select bind:value={partitionIndex} disabled={busy || partitions.length === 0}>
      {#each partitions as part, index}
        <option value={index}>
          {part.name} · {part.fstype} · {part.size ?? ""} {part.model ?? ""}
        </option>
      {/each}
    </select>
    <button class="ghost" onclick={addSelected} disabled={busy || partitions.length === 0}>
      Add from disk
    </button>
    <button class="ghost" onclick={automountAll} disabled={busy || partitions.length === 0}>
      Automount all
    </button>
  </div>

  {#if selected()?.needsOwner}
    <div class="mount-row">
      <label>
        Owner
        <select bind:value={owner}>
          <option value={0}>Read and write</option>
          <option value={1}>Read only</option>
          <option value={2}>None</option>
        </select>
      </label>
      <label>
        Group
        <select bind:value={group}>
          <option value={0}>Read and write</option>
          <option value={1}>Read only</option>
          <option value={2}>None</option>
        </select>
      </label>
      <label>
        Everyone
        <select bind:value={others}>
          <option value={0}>Read and write</option>
          <option value={1}>Read only</option>
          <option value={2}>None</option>
        </select>
      </label>
      <span class="muted small">umask 0{umask()}</span>
    </div>
  {/if}

  {#if selected()?.needsSubvolume}
    <div class="mount-row">
      <label>
        Subvolume
        <input bind:value={subvolume} placeholder="@data" />
      </label>
      <span class="muted small">The subvolume must already exist.</span>
    </div>
  {/if}

  <div class="mount-row">
    <select bind:value={diskIndex} disabled={busy || disks.length === 0}>
      {#each disks as disk, index}
        <option value={index}>{disk.name} · {disk.size ?? ""} {disk.model ?? ""}</option>
      {/each}
    </select>
    <button class="ghost" onclick={openCfdisk} disabled={busy || disks.length === 0}>
      Open cfdisk
    </button>
  </div>

  <div class="mount-row">
    <select bind:value={unformattedIndex} disabled={busy || unformatted.length === 0}>
      {#each unformatted as part, index}
        <option value={index}>{part.name} · {part.size ?? ""} {part.model ?? ""}</option>
      {/each}
    </select>
    <select bind:value={filesystem}>
      {#each filesystems as type}
        <option value={type}>{type}</option>
      {/each}
    </select>
    <button
      class="ghost"
      onclick={applyFilesystem}
      disabled={busy || unformatted.length === 0}
    >
      Apply filesystem
    </button>
  </div>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
  {#if note}
    <p class="muted small">{note}</p>
  {/if}
</div>

<style>
  .mounts-panel {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin: 0.75rem 0;
  }

  .mount-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
  }

  .mount-row select,
  .mount-row input {
    min-width: 10rem;
  }

  .mount-row label {
    display: flex;
    gap: 0.35rem;
    align-items: center;
  }
</style>
