# Limitations

This file collects the documented limits of the web UI. It is not a bug list.
The working list is `/home/den/repos/ojuny-future-repos/nix-web-gui/TODO.md`. The
authoritative module-shape rule is
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
- Custom widgets own their data. `facter` reads the record's `profileCatalog`
  JSON and falls back to the built-in catalog. `docker` reads and writes the
  container set. Neither resolves an arbitrary Nix expression.

## Composite editor coverage

The structured widgets render through
`/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/app/src/pages/CardCollection.svelte`
as row cards. The exact mapping is in
`/home/den/repos/ojuny-future-repos/nix-web-gui/docs/widgets.md`.

- `submodule` and `horizontal-submodule`: the parent becomes a heading and its
  described children become indented controls. `maxColumns` sets the card grid
  width. There is no per-child details row parity.
- `users`: account cards with add, remove, rename, validation, a password action,
  and `home-manager.users` sync when the sibling block exists. Save validates
  names, UIDs, shells, and the sudo-and-password invariant.
- `monitors`: output cards on a draggable canvas that updates `xPos` and `yPos`.
  `disabledFeatures` hides matching fields and their staged edits are skipped on
  save. The add-row panel detects outputs through `monitor.detect`.
- `keybinds`: row cards with configurable `bindFields`, `bindFieldTypes`,
  `bindPreview`, `bindSort`, and `bindKeyField`; list-typed fields render as
  comma-separated inputs.
- `docker`: row cards with configurable container field names; empty container
  fields are hidden.
- `bar` and `dock`: list-valued sections with attribute-set elements are
  described as row cards. The add dialog offers `@widget-preset` tiles and writes
  the preset defaults. Lists of plain strings keep the list editor.
- `grid-configuration`: cards use the placement canvas with `col`, `row`, `w`,
  and `h`, drag and resize handles, and `gridRegistry` ids as add-row choices.
- `menu`, `plugins`: row cards. The plugin add dialog lists the DMS registry and
  offers uninstalled entries. `plugins.settings` QML renders as schema-driven
  controls, and menus have the provider assistant with inline field help.
- `wallpaper` and `file`: the media picker lists, previews, uploads, and deletes
  files. A wallpaper with `presetSource` also lists generated presets using
  `presetOption`, and the picker offers a Default preset that stages `null`.
  Uploads and deletes run `git add`. Paths outside the option's directory
  fields are not browsable.
- `secureboot`: see below.

Row add, remove, rename, and list move are refused while staged edits exist.
Rename is offered for users, keybinds, monitors, docker, menu, and
grid-configuration. A `submodule` row can be removed but not added or renamed; a
`horizontal-submodule` row can be added and removed but not renamed.

## Table gaps

`/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/ui/src/lib/controls/Table.svelte`
implements hidden rows with a show toggle, row labels, locked keys, static first
cells for `table-list`, bool cells, dropdown cells, icon and image thumbnails
with upload buttons, move up and down, add, and remove.

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
  root parent and fail outside it. Each one requires the device password again
  and verifies it through PAM before running.
- The Lanzaboote flake input is still added from the Flake Inputs page.
- Secure Boot setup output returns at the end of one call. The Secure Boot
  rebuild streams one line per output line into the panel while it runs.

## Streaming and cancellation

`GET /api/events` sends `theme`, `theme.changed`, `progress`, and `heartbeat`.
Progress notes carry a `job` and one line of text:

- `scan` emits a start line, the menu count, and one line per menu file.
- `save` emits one `Writing <file>` line per distinct file, then a final
  `Wrote <n> file(s)` line.
- `apply` emits one line per command output line.
- `secureboot-rebuild` emits one line per rebuild output line.
- `flake.update.check` emits one line per input and can be cancelled.
- `rebuild.run` emits one line per output line and can be cancelled.

The shell shows stage and update lines in the status bar, appends rebuild lines
to the Rebuild output panel, and appends Secure Boot rebuild lines to the
Secure Boot panel. Cancel sends `job.cancel`, which terminates the session
worker and reopens the session. The same cancel path applies to scan, save, and
apply while they run in a worker; the in-process dev engine cannot cancel.

In-process development mode has no worker process, so `job.cancel` reports that
there is no running job.

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
- Rebuild output streams to the page and is also returned in the JSON result.

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

The service includes a repository-declared `system-auth` PAM service, so
`pam_unix`, account expiry, faillock, MFA, LDAP, and Kerberos modules configured
there apply to web login. Account expiry has a VM assertion; the other modules
are inherited but depend on operator provisioning. TLS is intentionally out of
scope for the local-only design.

## UI flows with limited rendering

`/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/app/src/pages/Shell.svelte`
reaches every major flow from a visible control: the sidebar has Open flake, Add
file, Settings, Git, and Flake Inputs, and the settings toolbar has Reload, Add
option, Discard, Diff, Save, and Save & rebuild.

The remaining limits are about feedback, not reachability:

- a rebuild streams output lines and has a Cancel button, but cancelling
  terminates the worker and reopens the session;
- an update check streams one line per input and has a Cancel check button with
  the same worker-restart behaviour;
- a scan sends a start line, the menu count, and one line per menu file; a save
  sends one line per written file; apply sends one command line per output line;
  `job.cancel` terminates the worker for any of them;
- the config panel no longer shows `terminal_command`, because the web frontend
  never opens a terminal. The stored value is preserved for the desktop app.

## Minimal variant behaviour

`packages.nix-web-gui-minimal` builds the server with
`--no-default-features --features pam` and the frontend with
`VITE_OJUNY_WIDGETS=0`. `system.capabilities` reports `ojunyWidgets: false`.

The visible difference is that the minimal UI filters out the descendants of
every Ojuny record and shows `This Ojuny widget is not available in the minimal
build.` in place of the record itself for `menu`, `plugins`, `bar`, `dock`,
`grid-configuration`, and `docker`. The engine emits those exact kebab-case
names.

The Docker environment editor and the plugin settings editor are replaced at
build time by a stub through the Vite `@ojuny/*` aliases, so their code does not
ship in the minimal bundle. The card canvas stays shared because users,
keybinds, monitors, and nested submodules use it too.

`checks.minimal-flag` proves the notice is present.
`checks.minimal-renderer` proves the two dedicated editors are absent.
