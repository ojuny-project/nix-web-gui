# Widgets

Every `WidgetKind` in the engine is emitted as a kebab-case `widget` string on a
control descriptor. The web page picks a renderer from that string in
`/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/ui/src/lib/components/Control.svelte`.

## How the renderer is chosen

- `/home/den/repos/ojuny-future-repos/nix-web-gui/engine/src/render.rs`
  `choose_widget` takes the `@option widget` value first. It turns a `spin` with
  both `min` and `max` and `horizontal` into `slider`, and turns `list` or
  `packages` whose value is not a literal list into `raw`.
- Without an explicit widget, an `imports` assignment whose origin is
  `modules.nixos.` or `modules.homeManager.` becomes `nixos-modules` or
  `hm-modules`.
- Next it derives a widget from the metadata type string, then from the scanned
  value kind.
- `widget_name` converts `format!("{widget:?}")` to kebab case: each uppercase
  character starts a new segment. `TableAttrset` becomes `table-attrset`,
  `TableList` becomes `table-list`, `IconTheme` becomes `icon-theme`,
  `CursorTheme` becomes `cursor-theme`, `HorizontalSubmodule` becomes
  `horizontal-submodule`, `GridConfiguration` becomes `grid-configuration`,
  `NixosModules` becomes `nixos-modules`, and `HmModules` becomes `hm-modules`.
- `control.setTyped` parses the submitted name with `WidgetKind::parse`, which
  accepts both the kebab-case and the concatenated spellings. The descriptor
  names and the parser inputs are now symmetric for every kind.

Dispatch order in `Control.svelte`:

1. `switch`
2. `radio`
3. `theme`
4. `dropdown`, `facter`, `font`, `icon-theme`, `cursor-theme`
5. `spin`, `slider`
6. `multiline`, `raw`
7. `list`, `packages`, `nixos-modules`, `hm-modules`
8. `color`
9. minimal build notice for an Ojuny kind
10. `table-attrset` or `table-list` with at least one declared column
11. the `rawWidgets` set, rendered as a textarea
12. everything else, rendered as a single-line text input

`rawWidgets` contains `raw`, `submodule`, `horizontal-submodule`, `table-attrset`,
`table-list`, `keybinds`, `monitors`, `users`, `secureboot`, `menu`, `plugins`,
`bar`, `dock`, `grid-configuration`, `docker`, and `mounts`.

## Widget table

