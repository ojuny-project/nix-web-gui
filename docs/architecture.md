# Architecture

The repository is split so `nix-fleet-manager` can reuse the settings logic and
the option renderers without inheriting this product's login flow.

```text
engine/                  reusable Rust library
  src/paths.rs           root confinement
  src/theme.rs           base16 name, file watch, broadcast
  src/session.rs         settings session, scan, staged values, describe, structure
  src/render.rs          widget choice, literal serialization, table cells
  src/structure.rs       composite row templates and monitor detection
  src/api/               request and response JSON types
  src/apply.rs           applyCommand and rebuild commands
  src/authoring.rs       templates, new files, snippets
  src/flake.rs           flake inputs, channels, suggestions, updates
  src/git.rs             status, diff, commit, push, pull, sync
  src/preview.rs         media listing, reads, uploads, deletes
  src/catalog.rs         font, icon, and cursor catalogs
  src/secureboot.rs      firmware, key state, setup commands, config block
  src/users.rs           password hashing
  src/config.rs          shared desktop config file

server/                  nix-web-gui product
  src/http.rs            axum router, cookies, CSRF, Origin, headers, SSE
  src/rpc.rs             engine RPC dispatch used by in-process and worker modes
  src/worker.rs          stdio JSON-RPC worker entry point
  src/worker_process.rs  per-user worker spawn and request proxy
  src/auth/              PAM, browser sessions, login throttle
  src/config.rs          CLI and runtime configuration
  src/main.rs            serve, popout, worker, cleanup and theme tasks

frontend/packages/ui/    reusable Svelte renderer
  src/lib/components/    Control, Field, Panel
  src/lib/controls/      switch, select, radio, number, text, textarea, list,
                         color, table, theme picker
  src/lib/theme.ts       base16 palette application
  src/lib/theme.css      semantic variables

frontend/packages/app/   nix-web-gui product
  src/api.ts             RPC client
  src/theme.ts           fetch and SSE theme follow
  src/pages/             Login, Shell, CardCollection, MonitorCanvas,
                         MediaPicker, SecureBootPanel
```

## Boundaries

- `engine` has no axum, PAM, tower-http, or frontend code. It is the crate a
  fleet manager can depend on.
- `server` owns authentication, browser sessions, transport, static files, and
  the per-user worker boundary. The root parent maps HTTP RPC to a worker.
- `frontend/packages/ui` has no auth, router, or RPC client. It renders a
  `ControlDescriptor` and reports changes through a callback.
- `frontend/packages/app` owns the login flow and product layout.

## Process model

The service has one root parent and zero or more worker processes:

```text
nix-web-gui serve (root)
  axum listener
  SessionStore        HashMap<session id, Session>, capped
  LoginThrottle       per-username failure and lockout state
  ThemeStore          2 second refresh task, broadcast channel
  EngineMode::Workers HashMap<session id, Arc<Mutex<Worker>>>
      |
      +-- nix-web-gui worker   first browser session
      |     stdio JSON-RPC
      |     Option<SettingsSession>
      |
      +-- nix-web-gui worker   second browser session
            stdio JSON-RPC
            Option<SettingsSession>
```

The root parent handles PAM, login, cookies, CSRF, the Origin check, security
headers, SSE, static files, worker spawning, the session cap, and the cleanup
task. In PAM mode it never opens a settings root, scans a file, or writes an
arbitrary configuration file. All ordinary engine work happens in the worker for
the signed-in account.

The parent also answers `system.ping` and `system.capabilities` and runs the
fixed `secureboot.setup`, `secureboot.rebuild`, and `secureboot.reboot` actions.
Those three need root and are handled in `http.rs` before the worker dispatch.
`secureboot.status`, `secureboot.config`, and `secureboot.writeConfig` stay in
the worker.

## Browser session lifecycle

1. `POST /api/login` checks the login throttle, authenticates through PAM in the
   parent, and clears the failure count. `SessionStore` creates a random session
   id and CSRF token and stores the username, the CSRF token, and an expiry.
2. The session count is capped at `--max-sessions`. A new session beyond the cap
   evicts the session with the earliest expiry, which is the oldest session.
3. The browser receives the session cookie and the CSRF token. The cookie is
   HttpOnly and SameSite=Strict, and gets `Secure` when the request reports
   `X-Forwarded-Proto: https` or `--secure-cookie` is set. The token is sent in
   `X-CSRF-Token` on every `POST /api/rpc`.
4. `POST /api/login`, `POST /api/logout`, and `POST /api/rpc` check `Origin`
   against `Host`.
5. The first non-system RPC for that session spawns a worker. `system.ping` and
   `system.capabilities` are answered in the parent and do not spawn anything.
6. The worker is cached under the session id and reused for later RPC calls.
   The worker holds one `Option<SettingsSession>`, so it has one open root at a
   time.
