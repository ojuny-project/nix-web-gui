# Security

## Current isolation

The systemd unit runs a small root parent. It handles PAM, login, cookies, CSRF,
and worker spawning. The listener binds to `127.0.0.1` by default and only
`wheel` may sign in.

- Each authenticated browser session gets a worker on its first non-system RPC.
- The worker runs with the user's uid, primary gid, and supplementary groups.
- The worker receives the user's HOME, USER, LOGNAME, XDG_CONFIG_HOME, and
  XDG_STATE_HOME, so the existing `~/.config/nix-settings-gui/config.json`,
  recents, and file permissions apply.
- The worker owns the scan, staged values, file writes, git, apply, rebuild, and
  Secure Boot status actions.
- The root parent never scans or writes a configuration file.
- Cross-user reads and writes are rejected by Unix permissions, not only by
  application checks.
- Development auth uses an in-process engine only when `--dev-auth` is set.
  Production PAM uses the per-user worker path.

## Transport

- The HTTP listener is loopback-only by default. A non-loopback address requires
  `services.nix-web-gui.allowRemote = true` and a non-empty allowed group list.
- TLS is terminated by a reverse proxy. There is no built-in certificate
  authority.
- Session cookies are HttpOnly and SameSite=Strict. The `Secure` flag is set
  when the reverse proxy reports HTTPS.
- Mutating RPC calls require the per-session CSRF token in `X-CSRF-Token`.
- `Cache-Control: no-store` is set for API responses.

## Paths

The engine confines edits to the configured root and refuses `/nix/store`.
Traversal and symlink escape are covered by tests. The root is opened by the
authenticated user's worker, so the operating system enforces the same file
permissions as a local shell.

## Privileged actions

Rebuild and Secure Boot actions need root on the device. The local GUI opens a
terminal and lets the user authenticate with sudo. The web GUI will re-prompt
for the device password, verify it through PAM, and feed it to `sudo -S` over
stdin. The password never appears in argv or logs.

There is no general root helper and no arbitrary root command.

## Module shape rule

The scanner keeps the documented limitation. An `@option` inside
`lib.mkMerge`, a list literal, `map`, `forEach`, `lib.optionalAttrs`, or a
`lib.mkIf` body is ignored. Keep annotated assignments in a plain attribute set.
The fix is always to change the module shape, never to add list or apply-node
descent to the scanner.
