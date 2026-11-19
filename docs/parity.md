# Capability parity

The web UI is not a copy of the GTK window. It is the same set of actions with
browser equivalents where the desktop depends on a local window, terminal, or
file chooser.

The `Test` column states the implementation status: `implemented`, `partial`, or
`pending`. `Minimal` says whether the minimal package keeps the capability.

| Action | Local GUI | Engine | Browser component | Minimal | Test |
|---|---|---|---|---|---|
| Open a flake | flake selector | `session.open`, `config.remember` | app shell | yes | implemented |
| Reload from disk | reload button | `session.reload` | toolbar Reload button | yes | implemented |
| Search options | search entry | `page.list` / `scan` | global results with jump and highlight | yes | implemented |
| System-only filter | checkbox | `scan` with `host` | app shell, current host preselected | yes | implemented |
| Advanced toggle | toggle | `control.describe` | Advanced section for advanced and hidden controls | yes | implemented |
| Switch | switch | `control.setTyped` | UI switch | yes | implemented |
| Dropdown | dropdown | `control.setTyped` | UI select | yes | implemented |
| Radio | radio | `control.setTyped` | UI radio | yes | implemented |
| Spin | spin button | `control.setTyped` | UI number | yes | implemented |
| Slider | scale | `control.setTyped` | UI number with range | yes | implemented |
| Text | entry | `control.setTyped` | UI text | yes | implemented |
| Multiline | text view | `control.setTyped` | UI textarea | yes | implemented |
| Path | entry | `control.setTyped` | UI text, no path picker | yes | implemented |
| Color | color dialog | `control.setTyped` | UI color | yes | implemented |
| Package and packages | package picker | `control.setTyped`, nixd validation | UI text or string list | yes | implemented |
| Plain string list | list editor | `control.setTyped` | UI list | yes | implemented |
| Module list | module picker | `control.setTyped` | UI list | yes | implemented |
| Submodule | nested controls | `control.setTyped` | generic child controls | yes | partial |
| Table | table editor | `control.setTyped` | hidden rows, labels, locked keys, typed and dropdown cells, move, icon upload | yes | implemented |
| Composite rows | nested controls | `structure.add`, `structure.remove`, `structure.rename` | add and remove rows; rename attribute-set rows | yes | partial |
| Font, icon, cursor | preview chooser | `control.setTyped`, `catalog.styles` | select with filtering and font preview | yes | partial |
| Theme | palette picker | `control.setTyped` | searchable swatch picker | yes | implemented |
| Wallpaper | preview strip | `preview.*` | picker, presets, upload, delete | yes | implemented |
| File and icon pickers | native file chooser | `preview.upload`, `preview.delete` | media picker and table icon picker | yes | implemented |
| Keybinds | row editor | `control.setTyped`, `structure.*` | row cards with configurable schemas and CSV lists | yes | implemented |
| Monitors | card editor | `control.setTyped`, `structure.*`, `monitor.detect` | output cards, drag canvas, output detection, disabled features | yes | implemented |
| Users | expander editor | `control.setTyped`, `structure.*` | account cards, validation, home-manager sync | yes | implemented |
| Facter | hardware profile | `control.setTyped` | UI select | yes | implemented |
| Secure Boot | stage dialog | `secureboot.*` | status panel plus setup actions | yes | partial |
| Save | save button | `edits.apply` | app shell | yes | implemented |
| Diff | diff dialog | `edits.preview` | staged-change panel | yes | implemented |
| Add file | file dialog | `file.add`, `file.templates` | sidebar Add file button and panel | yes | implemented |
| Add option | option picker | `option.add` | app dialog | yes | implemented |
| Insert snippet | snippet dialog | `insert.apply` | app dialog | yes | implemented |
| Flake inputs | input list | `flake.inputs`, `flake.input.*` | Flake Inputs page | yes | implemented |
| Missing inputs | input list | `flake.input.addAll` | banner and Declare all | yes | implemented |
| nixpkgs card | card | `flake.inputs`, `flake.update` | card with Check, Refresh, Edit | yes | implemented |
| Update checks | progress rows | `flake.update.check` | per-input Check button | yes | implemented |
| Git status, pull, push, commit, sync | git actions | `git.*` | Git page | yes | implemented |
| Working-tree diff | diff dialog | `git.diff` | Git page diff panel | yes | implemented |
| Apply command | live apply | `apply.run` | serial client apply queue | yes | implemented |
| Rebuild | terminal | `rebuild.run`, `job.cancel` | streamed output panel with Cancel | yes | implemented |
| Menu | row editor | `control.setTyped`, `structure.*` | row cards; no provider assistant | no | partial |
| Plugins | plugin editor | `control.setTyped`, `structure.*`, `plugins.*` | row cards and registry add; QML controls pending | no | partial |
| Bar | tile editor | `control.setTyped` | structured list sections | no | partial |
| Dock | tile editor | `control.setTyped` | structured list sections | no | partial |
| Grid | card canvas | `control.setTyped`, `structure.*` | placement canvas; drag/resize pending | no | partial |
| Docker | row editor | `control.setTyped`, `structure.*` | generic rows | no | partial |
| Config and flake path | settings dialog | `config.get`, `config.set`, `config.remember` | sidebar Settings button and panel | yes | implemented |

