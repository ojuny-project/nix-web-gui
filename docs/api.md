# HTTP API

The browser talks to the server over JSON-RPC 2.0. The server maps each method
to an `engine` call. Method names and error codes follow the existing
`nix-settings-backend` envelope where it already defines them.

## Transport

- `POST /api/rpc` with a JSON-RPC request body.
- The request needs the session cookie and the `X-CSRF-Token` header.
- `POST /api/login`, `POST /api/logout`, and `POST /api/rpc` reject an `Origin`
  that does not match the request `Host`. A missing `Origin` is allowed for CLI
  clients.
- `GET /api/events` is the authenticated SSE stream for theme changes, progress
  lines, and heartbeats. Progress covers scan, save, apply, update checks, and
  rebuild. `job.cancel` terminates the session worker that is running a job.
- `GET /api/theme` returns the active base16 name.
- `GET /api/health` returns `{ "status": "ok", "version": "<crate version>" }`
  and needs no session.
- `GET /api/state` returns the startup flake, recent flakes, hostname,
  authentication mode, startup page, and theme.
- The browser sits behind the same origin as the API. TLS is terminated by a
  reverse proxy.
- Every response carries a strict CSP, `X-Content-Type-Options`,
  `X-Frame-Options`, `Referrer-Policy`, `Permissions-Policy`, and
  `Cross-Origin-Opener-Policy`.

The request envelope is:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "page.list",
  "params": {}
}
```

`id` is echoed unchanged and defaults to `null` when omitted. `params` defaults
to `null`. The response envelope is either `result` or `error`:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {}
}
```

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "no open session"
  }
}
```

`system.ping`, `system.capabilities`, `secureboot.setup`, `secureboot.rebuild`,
and `secureboot.reboot` are answered by
`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/http.rs`. Every other
method is answered by
`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/rpc.rs`, either in
process or in the signed-in account's worker. The three privileged Secure Boot
methods always run in the root parent.

## Authentication

| Method | Body | Result |
|---|---|---|
| `GET /api/me` | none | `{ username, csrf }` or `401` |
| `GET /api/state` | none | `{ flake, configFlake, recentFlakes, hostname, page, auth, theme }` or `401` |
| `POST /api/login` | `{ username, password }` | sets the session cookie, returns `{ username, csrf }` |
| `POST /api/logout` | none | clears the session and drops the worker |

Credentials are checked through PAM against the device account. The PAM service
is `nix-web-gui`. The account phase runs after authentication, and the
configured allowed groups are enforced when the list is non-empty.

Failed logins are tracked per username. Five failures in five minutes lock the
account for five minutes and return HTTP `429`. A successful login clears the
counter. The session cookie is HttpOnly and SameSite=Strict, with `Secure` set
when the request reports `X-Forwarded-Proto: https` or the server runs with
`--secure-cookie`.

`GET /api/state` returns:

```json
{
  "flake": "/home/den/repos/nixos-configs/dendrite",
  "configFlake": "/home/den/repos/nixos-configs/dendrite",
  "recentFlakes": ["/home/den/repos/nixos-configs/dendrite"],
  "hostname": "dendrite",
  "page": "Theme",
  "auth": "pam",
  "theme": {
    "name": "everforest-dark-hard",
    "source": "/etc/nix-web-gui/theme",
    "version": 1
  }
}
```

`flake` is the `--flake` value first, then `flake_path` from the shared config.
`page` is the `--page` value. `auth` is `pam` or `dev`.

`/api/state` is served by the root parent, so `configFlake` and `recentFlakes`
come from the service account's config file. `config.get` runs in the signed-in
account's worker and reads that account's file instead.

## Current methods

| Method | Status |
|---|---|
| `system.ping` | implemented |
| `system.capabilities` | implemented |
| `session.open` | implemented |
| `session.close` | implemented |
| `session.reload` | implemented |
| `session.discard` | implemented |
| `session.state` | implemented |
| `scan` | implemented, sends one stage progress line |
| `page.list` | implemented |
| `page.describe` | implemented; local metadata by default, nixd with `NIX_WEB_GUI_METADATA=1` |
| `control.get` | implemented |
| `control.describe` | implemented with nixd and eval fallback |
| `control.set` | implemented |
| `control.setTyped` | implemented |
| `control.reset` | implemented |
| `edits.preview` | implemented |
| `edits.apply` | implemented, sends one progress line per written file |
| `file.add` | implemented |
| `file.templates` | implemented |
| `option.add` | implemented |
| `insert.apply` | implemented |
| `structure.add` | implemented |
| `structure.remove` | implemented |
| `structure.rename` | implemented |
| `structure.move` | implemented for list rows |
| `structure.addField` | implemented |
| `monitor.detect` | implemented |
| `flake.inputs` | implemented |
| `flake.input.addAll` | implemented |
| `flake.input.upsert` | implemented |
| `flake.input.delete` | implemented |
| `flake.channels` | implemented |
| `flake.suggest` | implemented |
| `flake.update.check` | implemented, sends one line per input and can be cancelled |
| `flake.update` | implemented |
| `git.status` | implemented |
| `git.diff` | implemented |
| `git.pull` | implemented |
| `git.sync` | implemented |
| `git.commit` | implemented |
| `git.push` | implemented |
| `apply.run` | implemented, sends one stage progress line |
| `rebuild.run` | implemented, streams `progress` notes over `/api/events` |
| `job.cancel` | implemented, terminates the session worker |
| `catalog.styles` | implemented |
| `plugins.registry` | implemented |
| `plugins.settings` | implemented, returns `{ "qml": "..." }` |
| `secureboot.status` | implemented |
| `secureboot.config` | implemented |
| `secureboot.writeConfig` | implemented |
| `secureboot.setup` | implemented |
| `secureboot.rebuild` | implemented |
| `secureboot.reboot` | implemented |
| `users.hashPassword` | implemented |
| `preview.list` | implemented |
| `preview.read` | implemented |
| `preview.upload` | implemented, stages the new file with `git add` |
| `preview.delete` | implemented, stages the removal with `git add` |
| `preview.icon` | implemented |
| `config.get` | implemented |
| `config.set` | implemented |
| `config.remember` | implemented |
| `echo` | test helper |

## Method examples

The examples below are the shapes produced by
`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/rpc.rs`. Fields the
engine serializes as `null` are shown only where they matter.

### `session.open`

Opens a root and scans it. The session holds one open root at a time. `root` is
required.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "session.open",
  "params": { "root": "/home/alice/nwg-flake" }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "apiVersion": 1,
    "menus": 1,
    "records": 1
  }
}
```