7. `POST /api/logout` removes the session, the worker map entry, and the pid map
   entry. Dropping the `Worker` calls `start_kill` on the child. `job.cancel`
   looks up the recorded pid, sends `SIGTERM`, and drops the worker entry; the
   next RPC spawns a fresh worker.
8. An expired session is removed from `SessionStore` on lookup. A five-minute
   task also calls `purge_expired` and `purge_stale_engine`, so expired sessions
   and workers or in-process state for missing sessions are dropped.
9. When the worker's stdin reaches EOF, `run_stdio` returns and the process
   exits. A killed parent therefore leaves no worker running once the pipe
   closes.

Development mode uses `EngineMode::InProcess` instead. That map holds one
`Option<SettingsSession>` per session id and runs in the parent under the server
account. No worker process is created.

## Worker spawn

`Worker::spawn` in
`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/worker_process.rs`
takes the authenticated username and:

- looks the account up with `uzers::get_user_by_name`;
- reuses the current executable with the `worker` subcommand;
- sets the working directory to the account's home directory;
- sets `HOME`, `USER`, `LOGNAME`, `XDG_CONFIG_HOME` (`$HOME/.config`), and
  `XDG_STATE_HOME` (`$HOME/.local/state`);
- pipes stdin and stdout and inherits stderr, so worker diagnostics reach the
  systemd journal;
- collects the account's group ids and appends the primary gid when it is
  missing;
- runs `setgroups`, then `setgid`, then `setuid` in `pre_exec`, and aborts the
  spawn with the last OS error if any call fails.

The process is started without a shell and without sudo. PATH is inherited from
the service. `/home/den/repos/ojuny-future-repos/nix-web-gui/nix/package.nix`
wraps the binary with `nix`, `nixd`, `nixfmt`, `git`, `curl`, `libxcrypt`,
`openssl`, `sbctl`, `systemd`, and `wlr-randr` on PATH, so the worker sees the
same tools the desktop app uses. `wlr-randr` backs `monitor.detect`.

The environment gives the worker the account's own config and recents:

- `$XDG_CONFIG_HOME/nix-settings-gui/config.json` is the shared desktop config
  read by `config.get`, `config.set`, `config.remember`, `apply.run`, and
  `rebuild.run`;
- git uses the account's git configuration and credentials;
- nix uses the account's nix configuration.

Because the worker runs with the account's uid, gid, and supplementary groups,
cross-user reads and writes are rejected by Unix permissions. The engine also
confines edits to the open root in `src/paths.rs`, but that is a second check,
not the isolation boundary.

## Startup and config

`GET /api/state` runs in the root parent and returns the initial flake from
`--flake` and the parent's `flake_path`, then the recent flakes and hostname.
The shell falls back to the `nix-web-gui.root` key in browser `localStorage`
when neither is set.

Opening a flake calls `session.open`, scans, and calls `config.remember` in the
signed-in account's worker. That sets `flake_path` and moves the path to the
front of `recent_flake_paths`, truncated to 10. The config panel and Git page are
separate views in `Shell.svelte`; the Settings and Add file buttons in the sidebar open their panels.

## Worker protocol

The parent and worker speak newline-delimited JSON-RPC 2.0 over the worker's
stdin and stdout. Each line is one complete JSON object. There is no length
prefix, no framing header, and no multiplexing.

Request line, written by the parent:

```json
{"jsonrpc":"2.0","id":1,"method":"page.list","params":{}}
```

Response line, written by the worker:

```json
{"jsonrpc":"2.0","id":1,"result":{"pages":[],"hosts":[]}}
```

The envelope is the same one the browser uses at `/api/rpc`. The parent builds
it in `/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/http.rs` and
forwards the `id`, `method`, and `params` unchanged. The worker dispatches it
with `rpc::handle_with_progress`, the same dispatcher that serves in-process
mode, so method names, params, results, and error codes are identical.

A long method may write progress lines before its response. Each is a complete
JSON object with `method: "progress"` and no `id`:

```json
{"method":"progress","params":{"job":"rebuild","line":"building '/nix/store/...'"}}
```

`scan`, `save`, and `apply` send one stage line. `update` sends one line per
input. `rebuild` sends one line per output line. The parent `Worker::request`
loop reads lines until it sees one without the `progress` method, adds the
session id, and forwards progress to the `/api/events` SSE stream. The browser
filters on its own session id.

Worker loop in `/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/worker.rs`:

1. read one line from stdin;
2. skip blank lines;
3. parse the line, or answer `-32700` with `id: null` on a parse error;
4. handle the request against the worker's `Option<SettingsSession>`, writing
   any progress lines immediately and flushing;
5. write the response line and flush.

