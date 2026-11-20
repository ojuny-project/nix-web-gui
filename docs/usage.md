# Usage

The browser flow over the engine in `/home/den/repos/ojuny-future-repos/nix-web-gui/engine`.
In PAM mode every write is performed by the signed-in account's worker, so the
account must be able to read and write the flake directory. The page code is
`/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/app/src/pages/Shell.svelte`.

## Sign in

`/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/app/src/App.svelte`
calls `GET /api/me` on load. A `401` shows the login form in
`/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/app/src/pages/Login.svelte`.

- `POST /api/login` sends `{ "username": "...", "password": "..." }`.
- The server checks the device account through PAM service
  `services.nix-web-gui.pamService` (default `nix-web-gui`), then enforces
  `services.nix-web-gui.allowedGroups` when the list is not empty.
- Five failed attempts in five minutes lock the account for five minutes. A
  blocked attempt returns HTTP `429` with the remaining wait.
- A successful login sets the `nix_web_gui_session` cookie and returns
  `{ "username": "...", "csrf": "..." }`. The cookie is HttpOnly and
  SameSite=Strict. It is `Secure` when the request reports
  `X-Forwarded-Proto: https` or the service runs with `--secure-cookie`.
- The browser sends the CSRF token in `X-CSRF-Token` on every `POST /api/rpc`.
- Login, logout, and RPC reject an `Origin` that does not match the request
  `Host`. Requests without an `Origin` header are allowed for CLI clients.
- Failures are shown inline. The messages come from
  `/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/auth/mod.rs`:
  `invalid username or password`, `account is not in an allowed group`,
  `unknown device account`.
- The session lifetime is `services.nix-web-gui.sessionTtlSeconds`, default
  `43200` seconds. At most `services.nix-web-gui.maxSessions` sessions exist,
  default `16`; the oldest is evicted first. Expired sessions and stale workers
  are purged every five minutes.
- Sign out calls `POST /api/logout`, removes the session, and drops the worker.

## Open a flake

The open form is rendered while the menu list is empty. It posts
`session.open` with `{ "root": "<path>" }`. The sidebar `Open flake` button
prompts for a path at any time and opens it in the same way.

The initial path comes from `GET /api/state`:

1. `--flake` from the service, which `/home/den/repos/ojuny-future-repos/nix-web-gui/modules/nixos.nix`
   sets from `services.nix-web-gui.flakePath`.
2. `flake_path` from the service account's
   `$HOME/.config/nix-settings-gui/config.json`, including the fallback read from
   `/etc/ojuny/flake-path`. `GET /api/state` runs in the root parent, so this is
   the parent's config file, not the browser account's.
3. The `nix-web-gui.root` key in browser `localStorage`.

After `session.open` the shell calls `scan` and renders the sidebar. When
`auto_git_sync` is enabled it also calls `git.pull` first, and reports a pull
failure in the status bar without blocking the open. It then calls
`config.remember` with the opened path, so `flake_path` and the ten most recent
paths follow the last open.

`config.remember` also feeds the recent-path suggestions on the open form and
the `Open flake` prompt. Opening a new root replaces the current settings
session, so save or discard staged edits first.

## Search

The search box filters the current page on option `path`, `label`, and
`description`. It also filters the menu list on `displayname`, `description`,
and the same record fields.

When the search matches controls in any menu, a `Search results` panel lists up
to 40 matches. Selecting a result switches to that menu, scrolls the control
into view, and highlights it for about two seconds.

## Categories

The sidebar lists `Flake Inputs`, `Git`, then a `Menus` label, then menus grouped
by the `@menu category` value. A menu without a category appears before the
first category heading. Menu icons are rendered when the menu declares one, and
a category heading collapses and expands its group. A `.nix` file without an
`@menu` block is never shown as a page, and a menu with `hidden = true` appears
only after the `Advanced` checkbox is enabled.

## Host filter

The `System only` checkbox and its host select appear only when `scan` returns a
non-empty `hosts` list. Enabling the checkbox rescans with `{ "host": "..." }`,
which filters menus through the scan import graph. The host selector uses
`GET /api/state` hostname when that host is present, otherwise the first host
returned by the scan.

## Advanced controls

Records whose `advanced` flag is true are always collected under an `Advanced`
heading after the normal groups, matching the desktop expander. The `Advanced`
checkbox reveals records and menus whose `hidden` flag is true; hidden records
keep their normal group when revealed. Group headings come from the record
`group` and `groupOrder` values.

## Editing and staging

A control change updates the page value, marks the path dirty, and schedules
`control.setTyped` after 250 ms for that path. The request carries
`{ "path", "widget", "value" }`; the engine serializes the typed value to the
canonical Nix literal and stores it as a staged edit.

