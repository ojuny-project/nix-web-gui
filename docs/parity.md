# Capability parity

The web UI is not a copy of the GTK window. It aims to cover the same actions
with browser equivalents where the desktop depends on a local window, terminal,
or file chooser. Rows marked `partial` are not desktop-equivalent yet.

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
| Table | table editor | `control.setTyped` | hidden rows, labels, locked keys, typed and dropdown cells, move, icon upload and thumbnails | yes | implemented |
| Composite rows | nested controls | `structure.add`, `structure.remove`, `structure.rename`, `structure.move` | row cards with add and remove, rename for attribute-set rows, and list move | yes | implemented |
| Font, icon, cursor | preview chooser | `control.setTyped`, `catalog.styles` | select with filtering, font sample, and icon or cursor previews | yes | implemented |
| Theme | palette picker | `control.setTyped` | searchable swatch picker | yes | implemented |
| Wallpaper | preview strip | `preview.*` | picker, preset tiles, Default preset, upload, delete | yes | implemented |
| File and icon pickers | native file chooser | `preview.upload`, `preview.delete` | media picker and table icon picker with thumbnails | yes | implemented |
| Keybinds | row editor | `control.setTyped`, `structure.*` | row cards with configurable schemas, CSV lists, previews, and sorting | yes | implemented |
| Monitors | card editor | `control.setTyped`, `structure.*`, `monitor.detect` | output cards, drag canvas, output detection, disabled features | yes | implemented |
| Users | expander editor | `control.setTyped`, `structure.*` | account cards, validation, password action, home-manager sync | yes | implemented |
| Facter | hardware profile | `control.setTyped` | UI select with `profileCatalog` profiles | yes | implemented |
| Secure Boot | stage dialog | `secureboot.*` | status panel plus setup actions | yes | partial |
| Save | save button | `edits.apply` | app shell | yes | implemented |
| Diff | diff dialog | `edits.preview` | staged-change panel | yes | implemented |
| Add file | file dialog | `file.add`, `file.templates` | sidebar Add file button and panel | yes | implemented |
| Add option | option picker | `option.add` | app dialog | yes | implemented |
| Insert snippet | snippet dialog | `insert.apply` | app dialog | yes | implemented |
| Flake inputs | input list | `flake.inputs`, `flake.input.*` | Flake Inputs page | yes | implemented |
| Missing inputs | input list | `flake.input.addAll` | banner and Declare all | yes | implemented |
| nixpkgs card | card | `flake.inputs`, `flake.update` | card with Check, Refresh, Edit | yes | implemented |
| Update checks | progress rows | `flake.update.check` | per-input Check with streamed lines and Cancel check | yes | implemented |
| Git status, pull, push, commit, sync | git actions | `git.*` | Git page | yes | implemented |
| Working-tree diff | diff dialog | `git.diff` | Git page diff panel | yes | implemented |
| Apply command | live apply | `apply.run` | serial client apply queue | yes | implemented |
| Rebuild | terminal | `rebuild.run`, `job.cancel` | streamed output panel with Cancel | yes | implemented |
| Menu | row editor | `control.setTyped`, `structure.*` | row cards with provider assistance and inline field help | no | implemented |
| Plugins | plugin editor | `control.setTyped`, `structure.*`, `plugins.*` | row cards, registry add, and schema-driven settings controls | no | implemented |
| Bar | tile editor | `control.setTyped`, `structure.*` | row cards; `@widget-preset` tiles and list move | no | partial |
| Dock | tile editor | `control.setTyped`, `structure.*` | row cards; `@widget-preset` tiles and list move | no | partial |
| Grid | card canvas | `control.setTyped`, `structure.*` | placement canvas with `gridRegistry` ids, drag, and resize; catalog sizes and profiles pending | no | partial |
| Docker | row editor | `control.setTyped`, `structure.*` | row cards with configurable container field names | no | implemented |
| Config and flake path | settings dialog | `config.get`, `config.set`, `config.remember` | sidebar Settings button and panel | yes | implemented |