### `page.list`

Lists pages, optionally filtered to a host from the scan import graph.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "page.list",
  "params": { "host": "dendrite" }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "pages": [
      {
        "id": "/home/alice/nwg-flake/modules/system.nix",
        "title": "System",
        "origin": "modules.system",
        "description": null,
        "category": null,
        "icon": null,
        "order": 1000,
        "systems": [],
        "hidden": false,
        "records": ["networking.hostName"]
      }
    ],
    "hosts": ["dendrite"]
  }
}
```

`id` is the menu file. `records` is a list of option paths, not descriptors.

### `page.describe`

Describes every control on a page with metadata. `file` is the menu file from
`page.list`.

The server uses the scanner's local metadata by default, so pages open without
waiting on nixd. Set `NIX_WEB_GUI_METADATA=1` in the service environment to use
nixd and the eval fallback. `NIX_WEB_GUI_SKIP_METADATA=1` forces local metadata
even when that variable is set.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "page.describe",
  "params": { "file": "/home/alice/nwg-flake/modules/system.nix" }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "controls": [
      {
        "path": "networking.hostName",
        "file": "/home/alice/nwg-flake/modules/system.nix",
        "value": "\"vm\"",
        "label": "Hostname",
        "description": "The host name of the machine.",
        "widget": "text",
        "choices": [],
        "list": null,
        "advanced": false,
        "hidden": false,
        "readonly": false,
        "type": "string",
        "depth": 0,
        "table": null,
        "visibleWhen": null
      }
    ]
  }
}
```

Composite widgets and structured lists carry a `children` array of descriptors
with the same shape. The engine stops expanding children at depth 6.

A table descriptor carries `{ list, columns, columnTypes, choices, keyEditable,
firstValueEditable, rows }`. Each row carries `{ key, label, hidden,
visibleWhen, fields }`.

### `control.setTyped`