- Nothing staged in a control is written until Save. Composite row actions and
  media uploads or deletes write immediately and require a clean staged state.
- `visibleWhen` gates hide controls in the page. The engine also skips a staged
  child whose gate does not hold when it builds the edit list.
- Font, icon-theme, and cursor-theme selections can also stage the matching
  package into the record's `package_option`.
- When the record has an `applyCommand` and the config option `live_apply` is
  true, the shell queues `apply.run` with the new literal. One queue drains at a
  time and the latest value per path wins.
- Dirty controls show `Reset`, which calls `control.reset` and restores the
  scanned value.
- Closing or reloading the tab while any path is dirty triggers the browser's
  unsaved-changes prompt.

## Tables

A `table-attrset` or `table-list` with declared columns renders the table editor:

- `Add row`, `Remove`, `Up`, and `Down` keep the row order;
- hidden rows are withheld until `Show N hidden row(s)` is enabled;
- an attribute-set key is editable when `keyEditable` is true or the key is
  empty, and is a locked label otherwise;
- `table-list` keeps its first column static unless `firstValueEditable` is
  true, and shows the row label beside the actions;
- bool cells are checkboxes, dropdown cells use the table `choices`, and icon or
  image cells show a thumbnail and a `Pick` button that uploads into `iconDir`.

Unknown columns are not rendered, but their existing values are written back
unchanged, so they are not dropped from a table edit. `@table-item` directives
that carry row labels, hidden flags, and `visibleWhen` gates are preserved.

## Composite rows

Composite widgets render as row cards through
`/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/app/src/pages/CardCollection.svelte`.
A `submodule` parent instead renders as a heading with ordinary child controls.
Each row card holds the row's fields, and the card header offers:

- `Add row` for users, keybinds, monitors, docker, menu, horizontal-submodule,
  grid-configuration, and structured `list` sections;
- `Rename` for users, keybinds, monitors, docker, menu, and
  grid-configuration;
- `Remove` for those parents, submodule rows, and structured `list` sections;
- `Up` and `Down` on structured `list` rows, such as bar and dock sections.

Keybind rows use `bindFields`, `bindFieldTypes`, `bindPreview`, `bindSort`, and
`bindKeyField`. Fields typed `list` render as comma-separated inputs. Docker rows
use the configured `containerImage`, `containerPorts`, `containerVolumes`,
`containerEnvironment`, `containerAutoStart`, and `containerAutoStartDefault`
names; empty container fields are hidden. `disabledFeatures` hides matching
child fields, and the engine skips their staged edits on save.

Monitors render output cards on a draggable canvas that updates `xPos` and
`yPos`. Adding a monitor can prefill the name, width, height, and refresh from
`monitor.detect`. Grid cards use the placement canvas with `col`, `row`, `w`,
and `h`, drag and resize handles, and `gridRegistry` ids as add-row choices. The
bar and dock add dialogs list `@widget-preset` tiles and write the preset
defaults into the new row. The plugins add dialog lists uninstalled DMS registry
entries.

Adding, renaming, moving, or removing a user also updates the matching
`home-manager.users` entry when that sibling block exists. These writes happen
immediately. Save or discard staged edits first.

Before save, each users widget is validated: account names, unique positive
UIDs, `pkgs.*` shells, and at least one normal sudo account with a password.

## Diff

`Diff` calls `edits.preview` and renders one block per staged edit:

```text
--- <file>
- <previous span>
+ <new literal>
```

Nothing is written by the preview. The Git page has a separate working-tree
diff from `git.diff`.

## Discard

`Discard` confirms when at least one path is dirty, then calls
`session.discard`, rescans, and reloads the page. The engine clears the staged
map and rescans the root, so the next save has nothing to write.

## Save

`Save` is disabled while nothing is dirty and while a request is running. It
calls `edits.apply`, which:

1. builds the edit list from the staged values, skipping readonly records,
   records whose `visibleWhen` gate does not hold, and records under a disabled
   feature;
2. validates staged package expressions with nixd; missing nixd infrastructure
   does not block the save, real diagnostics do;
3. validates every users widget;
4. rejects the save when any target file changed on disk after the scan;
5. writes every file or writes none;
6. clears the staged map and rescans.

The status bar reports the number of files written. When `auto_git_sync` is on,
the shell then calls `git.status`, prompts for a commit message when the tree is
dirty and the branch state is `current`, and calls `git.sync`. Other states are
reported as `Saved; git is <state>`.

## Live apply

