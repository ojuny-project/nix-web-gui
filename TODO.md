# TODO

Missing or partial work in nix-web-gui. The engine, server, frontend, worker VM
test, minimal package, and minimal-flag check pass. The remaining items below are
feature and hardening gaps.

## Verification gaps

- [x] Finish `checks.x86_64-linux.nixos-test-auth` and record the result. It
      passes: alice login, worker RPC, scan, staged edit, save, wrong-password
      rejection, and bob allowlist rejection.
- [x] Build the default package through the VM test.
- [x] Build the minimal package and run `checks.minimal-flag` against the
      committed tree. Both pass.
- [ ] Run the full `nix flake check` matrix.
- [ ] Add Playwright coverage for login, open, edit, save, diff, add file, add
      option, flake inputs, apply, rebuild, and media selection.
- [ ] Add a desktop-vs-engine golden test for every widget literal. The current
      render tests cover a small subset.
- [ ] Add a VM test for bob failing to read alice's flake and for a failed save
      leaving every file byte-identical.
- [ ] Add a theme SSE test that replaces the theme file and asserts one live
      update without reload.
- [ ] Add an origin, security-header, and cookie-flag test for the HTTP layer.

## Widget parity

### Keybinds

- [ ] Dedicated row editor with key, modifier, keys, display label, and extra
      fields. Rows currently render as nested child controls.
- [ ] Comma-separated modifier and key lists rendered as Nix string lists.
- [ ] Pop-out editor for long field sets.
- [ ] Search text from labels and fields.
- [x] Add and remove rows through the generic structure editor. Reordering is
      still pending.
- [ ] Preserve `@table-item` label and visibility.

### Monitors

- [x] Live detection through `monitor_detect` and `wlr-randr` for the add-row
      picker.
- [ ] Output cards with width, height, refresh, position, scale, transform,
      primary, mirror, and enabled state. Existing fields edit through the
      generic child controls.
- [ ] Grouped color, HDR, sync, render, signal, reserved, hot-corner, and
      background fields.
- [ ] `disabledFeatures` filtering in the editor and on save.
- [ ] Preserve the output name as the attribute key.
- [ ] Adapter behavior remains outside the widget and is unchanged.

### Users

- [ ] Dedicated account cards instead of the generic nested editor.
- [x] Add and remove users through the generic structure editor. Rename
      handling is still pending.
- [ ] Validate account names, group names, UIDs, shells, and the invariant that
      at least one normal sudo account with a password remains.
- [ ] Set a password when `hashedPassword` is missing. New users are created
      with `hashedPassword = null` so the existing password action can fill it.
- [ ] Edit description, extra groups, UID, shell, SSH keys, and extra options.
- [ ] Follow `home-manager.users` on rename, add, and remove when
      `homeManager = true`.
- [ ] Separate sudo switch.

### Wallpaper

- [ ] Preset source from the configured flake input.
- [x] Wallpaper strip with thumbnails and selected-state.
- [x] Upload and remove images under `wallpaperDir`. Uploads write directly to
      the directory; Git staging follows the configured sync flow.
- [ ] Keep custom paths and preset names distinct.
- [ ] Wire the cycle settings that live beside the wallpaper option.

### File and icon

- [x] File picker with the configured `fileDir` and `fileExtensions`.
- [x] Upload a file, validate size and extension, and stage it with Git.
- [ ] Icon table-column picker with preview and upload.
- [ ] Copy and remove operations for icons.
- [ ] Preserve the raw expression when a field is not a literal path.

### Fonts, icons, cursors, theme

- [x] Font previews and installed-only filtering from the catalog.
- [x] Monospace filtering.
- [ ] Icon-theme and cursor-theme preview sheets.
- [x] Variant filtering for icon themes.
- [x] Theme picker search and swatches instead of a plain select.
- [ ] High-contrast palette behavior.
- [ ] Live palette update from the theme `applyCommand`, not only from the
      system theme file.

### Tables

- [x] Row labels from `@table-item` and hidden rows with a show-hidden toggle.
- [x] Icon columns are text fields. Image previews are still pending.
- [x] Dropdown columns with the table `choices`.
- [x] `firstValueEditable` behavior for `table-list`.
- [x] Locked keys when `keyEditable = false`.
- [x] Preserve row order, with move-up and move-down controls. Unknown columns
      are not rendered but the serializer keeps declared columns.

### Submodules and horizontal submodules

- [ ] Horizontal submodule grid capped by `maxColumns`.
- [ ] Per-child details rows and depth indentation parity.
- [ ] Advanced and hidden child handling when the parent is advanced.

### Secure Boot

- [x] Setup flow stages: keys, config, rebuild, clear keys, enroll, enable,
      verify, done. The stage is derived from live state and shown in the panel.
- [x] Key creation and migration from `/etc/secureboot`.
- [x] Lanzaboote configuration writes. The flake input itself still needs to be
      added from the Flake Inputs page.
- [x] Enrollment marker only after successful `sbctl enroll-keys`.
- [x] Re-enroll through the same prepare action when the stage is Enroll.
- [ ] Per-action privileged re-authentication. Setup actions instead require the
      root service account and fail clearly outside it.
- [ ] Streamed rebuild output. The full output is returned when the command
      finishes.

## Ojuny widgets and minimal variant