| WidgetKind | Engine `widget` string | Web renderer |
|---|---|---|
| `Switch` | `switch` | `Switch.svelte` |
| `Dropdown` | `dropdown` | `Select.svelte` with `choices` |
| `Radio` | `radio` | `Radio.svelte` |
| `Spin` | `spin` | `Number.svelte` |
| `Slider` | `slider` | `Number.svelte` with `range` |
| `Text` | `text` | `Text.svelte` |
| `Multiline` | `multiline` | `Textarea.svelte` |
| `Path` | `path` | `Text.svelte`, no path picker |
| `Package` | `package` | `Text.svelte`; staged values are validated with nixd before save |
| `Packages` | `packages` | `List.svelte` with `packages` |
| `List` | `list` | `List.svelte` for plain strings; structured lists (attribute-set elements) are described as child controls |
| `NixosModules` | `nixos-modules` | `List.svelte` |
| `HmModules` | `hm-modules` | `List.svelte` |
| `Color` | `color` | `Color.svelte` |
| `Submodule` | `submodule` | Generic child controls; textarea raw when it has no children |
| `HorizontalSubmodule` | `horizontal-submodule` | Generic child controls; textarea raw when no children; `maxColumns` sets the card grid width |
| `GridConfiguration` | `grid-configuration` | `GridEditor.svelte`: canvas cards sized by `col`, `row`, `w`, and `h`, registry add flow, catalog sizes, and a selected-card settings panel |
| `TableAttrset` | `table-attrset` | `Table.svelte` with raw-expression cells, icon delete, and typed columns |
| `TableList` | `table-list` | `Table.svelte` with raw-expression cells, icon delete, and typed columns |
| `Keybinds` | `keybinds` | `BindsEditor.svelte`: one compact row per bind with title, description, preview, Edit, and Remove; a focused edit form for the configured bind fields |
| `Monitors` | `monitors` | `DisplayEditor.svelte`: scaled canvas, selected-output details, primary marker, transform dropdown with rotated tile preview, grouped feature fields, and output detection |
| `Theme` | `theme` | `ThemePicker.svelte` with search and swatches |
| `Wallpaper` | `wallpaper` | `Text.svelte` plus `MediaPicker.svelte` with `presetSource`/`presetOption` presets, a Default preset, upload, and delete |
| `File` | `file` | `Text.svelte` plus `MediaPicker.svelte` with upload and delete |
| `Menu` | `menu` | `MenuEditor.svelte`: provider assistant, compact rows, inline field help, and selected-row editing |
| `Plugins` | `plugins` | Row cards; the add dialog lists uninstalled DMS registry plugins; `plugins.settings` returns QML but no QML controls render |
| `Bar` | `bar` | `BarDockEditor.svelte`: per-section tiles with enable switches, preset picker, drag reorder, and selected-tile settings |
| `Dock` | `dock` | `BarDockEditor.svelte`: per-section tiles with enable switches, preset picker, drag reorder, and selected-tile settings |
| `Font` | `font` | `Select.svelte` from `catalog.styles`; monospace filtering and a live font preview |
| `IconTheme` | `icon-theme` | `Select.svelte` from `catalog.styles` plus preview thumbnails; variant filtering |
| `CursorTheme` | `cursor-theme` | `Select.svelte` from `catalog.styles` plus preview thumbnails |
| `Users` | `users` | Account cards with add, remove, rename, validation, password action, and `home-manager.users` sync |
| `Facter` | `facter` | `Select.svelte` with `auto` plus profile choices read from `profileCatalog`, falling back to the built-in catalog |
| `Docker` | `docker` | Row cards with configurable container field names; empty container fields are hidden and add, remove, and rename work |
| `Mounts` | `mounts` | Row cards over a mountpoint-keyed `fileSystems` set; each row edits device, filesystem, options, and neededForBoot; add, remove, and rename work; protected and duplicate mountpoints are rejected on save |
| `SecureBoot` | `secureboot` / `secure-boot` | `SecureBootFlow.svelte` with the full staged wizard, reauthentication, and streamed rebuild output |
| `Raw` | `raw` | `Textarea.svelte` |

There is no `Icon` widget kind. `icon` and `image` exist only as table column
types. `/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/app/src/pages/Shell.svelte`
still tests `record.widget === "icon"` before rendering `MediaPicker.svelte`, but
no engine descriptor carries that name. Icon-column uploads go through the
table's `onIconPick` callback instead.

## Table editor

`/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/ui/src/lib/controls/Table.svelte`
renders `table-attrset` and `table-list` from the descriptor's `table` object.
The table keeps row order in the submitted array.

- Hidden rows are filtered out until `Show N hidden row(s)` is enabled. New rows
  are added visible.
- An attribute-set row shows a key cell. The key is editable when
  `keyEditable` is true or the row key is empty; otherwise it renders as a
  locked label using `row.label || row.key`.
- For `table-list` the first column is static when `firstValueEditable` is
  false. The row label appears beside the row actions when one is declared.
- `bool` columns render as checkboxes, `dropdown` columns as a select when the
  table has `choices`, and `icon` or `image` columns as a thumbnail plus input
  and a `Pick` button that uploads through `preview.upload`.
- Every row can move up or down and can be removed. `Add row` appends a row.
- The editor submits `{ "rows": [...] }` through `control.setTyped`. The engine
  serializer writes quoted keys for attribute sets and bare attribute-set
  entries for lists.