## Implemented end to end

- Open a flake, scan pages, stage and save changes, preview a diff, and discard.
- The typed controls: switch, dropdown, radio, spin, slider, text, multiline,
  path, color, package, packages, plain list, module list, theme, and facter.
- Nested submodule controls with prefixed paths and staged child edits.
- The table editor: hidden rows, row labels, locked keys, static first cells for
  `table-list`, bool, text, dropdown, and icon/image cells, move up and down, and
  add and remove.
- Composite row add and remove through `structure.*` for users, keybinds,
  monitors, docker, menu, submodule, horizontal-submodule, and
  grid-configuration. Rename is offered for users, keybinds, monitors, docker,
  menu, and grid-configuration. User changes follow `home-manager.users` when
  present.
- User validation before save: valid account names, unique positive UIDs,
  `pkgs.*` shells, and at least one normal sudo account with a password.
- Font, icon-theme, and cursor-theme selectors from the shared catalog, with
  monospace and variant filtering and a font preview.
- A Secure Boot status card with firmware mode, setup mode, key bundle, keys,
  Lanzaboote state, signed files, Windows, BitLocker, ESP space, warnings, and
  the stage title and detail.
- Secure Boot key create or migrate, enrollment with the Microsoft and
  firmware-builtin options, configuration write into the owning module, rebuild,
  and reboot.
- A user password action that generates a yescrypt or SHA-512 hash and stages it
  into a `hashedPassword` field.
- A media picker for wallpaper and file widgets with listing, image previews,
  upload, and delete. Table icon and image columns upload into `iconDir`.
- A root-parent and per-user worker boundary with runtime PATH tooling.
- Global search with result navigation and highlight, system-only host
  filtering with current-host preselection, and grouped advanced and hidden
  controls.
- `visibleWhen` visibility gates, imported-from origins, live-apply config
  respect, automatic ff-only Git pull on open, and commit-and-push after save
  when `auto_git_sync` is enabled.
- `page.describe` metadata through nixd and the eval fallback.
- Add file through `file.add`, add option, and insert snippet.
- Flake input list, nixpkgs card, add/edit/delete, add-all, per-input check and
  update, and nixpkgs refresh.
- The Git page: status, pull, push, commit, sync, and the working-tree diff.
- Live apply with a serial client queue, and Save & rebuild.
- Config `flake_path` drives the initial open, opening a flake calls
  `config.remember`, and the config form exposes the path.
- Base16 theme follow over SSE, theme search, and swatches.

## Partial and pending

- The generic nested editor replaces dedicated keybind, monitor, user-card, and
  Ojuny renderers. The desktop layouts, tile catalog, bar and dock tile
  actions, grid canvas, and plugin registry are still pending. Bar and dock tile
  rows can be added, removed, renamed, moved, and edited as structured list
  elements.
- Wallpaper cycle settings next to the option are pending.
- Icon-theme and cursor-theme preview sheets are pending.
- Secure Boot has no per-action privileged re-authentication. Actions run in the
  root parent and fail clearly outside it.
- Rebuild output and update checks stream and can be cancelled by terminating
  the session worker. Scan, save, and apply remain blocking.
- Full PAM `system-auth`, account expiry, and multi-factor modules are pending.
- Built-in TLS is pending.
- Playwright and frontend unit tests are pending. `nix flake check` passes and
  `checks.parity` exists.
- The portable flake input and `binary-url.json` are pending. A tagged release
  workflow exists.

## Documented limitation

The scanner keeps the documented module-shape rule. An `@option` inside
`lib.mkMerge`, a list literal, `map`, `forEach`, `lib.optionalAttrs`, or a
`lib.mkIf` body is ignored. Keep annotated assignments in a plain attribute set.
The fix is always to change the module shape, never to add list or apply-node
descent to the scanner. The authoritative documents are
`/home/den/repos/ojuny-template/docs/05-settings-gui-limitations.md`,
`/home/den/repos/nix-settings-gui/docs/comment-syntax.md`,
`/home/den/repos/nix-settings-gui/docs/architecture.md`, and
`/home/den/repos/nix-settings-gui/docs/usage.md`.

## Gates

- `parity-capabilities`: no row may be missing an engine method, component,
  minimal flag, or test id.
- `gate-widget-output`: the engine writes the same Nix literal and applies the
  same validation as the desktop for a shared fixture.
- `gate-limitation-no-list-descent`: annotations inside lists stay invisible.
- `gate-minimal-no-ojuny`: the minimal build contains no Ojuny renderer or
  engine feature.
- `gate-ui-boundary`: the UI package has no auth, router, or RPC client.
- `gate-engine-boundary`: the engine has no server or frontend dependencies.

`checks.parity` is implemented. It checks `tests/widget-names.txt` against
`docs/widgets.md`, rejects the retired concatenated spellings in the docs and
the dispatcher, and asserts that `record_json` uses `render::widget_name`.