## Implemented end to end

- Open a flake, scan pages, stage and save changes, preview a diff, and discard.
- The typed controls: switch, dropdown, radio, spin, slider, text, multiline,
  path, color, package, packages, plain list, module list, theme, and facter.
- Nested submodule controls with prefixed paths and staged child edits.
- The table editor: hidden rows, row labels, locked keys, static first cells for
  `table-list`, bool, text, dropdown, and icon/image cells, move up and down, and
  add and remove.
- Composite row cards with add and remove, rename for attribute-set rows, and
  list move through `structure.*` for users, keybinds, monitors, docker, menu,
  bar, dock, horizontal-submodule, grid-configuration, and structured list
  sections. A `submodule` row can be removed but not added or renamed from the
  page. User changes follow `home-manager.users` when present.
- Keybind rows with configurable `bindFields`, `bindFieldTypes`, `bindPreview`,
  `bindSort`, `bindKeyField`, and comma-separated list fields. Docker rows use
  configurable container field names and hide empty ones.
- Monitor output cards on a draggable `xPos`/`yPos` canvas, with `monitor.detect`
  prefill and `disabledFeatures` filtering.
- Grid cards on the placement canvas with `col`, `row`, `w`, and `h`, drag and
  resize, and `gridRegistry` add choices.
- Bar and dock section add dialogs with `@widget-preset` tiles that write the
  preset defaults, plus list add, remove, rename, and move.
- Plugin row cards and an add dialog that lists the DMS registry and offers
  uninstalled entries. `plugins.settings` returns QML, but no QML controls are
  rendered.
- User account cards with valid account names, unique positive UIDs, `pkgs.*`
  shells, and at least one normal sudo account with a password.
- Font, icon-theme, and cursor-theme selectors from the shared catalog, with
  monospace and variant filtering, a font preview, and icon and cursor preview
  thumbnails.
- A Secure Boot status card with firmware mode, setup mode, key bundle, keys,
  Lanzaboote state, signed files, Windows, BitLocker, ESP space, warnings, and
  the stage title and detail.
- Secure Boot key create or migrate, enrollment with the Microsoft and
  firmware-builtin options, configuration write into the owning module, rebuild,
  and reboot.
- A user password action that generates a yescrypt or SHA-512 hash and stages it
  into a `hashedPassword` field.
- A media picker for wallpaper and file widgets with listing, image previews,
  `presetSource`/`presetOption` presets, a Default preset, upload, and delete.
  Table icon and image columns upload into `iconDir` and show thumbnails.
  Uploads and deletes are staged with `git add`.
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

## Remaining differences

- Menus have the provider assistant and inline field help.
- Plugin rows render schema-driven QML controls and can add missing fields.
- Bar and dock sections seed rows from `@widget-preset` tiles and support add,
  remove, rename, move, and field edits.
- Grid cards drag, resize, and seed from `gridRegistry` ids; catalog sizes and
  profile columns come from `grid.metadata`.
- Wallpaper cycle settings are ordinary controls beside the wallpaper option.
- Secure Boot setup, rebuild, and reboot each require the device password and
  verify it through PAM in the root parent. Rebuild output streams line by line.
- Scan, save, apply, rebuild, Secure Boot rebuild, and update checks stream
  progress. `job.cancel` terminates the worker for any of them; the in-process
  dev engine cannot cancel.
- The service includes a `system-auth` PAM stack and inherits its MFA, faillock,
  LDAP, and Kerberos modules. Account expiry is covered by the VM test; the
  other modules depend on operator provisioning.
- Built-in TLS is out of scope for the local-only design; a remote reverse proxy
  is the operator's responsibility.
- `checks.ui-smoke` drives the real UI over CDP, and `checks.parity` guards the
  widget names.
- The flake pins `nix-settings-gui` to the GitHub revision `3f3a980`. Only the
  `binary-url.json` release manifest is undecided.

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
