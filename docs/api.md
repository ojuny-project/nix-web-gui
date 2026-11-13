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
| `page.describe` | implemented with nixd and eval fallback |
| `control.get` | implemented |
| `control.describe` | implemented without nixd metadata |
| `control.set` | implemented |
| `control.setTyped` | implemented |
| `control.reset` | implemented |
| `edits.preview` | implemented |
| `edits.apply` | implemented |
| `file.add` | implemented |
| `file.templates` | implemented |
| `option.add` | implemented |
| `insert.apply` | implemented |
| `flake.inputs` | implemented |
| `flake.input.upsert` | implemented |
| `flake.input.delete` | implemented |
| `flake.channels` | implemented |
| `flake.suggest` | implemented |
| `flake.update.check` | implemented |
| `flake.update` | implemented |
| `git.status` | implemented |
| `git.diff` | implemented |
| `git.pull` | implemented |
| `git.sync` | implemented |
| `apply.run` | implemented |
| `rebuild.run` | implemented |
| `catalog.styles` | implemented |
| `secureboot.status` | implemented |
| `secureboot.config` | implemented |
| `users.hashPassword` | implemented |
| `preview.list` | implemented |
| `preview.read` | implemented |
| `config.get` | implemented |
| `config.set` | implemented |
| `echo` | test helper |

## Planned methods

The contract is filled in as the engine slices land.

| Group | Methods |
|---|---|
| Session | `session.state` |
| Pages | metadata batching beyond `page.describe` |
| Edits | package validation, conflict detection |
| Authoring | dialog UI for `file.add`, `option.add`, `insert.apply` |
| Flake | `flake.input.addAll`, the nixpkgs card UI, and scan progress |
| Git | separate commit and push controls, if needed |
| Apply | streamed rebuild output and cancellation |
| Secure Boot | `secureboot.status`, `secureboot.setup`, `secureboot.repair` |
| Users | the account-card UI |
| Previews | wallpaper and file media picker; icon upload is pending |
| Previews | `preview.wallpaper`, `preview.icon`, `preview.style`, `preview.font`, `preview.cursor` |
| Catalogs | `catalog.themes`, `catalog.fonts`, `catalog.icons`, `catalog.cursors`, `catalog.styles`, `plugins.registry`, `plugins.settings`, `facter.detect`, `monitors.detect` |
| Config | the settings panel UI |

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
