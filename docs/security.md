# Security

## Current isolation

The systemd unit runs a small root parent. It handles PAM, login, cookies, CSRF,
session limits, the security headers, and worker spawning. The listener binds to
`127.0.0.1` by default and `wheel` is the default allowed group.

- Each authenticated browser session gets a worker on its first non-system RPC.
- The worker runs with the user's uid, primary gid, and supplementary groups.
- The worker receives the user's HOME, USER, LOGNAME, XDG_CONFIG_HOME, and
  XDG_STATE_HOME, so the existing `~/.config/nix-settings-gui/config.json`,
  recents, and file permissions apply.
- The worker owns the scan, staged values, file writes, git, apply, rebuild, and
  Secure Boot status and configuration writes.
- The root parent answers `system.ping` and `system.capabilities` and runs the
  fixed Secure Boot setup, rebuild, and reboot actions. Those actions require
  the signed-in account's device password and verify it through PAM first. It never scans or writes
  an arbitrary configuration file.
- Cross-user reads and writes are rejected by Unix permissions, not only by
  application checks.
- Development auth uses an in-process engine only when `--dev-auth` is set.
  Production PAM uses the per-user worker path.

The session and engine maps are pruned every five minutes. Expired sessions are
removed from `SessionStore`, and engine state or workers whose session id no
longer exists are dropped.

## Login throttling

`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/auth/mod.rs` tracks
failed logins per username:

- five failures inside a five-minute window lock the account;
- the lockout lasts five minutes;
- a successful login clears the entry;
- a blocked login returns HTTP `429` and is recorded as a warning.

## Sessions

`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/auth/session.rs` stores
one session per browser:

- the session id and CSRF token are 32 random bytes, hex encoded;
- the lifetime is `--session-ttl-secs`, default `43200`;
- at most `--max-sessions` sessions exist, default `16`; creating one beyond the
  cap evicts the session with the earliest expiry, which is the oldest session;
- an expired session is rejected on lookup and removed;
- a five-minute task purges expired sessions and stale engine state or workers.

The cookie is named `nix_web_gui_session`, is HttpOnly, SameSite=Strict, and
Path=/. The `Secure` flag is set when `--secure-cookie` is passed or the request
carries `X-Forwarded-Proto: https`.

## Transport

- The HTTP listener is loopback-only by default. A non-loopback address requires
  `services.nix-web-gui.allowRemote = true` and a non-empty allowed group list.
- TLS is out of scope. The service is designed for loopback use. If an operator
  exposes it through a reverse proxy, certificate management and proxy
  hardening are their responsibility.
- Mutating RPC calls require the per-session CSRF token in `X-CSRF-Token`.
- `POST /api/login`, `POST /api/logout`, and `POST /api/rpc` check the `Origin`
  header. An `Origin` that does not match the request `Host` is rejected, and
  `Origin: null` is rejected. A request without an `Origin` header is allowed
  for CLI clients and is still gated by CSRF where applicable.
- Every response carries a strict Content Security Policy:
  `default-src 'self'; img-src 'self' data: blob:; style-src 'self'
  'unsafe-inline'; script-src 'self'; connect-src 'self'; font-src 'self';
  object-src 'none'; base-uri 'self'; frame-ancestors 'none'; form-action
  'self'`.
- Every response also sets `X-Content-Type-Options: nosniff`,
  `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`,
  `Permissions-Policy: geolocation=(), microphone=(), camera=()`, and
  `Cross-Origin-Opener-Policy: same-origin`.

## Paths and uploads

The engine confines edits to the configured root and refuses `/nix/store`.
Traversal and symlink escape are covered by tests. The root is opened by the
authenticated user's worker, so the operating system enforces the same file
permissions as a local shell.

Media handling in
`/home/den/repos/ojuny-future-repos/nix-web-gui/engine/src/preview.rs` adds:

- the media directory comes from `wallpaperDir`, `fileDir`, or `iconDir` and must
  stay inside the module directory and the configuration root;
- an upload name must be one plain file name with no directory separators, and
  hidden names are rejected;
- the extension must be one of the widget's accepted extensions;
- uploads are limited to 32 MiB;
- reads and deletes are confined to the widget's media directory;
- an upload or delete runs `git add -A -- <relative path>` so the change is
  staged for the next commit.

## Audit

The server writes structured `tracing` records for:

- a successful login, with the account;
- a failed login, with the account and the error;
- a login blocked by the rate limit, with the account;
- a logout, with the account;
- every RPC request, with the account, the method, and whether the response is
  an error.

Worker stderr is inherited by the service, so worker diagnostics reach the
systemd journal.

## Privileged actions

The root parent policy covers the Secure Boot actions that must run as root:

- `secureboot.setup` creates or migrates the key bundle and enrolls the keys.
  It accepts only `keep`, `create`, or `migrate`, plus the Microsoft and
  firmware-builtin options.
- `secureboot.rebuild` runs the configured rebuild command.
- `secureboot.reboot` runs `systemctl reboot` only when the request includes
  `confirm: true`.

Those three actions are handled in
`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/http.rs`, not in the
user worker. The engine functions check the effective uid and return a clear
error when they are not running as the root service account. The commands are
built by the engine from the live Secure Boot state; the browser cannot supply
command text.

`secureboot.status`, `secureboot.config`, and `secureboot.writeConfig` run in
the signed-in account's worker. The configuration write inserts only the missing
Lanzaboote assignments into the module that owns the Secure Boot toggle and
refuses a path outside the configuration root.

Secure Boot setup, rebuild, and reboot require the device password and verify it
through PAM in the root parent. `rebuild.run` can feed an optional password to
`sudo -S` when the configured command uses sudo. `apply.run` runs as the
signed-in account and follows its sudo policy. There is no general root helper
and no arbitrary root command.

## PAM

The NixOS module declares `security.pam.services.system-auth` and points
`security.pam.services.nix-web-gui.text` at it with four `include` lines: one
each for `auth`, `account`, `password`, and `session`. NixOS has no built-in
`system-auth`, so the module creates it as an ordinary PAM service; the name is
only the shared stack this repository includes. The service contains no module
lines of its own, so every module configured on `system-auth` applies to a web
login:

- `pam_unix` authentication, account expiry, and password rules from the NixOS
  defaults;
- `pam_faillock` when `security.pam.services.system-auth.logFailures` is set;
- MFA modules enabled on `system-auth`: `googleAuthenticator`, `duoSecurity`,
  `u2f`, `oath`, `yubicoAuth`, and `otpwAuth`;
- LDAP through `users.ldap.enable` and `users.ldap.loginPam`;
- Kerberos through `security.pam.krb5.enable`.

The allowlist groups are checked after PAM authentication. The VM test covers a
successful login, a wrong password, allowlist rejection, cross-user denial, and
an account whose `expires` date has passed. MFA, faillock, LDAP, and Kerberos are
inherited but untested: each needs an external token, LDAP server, or KDC that
the test does not provision.

## Module shape rule

The scanner keeps the documented limitation. An `@option` inside
`lib.mkMerge`, a list literal, `map`, `forEach`, `lib.optionalAttrs`, or a
`lib.mkIf` body is ignored. Keep annotated assignments in a plain attribute set.
The fix is always to change the module shape, never to add list or apply-node
descent to the scanner. The enforcement test is
`gate_limitation_no_list_descent` in
`/home/den/repos/ojuny-future-repos/nix-web-gui/engine/src/session.rs`, using the
`limitations` fixture. `docs/parity.md` calls the policy gate
`gate-limitation-no-list-descent`.