Stages a typed value. The engine serializes it to the canonical Nix literal.
`widget` is optional; an unknown or missing name falls back to
`render::choose_widget` for the record.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "control.setTyped",
  "params": {
    "path": "networking.hostName",
    "widget": "text",
    "value": "vm2"
  }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "staged": true,
    "value": "\"vm2\""
  }
}
```

When the record has a `package_option` and the widget is `font`,
`icon-theme`, or `cursor-theme`, the matching package is also staged and the
response adds:

```json
{
  "packagePath": "fonts.packages",
  "packageValue": "nerd-fonts"
}
```

A table value is submitted as `{ "rows": [...] }`. `control.set` takes an
already rendered Nix literal in `value` and returns `{ "staged": true }`.

### `edits.preview`

Returns the staged edits without writing. `start` and `end` are byte offsets in
the file. `before` is `null` when the span cannot be read.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "edits.preview",
  "params": {}
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "result": {
    "edits": [
      {
        "file": "/home/alice/nwg-flake/modules/system.nix",
        "start": 120,
        "end": 124,
        "before": "\"vm\"",
        "after": "\"vm2\""
      }
    ]
  }
}
```

### `edits.apply`

Writes every staged edit, or nothing when a package fails nixd validation, a
users widget fails validation, or a target file changed on disk since the scan.
Edits under an ancestor `disabledFeatures` entry are skipped. It clears the
staged map and rescans on success.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "edits.apply",
  "params": {}
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "result": { "files": 1 }
}
```

### `structure.add`

Adds one row to a composite widget. `mode` is optional and used to prefill a
monitor row. `defaults` is an optional object of field names to Nix literals,
used by bar and dock preset tiles. The call is refused while staged edits exist.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "structure.add",
  "params": {
    "path": "users.users",
    "key": "bob",
    "mode": null,
    "defaults": {}
  }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "result": { "added": true }
}
```

For a user row, a matching `home-manager.users.<key>` entry is added when the
sibling block exists.

### `structure.remove`

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "structure.remove",
  "params": { "parent": "users.users", "child": "users.users.bob" }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "result": { "removed": true }
}
```

### `structure.rename`

Renames one attribute-set row. `key` is the new row name. A user rename also
renames the matching `home-manager.users` entry.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 9,
  "method": "structure.rename",
  "params": {
    "parent": "users.users",
    "child": "users.users.alice",
    "key": "alicia"
  }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 9,
  "result": { "renamed": true }
}
```

### `structure.move`

Moves a direct list element by `offset` positions. Only list rows can move;
attribute-set rows keep their key order.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 31,
  "method": "structure.move",
  "params": {
    "parent": "desktop.ojuny.bar.left",
    "child": "desktop.ojuny.bar.left.[1]",
    "offset": -1
  }
}
```

Response:

```json
{ "jsonrpc": "2.0", "id": 31, "result": { "moved": true } }
```

### `structure.addField`

Inserts a field into an attribute-set row, or updates it when present. Used by
schema-driven plugin settings.

```json
{
  "jsonrpc": "2.0",
  "id": 32,
  "method": "structure.addField",
  "params": {
    "parent": "desktop.ojuny.dux.plugins.clock",
    "field": "settings",
    "literal": "{ }"
  }
}
```

### `plugins.registry`

Returns the DMS plugin registry when the flake declares one.

```json
{ "jsonrpc": "2.0", "id": 33, "method": "plugins.registry", "params": {} }
```

`plugins.schema` takes `{ "id": "clock" }` and returns typed controls parsed
from that plugin's Settings.qml. `menu.providers` takes `{ "topLevel": true }`
and returns provider labels, values, and field names; `menu.fieldHelp` takes a
field name and returns its help text.

### `monitor.detect`

Returns compositor outputs for the add-monitor panel. An unavailable compositor
or `wlr-randr` yields an empty list.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "method": "monitor.detect",
  "params": {}
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "result": {
    "outputs": [
      {
        "name": "DP-1",
        "make": "Dell",
        "model": "U2720Q",
        "enabled": true,
        "width": 3840,
        "height": 2160,
        "refresh": 60.0
      }
    ]
  }
}
```

### `preview.list`

