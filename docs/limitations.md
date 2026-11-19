# Limitations

This file collects the documented limits of the web UI. It is not a bug list.
Items marked pending point to `/home/den/repos/ojuny-future-repos/nix-web-gui/TODO.md`,
which is the working list. The authoritative module-shape rule is
`/home/den/repos/ojuny-template/docs/05-settings-gui-limitations.md`.

## Module shape rule

The scanner binds an `@option` comment to the next assignment in the same
container. It does not descend into a list of attribute sets, a function call
wrapper, or an apply node. This is a documented limitation, not a scanner bug.
The fix is always to change the module shape. The scanner is never extended to
accept these shapes.

These shapes make the control disappear:

```nix
config = lib.mkMerge [
  {
    /* @option widget = "switch"; label = "Enable example"; */
    ojuny.example.enable = true;
  }
];
```

```nix
config = [
  {
    /* @option widget = "switch"; label = "Enable example"; */
    ojuny.example.enable = true;
  }
];
```

```nix
config = map (name: {
  /* @option widget = "switch"; label = name; */
  ojuny.example.${name}.enable = true;
}) names;
```

```nix
config = lib.optionalAttrs active {
  /* @option widget = "switch"; label = "Enable example"; */
  ojuny.example.enable = true;
};
```

The supported shape is a plain attribute set, or annotations directly inside a
set that the module evaluates as bindings:

```nix
{ ... }:
{
  /* @option widget = "switch"; label = "Enable example"; */
  ojuny.example.enable = true;
}
```

The two-module pattern from
`/home/den/repos/ojuny-template/docs/05-settings-gui-limitations.md` keeps the
annotated defaults in the front module and the gated install logic in a backend
module behind `lib.mkIf`.

The gate that rejects a scanner patch adding list or apply-node descent is the
test `gate_limitation_no_list_descent` in
`/home/den/repos/ojuny-future-repos/nix-web-gui/engine/src/session.rs`. Its
fixture is `/home/den/repos/ojuny-future-repos/nix-web-gui/tests/fixtures/limitations`.
`/home/den/repos/ojuny-future-repos/nix-web-gui/docs/parity.md` names the same
gate as `gate-limitation-no-list-descent`.

### Disagreement about `lib.mkIf`

The two governing documents disagree about a `lib.mkIf` body:

- `/home/den/repos/ojuny-template/docs/05-settings-gui-limitations.md` says
  `lib.mkIf` is supported, and an annotated assignment may sit directly inside
  the conditional attribute set.
- `/home/den/repos/ojuny-future-repos/nix-web-gui/README.md`,
  `/home/den/repos/ojuny-future-repos/nix-web-gui/docs/architecture.md`, and the
  `mkif.nix` fixture in
  `/home/den/repos/ojuny-future-repos/nix-web-gui/tests/fixtures/limitations`
  treat `config = lib.mkIf true { /* @option ... */ time.timeZone = "UTC"; }` as
  invisible. The gate asserts that `time.timeZone` is absent.

The safe shape satisfies both documents: keep annotated assignments in a plain
attribute set, or use the front-module and backend-module split. Do not patch
the scanner to make a `lib.mkIf` body visible.

Other limits from the same document that apply to the web UI:

- The GUI edits the value span of an existing assignment. It cannot create an
  option without the authoring action.
- Dynamic attribute names (`${...}`) are not addressable.
- A value wrapped in a function call is not rewritten.
- Custom widgets own their data. `facter` finds the first `facter.json` in the
  flake, falling back to `modules/facter.json`. `docker` reads and writes the
  container set. Neither resolves an arbitrary Nix expression.

## Generic editor substitution

Dedicated desktop renderers do not exist in the browser for the structured
widgets. They use the generic nested editor. The exact mapping is in
`/home/den/repos/ojuny-future-repos/nix-web-gui/docs/widgets.md`.

- `submodule` and `horizontal-submodule`: the parent becomes a heading and its
  described children become ordinary indented controls. `maxColumns` is not
  applied to horizontal submodules. There is no per-child details row parity.
- `users`: no account cards and no separate sudo switch. Add, remove, and rename
  work, `home-manager.users` follows when present, and save validates names,
  UIDs, shells, and the sudo-and-password invariant.
- `monitors`: no output cards and no `disabledFeatures` filtering. The add-row
  panel detects outputs through `monitor.detect`.
- `keybinds`: no dedicated row editor and no comma-list handling. Add, remove,
  and rename work on the rows.
- `menu`, `plugins`, `bar`, `dock`, `grid-configuration`, `docker`: no dedicated
  Ojuny editors, catalogs, grid canvas, drag, resize, or profiles. Existing rows
  edit through the generic controls, and bar or dock tile rows can be added with
  an id.
- `bar` and `dock`: list-valued sections with attribute-set elements are
  described as child controls. Lists of plain strings keep the list editor.
- `wallpaper` and `file`: the media picker lists, previews, uploads, and deletes
  files. A wallpaper with `presetSource` also lists generated presets and their
  previews; the preset name is staged. Wallpaper preset sources and custom path handling are pending.
- `secureboot`: see below.

Row add, remove, and rename are refused while staged edits exist. Rename is
offered for users, keybinds, monitors, docker, menu, and grid-configuration;
the generic `submodule` and `horizontal-submodule` rows can be added and removed
but not renamed from the page.

