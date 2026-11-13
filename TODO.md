# TODO

Missing or partial work in nix-web-gui. The current slice builds, the engine and
server tests pass, and the frontend check and build pass. The per-user worker VM
test is the last verification gate.

## Verification gaps

- [ ] Finish `checks.x86_64-linux.nixos-test-auth` and record the result.
- [ ] Run a final `nix build .#default .#nix-web-gui-minimal` against the
      committed tree.
- [ ] Run `checks.minimal-flag`.
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
      fields.
- [ ] Comma-separated modifier and key lists rendered as Nix string lists.
- [ ] Pop-out editor for long field sets.
- [ ] Search text from labels and fields.
- [ ] Add, remove, and reorder rows.
- [ ] Preserve `@table-item` label and visibility.

### Monitors

- [ ] Live detection through `monitor_detect` and `wlr-randr`.
- [ ] Output cards with width, height, refresh, position, scale, transform,
      primary, mirror, and enabled state.
- [ ] Grouped color, HDR, sync, render, signal, reserved, hot-corner, and
      background fields.
- [ ] `disabledFeatures` filtering in the editor and on save.
- [ ] Preserve the output name as the attribute key.
- [ ] Adapter behavior remains outside the widget and is unchanged.

### Users

- [ ] Dedicated account cards instead of the generic nested editor.
- [ ] Add and remove users, including rename handling.
- [ ] Validate account names, group names, UIDs, shells, and the invariant that
      at least one normal sudo account with a password remains.
- [ ] Set a password when `hashedPassword` is missing.
- [ ] Edit description, extra groups, UID, shell, SSH keys, and extra options.
- [ ] Follow `home-manager.users` on rename, add, and remove when
      `homeManager = true`.
- [ ] Separate sudo switch.

### Wallpaper

- [ ] Preset source from the configured flake input.
- [ ] Wallpaper strip with thumbnails and selected-state.
- [ ] Copy and remove images under `wallpaperDir`.
- [ ] Stage copied or removed files with Git.
- [ ] Keep custom paths and preset names distinct.
- [ ] Wire the cycle settings that live beside the wallpaper option.

### File and icon

- [ ] File picker with the configured `fileDir` and `fileExtensions`.
- [ ] Upload a file, validate size and extension, and stage it with Git.
- [ ] Icon table-column picker with preview and upload.
- [ ] Copy and remove operations for icons.
- [ ] Preserve the raw expression when a field is not a literal path.

### Fonts, icons, cursors, theme

- [ ] Font previews and installed-only filtering.
- [ ] Monospace filtering.
- [ ] Icon-theme and cursor-theme preview sheets.
- [ ] Variant filtering for icon themes.
- [ ] Theme picker search and swatches instead of a plain select.
- [ ] High-contrast palette behavior.
- [ ] Live palette update from the theme `applyCommand`, not only from the
      system theme file.

### Tables

- [ ] Row labels from `@table-item`.
- [ ] Hidden rows and `visibleWhen` rows.
- [ ] Icon columns.
- [ ] Dropdown columns with the table `choices`.
- [ ] `firstValueEditable` behavior for `table-list`.
- [ ] Locked keys when `keyEditable = false`.
- [ ] Preserve row order and unknown columns.

### Submodules and horizontal submodules

- [ ] Horizontal submodule grid capped by `maxColumns`.
- [ ] Per-child details rows and depth indentation parity.
- [ ] Advanced and hidden child handling when the parent is advanced.

### Secure Boot

- [ ] Setup flow stages: keys, config, rebuild, clear keys, enroll, enable,
      verify, done.
- [ ] Key creation and migration from `/etc/secureboot`.
- [ ] Lanzaboote input and configuration writes.
- [ ] Enrollment marker only after successful `sbctl enroll-keys`.
- [ ] Repair keys action.
- [ ] Privileged re-authentication and `sudo -S` for setup actions.
- [ ] Streamed rebuild output and reboot guidance.

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

- [ ] Server-side `visibleWhen` save skipping. The app hides controls, but a
      staged value that becomes hidden can still be saved.
- [ ] Stale-file conflict detection. Two tabs or users can overwrite each other.
- [ ] Package validation through `nixd::query_errors` before save.
- [ ] A per-option reset/default action in the UI. `control.reset` exists but no
      control exposes it.
- [ ] Apply queue and single-command semantics on the server. The current client
      debounce can issue overlapping `apply.run` calls.
- [ ] Streamed scan, update-check, apply, save, and rebuild progress. Only theme
      events use SSE.
- [ ] Cancellation for update checks, scans, and rebuilds.
- [ ] `flake.input.addAll` and the missing-input banner.
- [ ] Flake input scan buttons and update-check progress in the UI.
- [ ] Nixpkgs card edit form and pinned-revision refresh.
- [ ] Separate Git commit and push actions. Only combined sync exists.
- [ ] Git diff UI. `git.diff` exists but no page renders it.
- [ ] Config `flake_path` should drive the initial open; it is currently only
      displayed.
- [ ] `terminal_command` is unused by the web rebuild path. Document this or
      remove it from the panel.
- [ ] Rebuild output is returned at the end of a blocking RPC. It is not
      streamed and cannot be cancelled.
- [ ] Apply and rebuild privilege escalation relies on the account's sudo
      configuration. There is no password re-auth or `sudo -S` implementation.

## Auth and security

- [ ] Login rate limiting and lockout.
- [ ] Maximum active sessions and a session cleanup task.
- [ ] `Secure` cookie flag when the reverse proxy reports HTTPS.
- [ ] Origin check on mutating requests.
- [ ] Strict CSP and standard security headers.
- [ ] Full PAM `system-auth` behavior. The service currently uses explicit
      `pam_unix.so` auth and account lines.
- [ ] PAM account expiration and multi-factor modules.
- [ ] Audit log for login, save, Git, apply, rebuild, and privileged actions.
- [ ] Cross-user file-isolation VM test for bob against alice's flake.
- [ ] A privileged helper or a documented policy for Secure Boot actions.
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

- [ ] `?popout=<page>` detached-page behavior. Only the CLI `--popout` launcher
      exists.
- [ ] Unsaved-changes prompt on tab close or navigation.
- [ ] Conflict prompt after a stale scan.
- [ ] Global search result navigation and highlight. Search currently filters
      the current page and menu list.
- [ ] Automatic current-host detection. The system-only filter relies on a
      manual host selector.
- [ ] Sectioned Advanced area. Advanced controls are filtered but not grouped.
- [ ] Menu icons and category collapse.
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