Lists media files for a `wallpaper`, `file`, or table icon/image option. `value`
is the path relative to the configured media directory.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 11,
  "method": "preview.list",
  "params": { "path": "desktop.ojuny.wallpaper" }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 11,
  "result": {
    "files": [
      {
        "name": "forest.png",
        "value": "forest.png",
        "path": "/home/alice/nwg-flake/wallpapers/forest.png"
      }
    ],
    "presets": [
      {
        "name": "everforest",
        "label": "Everforest",
        "path": "/nix/store/...-wallpapers/everforest.png",
        "preview": { "mime": "image/png", "data": "iVBORw0..." }
      }
    ],
    "presetError": null
  }
}
```

`presets` is populated only when the widget declares `presetSource`; it is
generated by the wallpaper metadata eval using `presetOption`. `presetError`
reports an eval failure without hiding the local files. The page adds a Default
preset choice that stages `null`; that choice is client-side and not part of this
response.

### `preview.read`

Returns one file as base64 with a MIME type. The path is confined to the open
root and, when `option` is supplied, to that option's media directory.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 12,
  "method": "preview.read",
  "params": {
    "path": "/home/alice/nwg-flake/wallpapers/forest.png",
    "option": "desktop.ojuny.wallpaper"
  }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 12,
  "result": {
    "mime": "image/png",
    "data": "iVBORw0KGgo..."
  }
}
```

### `preview.upload`

Writes one base64 file into the option's media directory. The name must be a
single file name with no directory separators, and it must not start with a dot.
The extension must be accepted by the widget. The limit is 32 MiB and an empty
file is rejected.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 13,
  "method": "preview.upload",
  "params": {
    "path": "desktop.ojuny.wallpaper",
    "name": "forest.png",
    "data": "iVBORw0KGgo..."
  }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 13,
  "result": {
    "name": "forest.png",
    "value": "forest.png",
    "path": "/home/alice/nwg-flake/wallpapers/forest.png"
  }
}
```

A table icon or image column uploads with the table option path and writes the
file into the configured `iconDir`. The new file is staged with
`git add -A -- <relative path>`.

### `preview.delete`

Removes one file from the option's media directory and stages the removal with
`git add -A -- <relative path>`.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 14,
  "method": "preview.delete",
  "params": { "path": "desktop.ojuny.wallpaper", "name": "forest.png" }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 14,
  "result": { "deleted": "forest.png" }
}
```

### `users.hashPassword`

Hashes a password for `users.users.<name>.hashedPassword`. The password travels
on stdin, never in argv.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 15,
  "method": "users.hashPassword",
  "params": { "password": "correct horse battery staple" }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 15,
  "result": {
    "hash": "$y$j9T$..."
  }
}
```

### `git.status`

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 16,
  "method": "git.status",
  "params": {}
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 16,
  "result": {
    "branch": "main",
    "upstream": "origin/main",
    "dirty": true,
    "ahead": 0,
    "behind": 0,
    "state": "current"
  }
}
```

`state` is `current`, `behind`, `diverged`, or `no-upstream`. A directory that
is not a git repository returns error `-32012`.

### `git.diff`

Returns the working-tree status block and the unstaged patch for the root.
Untracked files appear only in the status block.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 17,
  "method": "git.diff",
  "params": {}
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 17,
  "result": {
    "diff": "Status:\n M modules/system.nix\n\ndiff --git a/modules/system.nix b/modules/system.nix\n..."
  }
}
```

### `git.commit`

Stages every change under the root and commits. It does not push. `message` is
required.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 18,
  "method": "git.commit",
  "params": { "message": "Update settings" }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 18,
  "result": { "commit": "1a2b3c4" }
}
```

### `git.push`

Pushes the current branch to its configured upstream.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 19,
  "method": "git.push",
  "params": {}
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 19,
  "result": { "pushed": "origin/main" }
}
```

### `git.sync`

Fetches, refuses when the branch is behind or diverged, then commits and pushes.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 20,
  "method": "git.sync",
  "params": { "message": "Update settings" }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 20,
  "result": { "synced": true }
}
```

### `flake.inputs`

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 21,
  "method": "flake.inputs",
  "params": {}
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 21,
  "result": {
    "inputs": [
      {
        "name": "nixpkgs",
        "url": "github:nixos/nixpkgs/nixos-unstable",
        "flake": true,
        "submodules": false,
        "lfs": false,
        "follows": {},
        "description": null
      }
    ],
    "nixpkgs": {
      "name": "nixpkgs",
      "url": "github:nixos/nixpkgs/nixos-unstable",
      "flake": true,
      "submodules": false,
      "lfs": false,
      "follows": {},
      "description": null
    },
    "locked": {
      "type": "github",
      "owner": "nixos",
      "repo": "nixpkgs",
      "ref": "nixos-unstable",
      "rev": "0123456789abcdef0123456789abcdef01234567",
      "lastModified": 1720000000,
      "narHash": "sha256-..."
    },
    "missing": []
  }
}
```

`nixpkgs` and `locked` are `null` when the input or lock entry is missing.
`missing` lists `@inputs` declarations absent from `flake.nix`.

### `flake.input.addAll`

Declares every missing `@inputs` entry and reloads the session.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 22,
  "method": "flake.input.addAll",
  "params": {}
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 22,
  "result": { "added": ["nix-settings-gui"] }
}
```