- [ ] Dedicated menu editor with providers and field help.
- [ ] Plugins editor with the DMS registry and declarative settings QML controls.
- [ ] Bar editor with left, center, right sections, drag or move actions.
- [ ] Dock editor with widget tiles and extras.
- [ ] Grid editor with card canvas, drag, resize, catalog sizes, and profiles.
- [ ] Docker row editor.
- [ ] Tile catalog and `@widget-preset` fields.
- [ ] Theme, font, icon, and cursor DMS catalogs.
- [ ] The minimal build currently only marks Ojuny controls unavailable. It has
      no dedicated Ojuny components to exclude, so the default/minimal
      difference is mostly the notice and the backend feature flag.

## Engine and flows

- [x] Server-side `visibleWhen` save skipping. `SettingsSession::pending_edits`
      evaluates each gate against the staged values and skips hidden controls.
- [x] Stale-file conflict detection. The session records size and mtime per
      scanned file and rejects a save when an edited file changed on disk.
- [ ] Package validation through `nixd::query_errors` before save.
- [x] A per-option reset action. Dirty controls show Reset, which calls
      `control.reset` and restores the scanned value.
- [ ] Apply queue and single-command semantics on the server. The current client
      debounce can issue overlapping `apply.run` calls.
- [ ] Streamed scan, update-check, apply, save, and rebuild progress. Only theme
      events use SSE.
- [ ] Cancellation for update checks, scans, and rebuilds.
- [x] `flake.input.addAll` and the missing-input banner.
- [x] Flake input check and update buttons in the UI. Progress is still a
      single blocking response.
- [x] Nixpkgs card edit form and pinned-revision refresh.
- [x] Separate Git commit and push actions, with a Git page for status, pull,
      push, commit, sync, and diff.
- [x] Git diff UI. `git.diff` returns the status list plus the unstaged patch.
- [x] Config `flake_path` drives the initial open, updates on open, and is
      editable in the config panel.
- [ ] `terminal_command` is unused by the web rebuild path. Document this or
      remove it from the panel.
- [ ] Rebuild output is returned at the end of a blocking RPC. It is not
      streamed and cannot be cancelled.
- [ ] Apply and rebuild privilege escalation relies on the account's sudo
      configuration. There is no password re-auth or `sudo -S` implementation.

## Auth and security

- [x] Login rate limiting and lockout: five failures in five minutes lock an
      account for five minutes.
- [x] Maximum active sessions and a cleanup task. The oldest session is dropped
      beyond the cap, and expired sessions and workers are purged every five
      minutes.
- [x] `Secure` cookie flag from `X-Forwarded-Proto: https` or `--secure-cookie`.
- [x] Origin check on mutating requests. Requests without an Origin header stay
      usable for CLI clients and are still gated by CSRF.
- [x] Strict CSP and standard security headers.
- [ ] Full PAM `system-auth` behavior. The service currently uses explicit
      `pam_unix.so` auth and account lines.
- [ ] PAM account expiration and multi-factor modules.
- [x] Audit log through structured tracing: login success and failure, rate
      limits, logout, and every RPC method with the account and outcome.
- [ ] Cross-user file-isolation VM test for bob against alice's flake.
- [x] A documented policy for privileged actions: the service parent runs as
      root and only fixed engine commands are executed.
- [ ] TLS is still terminated by a reverse proxy and is not built in.
- [ ] `allowRemote` remains a gate; production remote exposure still needs the
      hardening above.

## Packaging and release

- [ ] The `nix-settings-gui` input is `git+file:///home/den/repos/nix-settings-gui`
      for local development. Replace it with a pinned GitHub input for a
      release.
- [ ] Add release workflow, `binary-url.json`, and prebuilt binary support like
      the desktop repository.
- [ ] Add a container image if self-hosting is in scope.
- [ ] Add frontend unit tests and a browser test package to the flake checks.
- [ ] Add a `checks.parity` manifest check. `docs/parity.md` is documentation,
      not an enforced gate.
- [ ] Add a minimal-build test that proves no Ojuny renderer ships. The current
      `checks.minimal-flag` only proves the notice is present.

## Frontend UX

- [x] `?popout=<page>` detached-page behavior. The app hides navigation and
      opens the matching menu when the query parameter is present.
- [x] Unsaved-changes prompt on tab close.
- [x] Conflict prompt after a stale scan. The save error names the changed files
      and asks for a reload.
- [x] Global search result navigation and highlight. Search lists matches from
      every menu and jumps to the control.
- [x] Automatic current-host detection. The matching host is preselected in the
      system-only selector.
- [x] Sectioned Advanced area. Advanced and hidden controls are grouped under
      an Advanced heading.
- [x] Menu icons and category collapse.
- [ ] Responsive and accessibility audit.
- [ ] Loading and error states for every long-running action.
- [ ] Empty states for menus with no controls and missing media directories.

## Documentation

- [ ] `docs/usage.md` for the web flow.
- [ ] `docs/deployment.md` for service, reverse proxy, TLS, and remote access.
- [ ] `docs/widgets.md` mapping every desktop widget to its web renderer.
- [ ] `docs/limitations.md` for the web substitutes and unrendered widgets.
- [ ] Expand `docs/api.md` with request and response examples for every method.
- [ ] Add the worker protocol and lifecycle to `docs/architecture.md`.
- [ ] Link `TODO.md` from the README.