## Table gaps

`/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/ui/src/lib/controls/Table.svelte`
implements hidden rows with a show toggle, row labels, locked keys, static first
cells for `table-list`, bool cells, dropdown cells, icon and image upload
buttons, move up and down, add, and remove.

Remaining gaps:

- Columns not declared in `columns` are not rendered. The serializer writes
  their existing values back unchanged, so a row edit does not drop them.
- A table with no declared columns falls back to the raw textarea, and the
  serializer rejects that string with `table has no columns`.

## Secure Boot setup actions

The web UI renders the status and actions in
`/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/app/src/pages/SecureBootPanel.svelte`:
firmware mode, UEFI, setup mode, sbctl, key bundle, keys present and enrolled,
Lanzaboote, signed and unsigned files, Windows, BitLocker, ESP free space, and
the warnings list.

Implemented actions:

- key creation or migration from `/etc/secureboot`;
- enrollment with the Microsoft and firmware-builtin options;
- the Lanzaboote configuration block, written into the module that owns the
  toggle;
- rebuild and reboot, with a confirmation for reboot;
- the enrollment marker after a successful enrollment.

Limits:

- Firmware steps stay manual: reset to Setup Mode before enrolling, then enable
  Secure Boot without resetting the keys.
- `secureboot.setup`, `secureboot.rebuild`, and `secureboot.reboot` run in the
  root parent and fail outside it. There is no per-action password
  re-authentication.
- The Lanzaboote flake input is still added from the Flake Inputs page.
- Setup output is returned at the end of one blocking call. Rebuild output
  streams line by line.

## Streaming and cancellation

`GET /api/events` sends `theme`, `theme.changed`, `progress`, and `heartbeat`.
Only `rebuild.run` emits progress notes, one per output line. The shell appends
them to the Rebuild output panel and offers a Cancel button while a rebuild is
running. Cancelling sends `job.cancel`, which terminates the session worker and
reopens the session.

Everything else is a request and a response:

- `scan` and `page.describe` return when complete; scan sends one progress line.
- `edits.apply` writes and rescans in one call; it sends one progress line.
- `apply.run` runs a shell command and returns at the end; it sends one progress
  line.
- `flake.update.check` streams one progress line per input and can be cancelled.

In-process development mode has no worker process, so `job.cancel` reports that
there is no running job. There is no progress stream or cancellation for scan,
save, or apply.

The client serializes live apply calls with one queue and keeps the latest value
per path, so a debounce cannot run two `apply.run` calls at once. The server does
not add a second queue.

## Apply and rebuild privileges

`apply.run` and `rebuild.run` execute as the signed-in account, inside that
account's worker. They inherit the account's sudo configuration.

- There is no password re-entry.
- There is no `sudo -S` input path.
- There is no general root helper and no arbitrary root command.
- `rebuild_command` and each `applyCommand` are shell strings run with
  `sh -lc` in the flake root.
- `terminal_command` is stored in
  `$HOME/.config/nix-settings-gui/config.json` and is not used by the web
  rebuild path.
- Rebuild output is returned in the JSON result and the page does not render it.

## Auth hardening

Implemented in `/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/http.rs`,
`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/auth/mod.rs`, and
`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/auth/session.rs`:

- login throttling with a five-failure, five-minute window and a five-minute
  lockout;
- a maximum active session count with oldest-first eviction;
- a five-minute cleanup of expired sessions and stale workers;
- the `Secure` cookie flag from `X-Forwarded-Proto: https` or `--secure-cookie`;
- an `Origin` check on login, logout, and RPC;
- a strict CSP and standard security headers;
- structured tracing for login, logout, rate limits, and every RPC.

Pending from `/home/den/repos/ojuny-future-repos/nix-web-gui/TODO.md`:

- full PAM `system-auth` behaviour, account expiry, and multi-factor modules;
- built-in TLS.

PAM still uses explicit `pam_unix.so` auth and account lines in
`/home/den/repos/ojuny-future-repos/nix-web-gui/modules/nixos.nix`.

## UI flows with limited rendering

`/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/app/src/pages/Shell.svelte`
reaches every major flow from a visible control: the sidebar has Open flake, Add
file, Settings, Git, and Flake Inputs, and the settings toolbar has Reload, Add
option, Discard, Diff, Save, and Save & rebuild.

The remaining limits are about feedback, not reachability:

- a rebuild streams output lines and has a Cancel button, but cancelling
  terminates the worker and reopens the session;
- a scan, save, update check, or apply returns one blocking response and has no
  cancel action;
- the config panel no longer shows `terminal_command`, because the web frontend
  never opens a terminal. The stored value is preserved for the desktop app.

## Minimal variant behaviour

`packages.nix-web-gui-minimal` builds the server with
`--no-default-features --features pam` and the frontend with
`VITE_OJUNY_WIDGETS=0`. `system.capabilities` reports `ojunyWidgets: false`.

The visible difference is that the minimal UI filters out every Ojuny record and
its descendants, then shows `This Ojuny widget is not available in the minimal
build.` for `menu`, `plugins`, `bar`, `dock`, `grid-configuration`, and
`docker`. The engine emits those exact kebab-case names. There are no dedicated
Ojuny components to exclude yet, so the default and minimal trees share every
renderer implementation.

`checks.minimal-flag` only greps for the notice string. It does not prove that no
Ojuny renderer ships.