### `config.get`

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 23,
  "method": "config.get",
  "params": {}
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 23,
  "result": {
    "flake_path": "/home/alice/nwg-flake",
    "terminal_command": null,
    "rebuild_command": "sudo nixos-rebuild switch --flake .#host",
    "recent_flake_paths": ["/home/alice/nwg-flake"],
    "live_apply": true,
    "reload_mango_on_rebuild": false,
    "reload_dms_on_rebuild": false,
    "expand_details_by_default": false,
    "auto_git_sync": false
  }
}
```

`$XDG_CONFIG_HOME/nix-settings-gui/config.json`, default
`$HOME/.config/nix-settings-gui/config.json`. `flake_path` falls back to
`/etc/ojuny/flake-path` when unset.

### `config.set`

The params are the whole config object, not a wrapper. Missing fields fall back
to the serde defaults.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 24,
  "method": "config.set",
  "params": {
    "flake_path": "/home/alice/nwg-flake",
    "terminal_command": null,
    "rebuild_command": "sudo nixos-rebuild switch --flake .#host",
    "recent_flake_paths": ["/home/alice/nwg-flake"],
    "live_apply": true,
    "reload_mango_on_rebuild": false,
    "reload_dms_on_rebuild": false,
    "expand_details_by_default": true,
    "auto_git_sync": true
  }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 24,
  "result": { "saved": true }
}
```

### `config.remember`

Sets `flake_path`, moves the flake to the front of `recent_flake_paths`,
truncates the list to 10, and saves. The page calls it when a flake opens.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 25,
  "method": "config.remember",
  "params": { "flake": "/home/alice/nwg-flake" }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 25,
  "result": { "remembered": true }
}
```

An empty `flake` returns error `-32010`.

### `apply.run`

Runs one record's `applyCommand` with the plain new value. The path must exist
in the session and the record must declare `applyCommand`.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 26,
  "method": "apply.run",
  "params": {
    "path": "desktop.ojuny.style",
    "value": "\"everforest-dark-hard\""
  }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 26,
  "result": { "applied": true }
}
```

`value` is the Nix literal. The engine converts it to a plain value before
substituting it into the command.

### `rebuild.run`

Runs `rebuild_command` from the shared config in the flake root, with the
configured compositor reloads appended.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 27,
  "method": "rebuild.run",
  "params": {}
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 27,
  "result": {
    "output": "building the system configuration...\n"
  }
}
```

The call returns when the command exits. While it runs, `/api/events` sends one
`progress` event per output line with `params.job = "rebuild"`. The response
contains the full combined output. An optional `password` is sent to `sudo -S`
when the configured command uses sudo. On failure the error message is the last
non-empty output line.

### `job.cancel`

Stops the worker running a long job. The browser cookie stays valid; the next RPC
starts a fresh worker, so the flake must be reopened.

Request:

```json
{ "jsonrpc": "2.0", "id": 28, "method": "job.cancel", "params": {} }
```

Response:

```json
{ "jsonrpc": "2.0", "id": 28, "result": { "cancelled": true } }
```

### `secureboot.status`

Returns the live state and the derived setup stage.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 28,
  "method": "secureboot.status",
  "params": { "configured": true }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 28,
  "result": {
    "state": {
      "firmware_mode": "Setup",
      "efi": true,
      "setup_mode_enabled": true,
      "sbctl_installed": true,
      "key_bundle": "/var/lib/sbctl",
      "keys_present": true,
      "keys_enrolled": false,
      "lanzaboote_active": true,
      "signed_files": 0,
      "unsigned_files": 2,
      "windows_present": false,
      "bitlocker_present": false,
      "esp_free_bytes": 123456789,
      "errors": []
    },
    "stage": {
      "title": "Enroll keys",
      "detail": "..."
    }
  }
}
```

### `secureboot.config`

