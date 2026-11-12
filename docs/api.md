# HTTP API

The browser talks to the server over JSON-RPC 2.0. The server maps each method
to an `engine` call. Method names and error codes follow the existing
`nix-settings-backend` envelope where it already defines them.

## Transport

- `POST /api/rpc` with a JSON-RPC request body.
- The request needs the session cookie and the `X-CSRF-Token` header.
- `GET /api/events` is the authenticated SSE stream for progress, cancellation,
  and theme changes.
- `GET /api/theme` returns the active base16 name.
- `GET /api/state` returns the startup flake, authentication mode, and theme.
- The browser sits behind the same origin as the API. TLS is terminated by a
  reverse proxy.

## Authentication

| Method | Body | Result |
|---|---|---|
| `GET /api/me` | none | `{ username, csrf }` or `401` |
| `GET /api/state` | none | `{ flake, auth, theme }` or `401` |
| `POST /api/login` | `{ username, password }` | sets the session cookie, returns `{ username, csrf }` |
| `POST /api/logout` | none | clears the session |

Credentials are checked through PAM against the device account. The PAM service
is `nix-web-gui`. The account phase runs after authentication, and the
configured allowed groups are enforced when the list is non-empty.

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
| `scan` | implemented |
| `page.list` | implemented |
| `control.get` | implemented |
| `control.set` | implemented |
| `control.reset` | implemented |
| `edits.preview` | implemented |
| `edits.apply` | implemented |
| `echo` | test helper |

## Planned methods

The contract is filled in as the engine slices land.

| Group | Methods |
|---|---|
| Session | `session.reload`, `session.state` |
| Pages | `control.describe` |
| Edits | `edits.preview`, `edits.apply` |
| Authoring | `file.add`, `option.add`, `insert.apply` |
| Flake | `flake.inputs`, `flake.input.upsert`, `flake.input.delete`, `flake.input.addAll`, `flake.nixpkgs`, `flake.scan`, `flake.update.check`, `flake.update` |
| Git | `git.status`, `git.diff`, `git.sync`, `git.commit`, `git.push` |
| Apply | `apply.run`, `rebuild.run` |
| Secure Boot | `secureboot.status`, `secureboot.setup`, `secureboot.repair` |
| Users | `users.hashPassword` |
| Previews | `preview.wallpaper`, `preview.icon`, `preview.style`, `preview.font`, `preview.cursor` |
| Catalogs | `catalog.themes`, `catalog.fonts`, `catalog.icons`, `catalog.cursors`, `catalog.styles`, `plugins.registry`, `plugins.settings`, `facter.detect`, `monitors.detect` |
| Config | `config.get`, `config.set` |

## Error codes

| Code | Meaning |
|---|---|
| `-32000` | no open settings session |
| `-32010` | scan failed |
| `-32011` | unknown option path |
| `-32012` | edit failed |
| `-32013` | rescan failed after save |
| `-32601` | unknown method |
| `-32700` | parse error |

## Engine boundary

The RPC types live in `engine`. The server owns authentication, cookies, CSRF,
SSE framing, and static files. `nix-fleet-manager` can reuse the same engine
calls without using this server.