`live_apply` defaults to true in
`/home/den/repos/ojuny-future-repos/nix-web-gui/engine/src/config.rs`. With it
enabled, a staged change to a record with an `applyCommand` runs that command in
the flake root. The command receives the plain value, not the Nix literal. The
client apply queue runs one command at a time.

## Save and rebuild

`Ctrl+S` or `Cmd+S` saves. `Ctrl+Shift+S` or `Cmd+Shift+S` saves and then calls
`rebuild.run`. The toolbar Save & rebuild button starts it after a successful
save. Output lines stream into the Rebuild output panel, and Cancel is available
while the command runs.

`rebuild.run` runs `rebuild_command` from the shared config in the flake root,
appending `mmsg dispatch reload_config` and `dms kill && mmsg dispatch spawn_shell,dms run`
when the matching reload options are set. The command runs as the signed-in
account, so the account's sudo configuration applies. There is no password
re-entry and no `sudo -S`. The response also carries the full output after the
command exits. The `terminal_command` config field is ignored by the web path.

Scan, save, and live apply send a single stage line to the status bar. They do
not stream line-level output and have no Cancel action.

## Add file

The panel creates a file from `file.templates` or from a built-in `raw`,
`nixos`, or `home` template:

- `path` defaults to `<root>/modules/module.nix`;
- `moduleName` fills `flake.modules.<kind>.<name>`;
- template placeholders are rendered before the write;
- `file.add` writes the file, reloads, and returns a reminder when `flake.nix`
  exists without `import-tree`.

The sidebar `Add file` button opens the panel.

## Add option

`Add option` opens a panel for the current menu file. It sends `option.add` with
the file, origin, option path, widget, label, and initial value. The engine
inserts an `@option` block directly above the new assignment.

The widget list is limited to the `optionWidgets` array in `Shell.svelte`:
`switch`, `dropdown`, `radio`, `spin`, `slider`, `text`, `multiline`, `path`,
`package`, `packages`, `list`, `color`, `submodule`, `users`, `facter`,
`docker`, and `raw`. Table, keybind, monitor, theme, wallpaper, file, secureboot,
menu, plugins, bar, dock, and grid-configuration widgets cannot be authored from
this panel.

## Insert snippet

When the current menu has `@insert` markers with matching `@snippet`
definitions, a button per marker opens a placeholder form. `insert.apply` sends
the file, marker name, marker offset, and placeholder values. The engine renders
the snippet and inserts it at the marker, then reloads.

## Flake inputs

`Flake Inputs` in the sidebar calls `flake.inputs` and shows:

- a nixpkgs card with the declared URL and the first 12 characters of the locked
  revision, with `Check`, `Refresh`, and `Edit` actions;
- a `Missing inputs` banner when an `@inputs` marker declares an input absent
  from `flake.nix`, with `Declare all` calling `flake.input.addAll`;
- the remaining top-level inputs with name, URL, and description.

Each input has `Check` (`flake.update.check`) and `Update` (`flake.update`).
`Check` streams one status line per input and the page offers `Cancel check`
while it runs. `Edit` opens the same form used for adding an input. The name is
fixed while editing. `Save` calls `flake.input.upsert` with `flake: true` and
reloads the session. `Delete` confirms and calls `flake.input.delete`.

The page has no channel scan or reference suggestion UI. `flake.channels` and
`flake.suggest` exist in the RPC layer.

## Git

The `Git` page calls `git.status` and renders the branch, upstream, dirty or
clean state, ahead and behind counts, and the remote state. It offers:

- `Refresh`, `Pull` (`git.pull --ff-only`), `Push` (`git.push`), and `Sync`
  (`git.sync`, which only commits and pushes when the branch is current);
- a commit message form and `Commit` (`git.commit`), which stages every change
  and commits without pushing; it is disabled while the tree is clean;
- `Show diff` (`git.diff`), which displays the short status list plus the
  unstaged working-tree patch. Untracked files appear only in the status block.

Automatic Git behaviour is config-driven: `git.pull` on open and
commit-and-push after save when `auto_git_sync` is enabled.

## Media picker

For a `wallpaper` or `file` record, a media grid is rendered under the control:

- `preview.list` returns files from `wallpaper_dir` or `file_dir` relative to
  the module directory, filtered by the widget's extension list;
- the first 12 image files are loaded through `preview.read` and shown as data
  URLs;
- clicking a file stages its relative `value` through the normal change path;
- `Choose file` uploads through `preview.upload`. Uploads are limited to 32 MiB
  and to the widget's accepted extensions;
- `Delete` calls `preview.delete` after a confirmation;
- `Refresh` reloads the listing.