Returns the Lanzaboote configuration block as text.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 29,
  "method": "secureboot.config",
  "params": {}
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 29,
  "result": { "block": "imports = [ inputs.lanzaboote.nixosModules.lanzaboote ];\n..." }
}
```

### `secureboot.writeConfig`

Inserts only the missing Lanzaboote assignments into the module that owns the
Secure Boot toggle. Existing assignments are left untouched.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 30,
  "method": "secureboot.writeConfig",
  "params": {
    "file": "/home/alice/nwg-flake/modules/secureboot.nix",
    "origin": "modules.secureboot"
  }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 30,
  "result": { "written": ["imports = [ inputs.lanzaboote.nixosModules.lanzaboote ];"] }
}
```

### `secureboot.setup`

Creates or migrates keys and enrolls them. `keyAction` is `keep`, `create`, or
`migrate`. `microsoft` defaults to true and `firmwareBuiltin` to false. This
method runs in the root parent and fails outside it.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 31,
  "method": "secureboot.setup",
  "params": {
    "keyAction": "create",
    "microsoft": true,
    "firmwareBuiltin": false
  }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 31,
  "result": {
    "output": ["$ sh -c 'test ! -d /var/lib/sbctl/keys && sbctl create-keys'", "..."],
    "stage": { "title": "Rebuild NixOS", "detail": "..." }
  }
}
```

### `secureboot.rebuild`

Runs the configured rebuild command in the root parent. `root` is required.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 32,
  "method": "secureboot.rebuild",
  "params": { "root": "/home/alice/nwg-flake" }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 32,
  "result": { "output": "building the system configuration...\n" }
}
```

### `secureboot.reboot`

Reboots the machine. It requires `confirm: true`.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 33,
  "method": "secureboot.reboot",
  "params": { "confirm": true }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 33,
  "result": { "rebooting": true }
}
```

## Error shapes

HTTP errors from `/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/http.rs`
are plain objects, not JSON-RPC:

```json
{ "error": "not signed in" }
```

| Status | Meaning |
|---|---|
| `400` | missing or empty login fields |
| `401` | not signed in, expired session, or failed login |
| `403` | missing or invalid CSRF token, or a cross-origin POST |
| `429` | too many failed logins; the account is locked out |
| `502` | worker spawn or worker transport failure |

A JSON-RPC error keeps the request `id`:

```json
{
  "jsonrpc": "2.0",
  "id": 34,
  "error": {
    "code": -32011,
    "message": "unknown option path: networking.nope"
  }
}
```

| Code | Meaning |
|---|---|
| `-32000` | no open settings session |
| `-32010` | scan failed, or a required path is missing |
| `-32011` | unknown option path, unknown page, media failure, or serialization failed |
| `-32012` | edit, git, apply, rebuild, config, structure, or Secure Boot action failed |
| `-32013` | rescan failed after `session.reload`, `session.discard`, or a flake input change |
| `-32601` | unknown method |
| `-32700` | parse error |

`secureboot.setup`, `secureboot.rebuild`, and `secureboot.reboot` require a
`password` parameter for the signed-in device account and also use `-32012` for
a validation, authentication, or privilege failure.

Methods that do not need an open session are `echo`, `system.ping`,
`system.capabilities`, `catalog.styles`, `secureboot.status`,
`secureboot.config`, `secureboot.setup`, `secureboot.rebuild`,
`secureboot.reboot`, `users.hashPassword`, `config.get`, `config.set`,
`config.remember`, `monitor.detect`, `flake.channels`, and `flake.suggest`.
Every other method returns `-32000` before `session.open`.

## Implemented surface

| Group | Status |
|---|---|
| Pages | `page.describe`; scan sends a start line, the menu count, and one line per menu file |
| Edits | stale-file errors name the changed files; save sends one line per written file |
| Authoring | add file, add option, snippets, and structure edits |
| Flake | channels, reference suggestions, add-all, check, and update |
| Git | status, diff, pull, push, commit, sync |
| Apply | streamed and cancellable in worker mode |
| Secure Boot | per-action PAM re-authentication; rebuild output streams |
| Previews | wallpaper presets, media upload, delete, and selection |
| Catalogs | `catalog.styles`, `grid.metadata`, `menu.providers`, `plugins.registry`, and `plugins.settings` |
| Tables | typed cells, raw expressions, icons, and locked keys |
| Plugins | schema-driven controls with missing-field creation |
| Config | the Settings panel and `config.set`/`config.remember` |

Only the `binary-url.json` release manifest is undecided.

## Engine boundary

The RPC types live in `engine`. The server owns authentication, cookies, CSRF,
SSE framing, and static files. `nix-fleet-manager` can reuse the same engine
calls without using this server.