The loop is single threaded. Only one request is in flight per worker. The
parent's worker mutex enforces the same rule from the HTTP side. Stderr is
not part of the protocol.

## Request proxy and failure path

`Worker::request`:

1. serializes the request to a line and appends `\n`;
2. writes and flushes it to the child stdin;
3. reads lines from the child stdout, forwarding `progress` lines, until it
   reads the response line;
4. returns `the settings worker exited` when the read returns 0 bytes;
5. returns `invalid worker response` when the line is not valid JSON.

`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/http.rs` treats any
of those as a transport error:

- the worker is removed from the session's map entry;
- the RPC call returns HTTP `502` with `{ "error": "<message>" }`;
- there is no retry inside the same request.

The browser session cookie stays valid. The next RPC spawns a fresh worker,
which has no open settings session, so it returns `-32000` until the page calls
`session.open` again.

Failure cases:

| Case | Result |
|---|---|
| Unknown device account at spawn | `502`, `unknown device account: <name>` |
| Spawn failed, for example a permission error | `502`, `cannot start the settings worker: ...` |
| Worker exited, EOF on stdout | `502`, worker dropped |
| Worker wrote a non-JSON line | `502`, worker dropped |
| Worker request write failed | `502`, worker dropped |
| Parent restart | workers exit when their stdin closes |

The root parent only sends engine requests to the worker. It does not send PAM
prompts, file descriptors, or environment updates after spawn.

## Concurrency

- Each browser session is one worker process, guarded by an `Arc<Mutex<Worker>>`,
  so requests for the same session serialize.
- Different sessions run in separate processes and in parallel.
- There is no worker pool and no reuse across accounts.
- In worker mode the root parent holds no settings state, so a worker restart
  loses only the open session and the staged edits. Unsaved changes in the
  browser are lost with them.
- The client serializes live apply calls with one queue and keeps the latest
  value per path.

## HTTP surface

`/api/health`, `/api/login`, `/api/logout`, `/api/me`, `/api/state`,
`/api/theme`, `/api/rpc`, and `/api/events` are the API routes. The rest of the
path is served from the built frontend with `index.html` as the fallback.

A middleware adds the CSP and the security headers to every response. Login
failures are throttled per username. `tracing` records login success, login
failure, rate limits, logout, and every RPC method with the account and outcome.

## RPC groups

`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/rpc.rs` dispatches the
settings, authoring, flake, git, preview, and structure calls:

- `preview.upload` and `preview.delete` write and remove media through
  `engine/src/preview.rs`; both stage the change with `git add`;
- `structure.add`, `structure.remove`, `structure.rename`, and `structure.move`
  edit composite rows through `engine/src/structure.rs` and `SettingsSession`;
- `monitor.detect` reads compositor outputs through `wlr-randr`;
- `plugins.registry` reads the DMS registry and `plugins.settings` returns QML;
  the browser renders registry rows but no QML controls;
- `secureboot.writeConfig` inserts the missing Lanzaboote assignments in the
  worker; `secureboot.setup`, `secureboot.rebuild`, and `secureboot.reboot` are
  intercepted by `http.rs` and run in the root parent;
- `git.commit` and `git.push` are separate from `git.sync`;
- `config.remember` updates the shared config on open, and `flake.input.addAll`
  declares the missing `@inputs` entries.

## Theme path

`ojuny.style.palette` names a tinted base16 scheme. `modules/ojuny-theme.nix`
projects that name into `services.nix-web-gui.theme`. The NixOS module writes
the name to `/etc/nix-web-gui/theme`. The server serves it at `/api/theme` and
watches the file. The UI resolves the name from the same base16 collection as
`ojuny-site` and applies it through CSS variables. A live palette change pushes
one `theme.changed` SSE event, so the page updates without a reload.

The theme store refreshes every 2 seconds in a parent task created in
`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/main.rs`. It emits
only when the name changes. `/api/events` also sends one `theme` event on
connect and a `heartbeat` every 15 seconds.

## Documented limitation

The scanner binds an `@option` comment to the next assignment in the same
container. It does not descend into a list of attribute sets. An annotation
inside `lib.mkMerge [...]`, a list literal, `map`, `forEach`,
`lib.optionalAttrs` or a `lib.mkIf` body is ignored. Keep annotated assignments
in a plain attribute set.

The fix for an invisible option is always to change the module shape. This
repository does not modify the scanner to add list or apply-node descent. The
gate is `gate_limitation_no_list_descent` in
`/home/den/repos/ojuny-future-repos/nix-web-gui/engine/src/session.rs`. See
`/home/den/repos/ojuny-future-repos/nix-web-gui/docs/limitations.md` for the
disagreement between this rule and
`/home/den/repos/ojuny-template/docs/05-settings-gui-limitations.md` about
`lib.mkIf`.