A wallpaper option with `presetSource` shows a Presets section generated from
that flake input, using `presetOption` for the metadata eval and previews from
the generated files; choosing a preset stages its name. The picker also offers a
`Default preset` choice, which stages `null`. Wallpaper images accept `png`,
`jpg`, `jpeg`, `webp`, `svg`, and `gif`. File widgets accept those image types
plus `pdf`, `txt`, and `md`, or the extensions declared on the option. Table icon
and image columns use a file picker that uploads into `iconDir`, shows a
thumbnail, and stages the returned file name in the cell. Uploads and deletes
write directly to the media directory and run `git add -A -- <path>` on the
changed file, so the next commit includes them.

## Password action

Any record whose path ends in `.hashedPassword` gets a `Set password` button.
The shell prompts for the password twice, calls `users.hashPassword`, and stages
the returned hash into the field through `control.setTyped`. The hash is
produced by `mkpasswd -m yescrypt -s`, with `openssl passwd -6 -stdin` as a
fallback, and the password is passed on stdin.

New users created from the structure editor start with
`hashedPassword = null`, so the password action can fill it. Save then requires
at least one normal sudo account with a password.

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl+S`, `Cmd+S` | Save |
| `Ctrl+Shift+S`, `Cmd+Shift+S` | Save and rebuild |
| `Ctrl+F`, `Cmd+F` | Focus the search box |
| `Ctrl+R`, `Cmd+R` | Discard |
| `Escape` | Close the add-file, add-option, config, add-row, structure, and insert panels |

These bindings are global while the shell is mounted.

## Config panel

The `Configure` panel edits the shared desktop config file at
`$XDG_CONFIG_HOME/nix-settings-gui/config.json`, default
`$HOME/.config/nix-settings-gui/config.json`:

| Field | Effect |
|---|---|
| Flake path | `flake_path`, the initial open and the first recent path |
| Rebuild command | Command run by `rebuild.run` |
| Apply changes immediately | `live_apply` |
| Sync git on open and save | `auto_git_sync` |
| Expand setting details | `expand_details_by_default` |
| Reload mango after rebuild | Appends the mango reload to the rebuild command |
| Reload DMS after rebuild | Appends the DMS reload to the rebuild command |

`Save config` sends the whole object to `config.set`. The shell applies
`live_apply`, `auto_git_sync`, and `expand_details_by_default` immediately. The
panel does not show `terminal_command`; the stored value is preserved for the
desktop app.

The sidebar `Settings` button opens the panel. Opening a flake also calls
`config.remember`, so `flake_path` and `recent_flake_paths` update without the
panel.

## Secure Boot

A `secureboot` record renders a raw Nix text box plus the Secure Boot panel. The
panel reads `secureboot.status` and shows firmware mode, UEFI, setup mode,
sbctl, key bundle, keys present and enrolled, Lanzaboote state, signed and
unsigned files, Windows, BitLocker, ESP free space, warnings, and the derived
stage title and detail.

Actions:

- `Prepare or enroll keys` calls `secureboot.setup` with `keep`, `create`, or
  `migrate`, plus the `Include Microsoft keys` and `Include firmware builtin
  keys` options. `create` asks for confirmation first.
- `Write configuration` calls `secureboot.writeConfig` and inserts only the
  missing Lanzaboote assignments into the module that owns the toggle.
- `Rebuild` calls `secureboot.rebuild`.
- `Reboot` calls `secureboot.reboot` with `confirm: true` after confirmation.
- `Show config block` calls `secureboot.config`.

Key setup, rebuild, and reboot run in the root parent and fail with a clear
message outside it. Each action requires the device password again and verifies
it through PAM. Firmware steps, including resetting to Setup Mode and enabling
Secure Boot, stay manual.

## Theme and the minimal build

- The header and footer show the active base16 scheme name. The server serves it
  at `/api/theme` and pushes `theme.changed` on `/api/events`; the page applies
  the new palette without a reload. By default the name comes from
  `/etc/nix-web-gui/theme`.
- The theme control is a searchable swatch picker instead of a plain select. The
  search matches the scheme id, name, and author.
- A font control lists installed fonts, filters to monospace when the option
  asks for it, and renders a live preview sample.
- An icon-theme control filters by the option `variant`; icon and cursor
  controls show preview thumbnails from `catalog.styles`.
- A `packages.nix-web-gui-minimal` build sets `VITE_OJUNY_WIDGETS=0`. The shell
  hides the descendants of an Ojuny record and shows
  `This Ojuny widget is not available in the minimal build.` in its place.
- `?popout=<page>` is read by `App.svelte`, passed to `Shell.svelte`, hides the
  sidebar and footer, and selects the matching menu.
