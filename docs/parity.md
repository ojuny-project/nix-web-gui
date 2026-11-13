# Capability parity

The web UI is not a copy of the GTK window. It is the same set of actions with
browser equivalents where the desktop depends on a local window, terminal, or
file chooser.

| Action | Local GUI | Engine | Browser component | Minimal | Test |
|---|---|---|---|---|---|
| Open a flake | flake selector | `session.open` | app shell | yes | planned |
| Reload from disk | reload button | `session.reload` | app shell | yes | planned |
| Search options | search entry | `page.list` / search | app shell | yes | planned |
| System-only filter | checkbox | `session.state` | app shell | yes | planned |
| Advanced toggle | toggle | `control.describe` | app shell | yes | planned |
| Switch | switch | `control.set` | UI switch | yes | planned |
| Dropdown | dropdown | `control.set` | UI select | yes | planned |
| Radio | radio | `control.set` | UI radio | yes | planned |
| Spin | spin button | `control.set` | UI number | yes | planned |
| Slider | scale | `control.set` | UI number | yes | planned |
| Text | entry | `control.set` | UI text | yes | planned |
| Multiline | text view | `control.set` | UI textarea | yes | planned |
| Path | entry | `control.set` | UI text | yes | planned |
| Color | color dialog | `control.set` | UI color | yes | planned |
| Package and packages | package picker | `control.set` | UI text/list | yes | planned |
| List | list editor | `control.set` | UI list | yes | planned |
| Module list | module picker | `control.set` | UI list | yes | planned |
| Submodule | nested controls | `control.set` | UI group | yes | planned |
| Table | table editor | `control.set` | UI table | yes | planned |
| Font, icon, cursor | preview chooser | `control.set` | UI chooser | yes | planned |
| Theme | palette picker | `control.set` | UI theme picker | yes | planned |
| Wallpaper | preview strip | `control.set` | UI preview | yes | planned |
| File and icon pickers | native file chooser | upload | UI upload | yes | planned |
| Keybinds | row editor | `control.set` | UI rows | yes | planned |
| Monitors | card editor | `control.set` | UI cards | yes | planned |
| Users | expander editor | `control.set` | UI users | yes | planned |
| Facter | hardware profile | `control.set` | UI select | yes | planned |
| Secure Boot | stage dialog | `secureboot.*` | UI wizard | yes | planned |
| Save | save button | `edits.apply` | app shell | yes | planned |
| Diff | diff dialog | `edits.preview` | app shell | yes | planned |
| Add file | file dialog | `file.add` | app dialog | yes | planned |
| Add option | option picker | `option.add` | app dialog | yes | planned |
| Insert snippet | snippet dialog | `insert.apply` | app dialog | yes | planned |
| Flake inputs | input list | `flake.inputs` | app page | yes | planned |
| nixpkgs card | card | `flake.nixpkgs` | app page | yes | planned |
| Update checks | progress rows | `flake.update.check` | app page | yes | planned |
| Git sync | git button | `git.sync` | app shell | yes | planned |
| Apply command | live apply | `apply.run` | UI status | yes | planned |
| Rebuild | terminal | `rebuild.run` | app log | yes | planned |
| Menu | row editor | `control.set` | UI menu | no | planned |
| Plugins | plugin editor | `control.set` | UI plugins | no | planned |
| Bar | tile editor | `control.set` | UI bar | no | planned |
| Dock | tile editor | `control.set` | UI dock | no | planned |
| Grid | card canvas | `control.set` | UI grid | no | planned |
| Docker | row editor | `control.set` | UI docker | no | planned |

## Current implementation

Implemented end to end:

- Open a flake, scan pages, stage and save changes, preview a diff, discard.
- Switch, dropdown, radio, spin, slider, text, multiline, path, color, package,
  list, packages, module list, theme, and facter controls.
- Nested submodule controls with prefixed paths and staged child edits.
- Generic nested editors for users, monitors, docker, menu, bar, dock, grid,
  plugins, and keybinds until their dedicated renderers land.
- Font, icon-theme, and cursor-theme value selectors from the shared catalog,
  including package sidecar updates.
- A Secure Boot status card with firmware mode, setup mode, key bundle, keys,
  Lanzaboote state, signed files, Windows, BitLocker, ESP space, and warnings.
- A basic `table-attrset` and `table-list` editor with columns, typed cells,
  row add and remove, and key editing.
- A user password action that generates a yescrypt or SHA-512 hash and stages
  it into an existing `hashedPassword` field.
- A media picker for wallpaper and file widgets. It lists files in the widget
  directory, shows image previews, and stages the selected relative value.
- Global search over menus and controls.
- A system-only filter with a host selector backed by the scan import graph.
- `visibleWhen` visibility gates, imported-from origins, live-apply config
  respect, automatic ff-only Git pull on open, and commit-and-push after save
  when `auto_git_sync` is enabled.
- `page.describe` metadata through nixd and the eval fallback.
- Add file, add option, and insert snippet.
- Flake input list, nixpkgs card, add/edit/delete, channels, scans, update check,
  and update.
- Git status, diff, pull, and sync.
- Live apply and Save & rebuild.
- Config panel backed by the shared desktop config file.
- Base16 theme follow over SSE.

Not yet implemented:

- Table row labels, hidden rows, icon columns, and dropdown columns are not yet
  rendered in the web table editor. Dedicated keybind, monitor, user, wallpaper,
  file, icon, and Ojuny renderers are still pending. Secure Boot setup actions
  still need the privileged re-auth flow. The generic nested editor and raw
  fallback are placeholders, not full parity.
- Automated parity tests for the rows above.
- Worker isolation and the remaining auth hardening.
- Playwright tests. The NixOS VM auth test exists and checks service startup,
  alice login, wrong-password rejection, and bob allowlist rejection.

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