- Columns not declared in `columns` are not rendered. The serializer writes the
  declared columns first and then any undeclared fields it received, so a row
  edit does not drop them. `@table-item` directives that carry row labels,
  hidden flags, and `visibleWhen` gates are preserved on save.

## Composite row editor

`page.describe` adds a `children` array when a record is a composite widget or a
structured list, up to depth 6. The composite kinds in
`/home/den/repos/ojuny-future-repos/nix-web-gui/engine/src/session.rs`
`is_composite_widget` are `Submodule`, `HorizontalSubmodule`, `Users`,
`Monitors`, `Docker`, `Mounts`, `Menu`, `Bar`, `Dock`, `GridConfiguration`, `Plugins`, and
`Keybinds`.

`is_structured_list` adds children for a `list` whose elements include an
attribute set, such as a bar or dock section. A list of plain strings keeps the
`List.svelte` editor.

`Shell.svelte` renders composite records through
`/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/app/src/pages/CardCollection.svelte`.
A `submodule` parent is rendered as a heading and its described children are
ordinary controls. For the other composite kinds:

- the parent heading, then one row card per child, with the child's own widget
  kind for each field;
- `Add row` when the parent is `users`, `keybinds`, `monitors`, `docker`, `mounts`,
  `menu`, `horizontal-submodule`, `grid-configuration`, or a structured `list` section;
- `Rename` on a row whose parent is one of those widgets, except
  `horizontal-submodule`;
- `Remove` on a row whose parent is one of those widgets, `submodule`, or a
  structured `list`;
- `Up` and `Down` on a structured `list` row;
- monitor records also render `MonitorCanvas.svelte`, and grid records render on
  the placement canvas with drag and resize handles.

Row add, remove, rename, and move are written immediately and are refused while
staged edits exist. `structure.add`, `structure.remove`, `structure.rename`, and
`structure.move` are the RPC methods. User rows also add, remove, or rename the
matching `home-manager.users` entry when that sibling block exists. A `submodule`
row can be removed but not added or renamed from the page; a
`horizontal-submodule` row can be added and removed but not renamed.

The parent value itself is not editable in this mode, only its children. A
composite kind with no children falls through to the raw textarea.

Remaining differences from the desktop layouts are the menu provider assistant,
plugin QML controls, and the bar and dock catalogs beyond the
`@widget-preset` tiles. See
`/home/den/repos/ojuny-future-repos/nix-web-gui/TODO.md`.

## Raw fallback behaviour

Structured kinds serialize their submitted value verbatim. In
`/home/den/repos/ojuny-future-repos/nix-web-gui/engine/src/render.rs`
`serialize`, every kind outside the typed arms returns the submitted string
unchanged, so a raw textarea can carry an arbitrary Nix expression.

`TableAttrset` and `TableList` require `{ "rows": [...] }`. A table with no
declared columns falls back to the raw textarea, and that string is rejected by
the serializer with `table has no columns`.

## Minimal build

`packages.nix-web-gui-minimal` is built with `withOjunyWidgets = false`. The
frontend receives `VITE_OJUNY_WIDGETS=0`, and
`/home/den/repos/ojuny-future-repos/nix-web-gui/frontend/packages/ui/src/lib/features.ts`
then treats these kinds as unavailable:

`menu`, `plugins`, `bar`, `dock`, `grid-configuration`, `docker`.

Because the engine emits those exact kebab-case names, the notice
`This Ojuny widget is not available in the minimal build.` matches every kind.
The shell filters every descendant of an Ojuny record and shows the notice in
place of the parent record.

`vite.config.ts` maps `@ojuny/env-editor` and `@ojuny/plugin-settings` to a stub
when the flag is `0`, so the Docker environment editor and the plugin settings
editor are not bundled. `checks.minimal-renderer` enforces that absence.

The backend flag is independent: `system.capabilities` reports `ojunyWidgets`
from the server feature `ojuny-widgets`.
