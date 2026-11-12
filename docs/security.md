# Security

## Current state

This is the first vertical slice. It authenticates through PAM and serves the
settings engine from the same process. The systemd unit runs as root so PAM can
read the device account database. The listener binds to `127.0.0.1` by default
and only `wheel` may sign in.

Do not expose this build to a non-loopback address yet. The root worker split is
the next security step.

## Planned isolation

- A small root parent handles PAM, login, cookies, CSRF, and worker spawning.
- Each authenticated browser session gets a worker that runs with that user's
  uid and gid. The worker owns the scan, staged values, file writes, git, apply,
  and rebuild actions.
- The root parent never scans or writes a configuration file.
- The worker receives the user's HOME and XDG directories, so the existing
  `~/.config/nix-settings-gui/config.json`, recents, and file permissions apply.
- Cross-user reads and writes are rejected by Unix permissions, not only by
  application checks.

## Transport

- The HTTP listener is loopback-only by default.
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
`lib.mkMerge`, a list literal, `map`, `forEach`, or `lib.optionalAttrs` is
ignored. `lib.mkIf` is supported. The fix is always to change the module shape,
never to add list or apply-node descent to the scanner.
