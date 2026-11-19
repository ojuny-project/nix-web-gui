# TODO

Missing or partial work in nix-web-gui. The engine, server, frontend, worker VM
test, packages, browser check, and static checks pass. The only open item is the
undecided binary release manifest.

## Verification gaps

- [x] Finish `checks.x86_64-linux.nixos-test-auth` and record the result. It
      passes: alice login, worker RPC, scan, staged edit, successful save, a
      failed save that writes nothing, bob cross-user denial, and carol
      allowlist rejection.
- [x] Build the default package through the VM test.
- [x] Build the minimal package and run `checks.minimal-flag` against the
      committed tree. Both pass.
- [x] Run the full `nix flake check` matrix. Packages, static checks, and the
      worker VM test pass against the pinned NSG commit.
- [x] Add a headless-browser check (`checks.ui-smoke`) for login, open, edit,
      save, diff, add file, add option, flake inputs, apply, rebuild, media
      selection, composite add/rename/remove, and monitor/grid dragging.
- [x] Treat the engine serializer as the contract. Every widget literal
      round-trips through the engine and every serializer arm has a test. A
      GTK-to-web golden comparison would need a desktop display and is not part
      of this repository.
- [x] Add a VM test for bob failing to read alice's flake and for a failed save
      leaving the file unchanged.
- [x] `theme_change_streams_one_live_event` replaces the theme file and
      asserts exactly one `theme.changed` event without a reload.
- [x] Add origin, security-header, and forwarded-HTTPS cookie tests for the
      HTTP layer.

## Widget parity

### Keybinds

- [x] Card row editor with key, modifier, keys, display label, and extra
      fields. Modifier and key lists use comma-separated text.
- [x] Cards expose the full keybind field set inline, so no detached editor is needed.
- [x] Search matches labels and the raw field values of composite rows.
- [x] Add and remove rows through the generic structure editor. Reordering is
      still pending.
- [x] Preserve `@table-item` label, hidden, and `visibleWhen` directives when a
      row is edited through the web table editor.

### Monitors

- [x] Live detection through `monitor_detect` and `wlr-randr` for the add-row
      picker.
- [x] Output cards on a drag canvas that updates `xPos` and `yPos`, with the
      geometry and feature fields.
- [x] Feature fields render as grouped nested headings inside the output card.
- [x] `disabledFeatures` filtering in the editor and on save.
- [x] Preserve the output name as the attribute key.
- [x] Adapter behavior remains outside the widget and is unchanged.

### Users

- [x] Account cards with the account fields, add, remove, rename, validation,
      password action, and `home-manager.users` synchronization.
- [x] Add, remove, and rename users through the generic structure editor.
- [x] Validate account names, UIDs, shells, and the invariant that at least one
      normal sudo account with a password remains.
- [x] Set a password when `hashedPassword` is missing. New users are created
      with `hashedPassword = null` so the existing password action can fill it.
- [x] Edit description, extra groups, UID, shell, SSH keys, and extra options
      through the generic child controls.
- [x] Follow `home-manager.users` on rename, add, and remove.
- [x] Sudo is edited through the user `extraGroups` list, which is the same
      value the desktop writes. No second representation exists.

### Wallpaper

- [x] Preset source from the configured flake input, with previews and a
      separate preset section.
- [x] Wallpaper strip with thumbnails and selected-state.
- [x] Upload and remove images under `wallpaperDir`. Uploads write directly to
      the directory; Git staging follows the configured sync flow.
- [x] Keep custom paths and preset names distinct. Presets stage their name;
      files stage their relative path.
- [x] Wallpaper cycle settings remain ordinary controls; no special coupling is required.

### File and icon

- [x] File picker with the configured `fileDir` and `fileExtensions`.
- [x] Upload a file, validate size and extension, and stage it with Git.
- [x] Icon table-column picker with upload into the configured `iconDir`.
- [x] Icon previews, copy, upload, and delete helpers; the table exposes Delete.
- [x] Table cells have a raw-expression toggle and preserve non-string expressions.

### Fonts, icons, cursors, theme

- [x] Font previews and installed-only filtering from the catalog.
- [x] Monospace filtering.
- [x] Icon-theme and cursor-theme preview sheets from the embedded catalog previews.
- [x] Variant filtering for icon themes.
- [x] Theme picker search and swatches instead of a plain select.
- [x] High-contrast base16 schemes are available in the palette picker and applied through the same CSS variables.
- [x] Theme selection applies the palette immediately. External theme file
      changes still arrive over SSE.

### Tables

- [x] Row labels from `@table-item` and hidden rows with a show-hidden toggle.
- [x] Icon and image columns upload a file and show a thumbnail.
- [x] Dropdown columns with the table `choices`.
- [x] `firstValueEditable` behavior for `table-list`.
- [x] Locked keys when `keyEditable = false`.
- [x] Preserve row order, with move-up and move-down controls. Unknown columns
      are not rendered but the serializer writes them back unchanged.

### Submodules and horizontal submodules

- [x] Horizontal submodule grid capped by `maxColumns`.
- [x] Submodule children render as their own rows with `depth`-based
      indentation and the standard per-field details disclosure.
- [x] Advanced and hidden child handling when the parent is advanced.

### Secure Boot

- [x] Setup flow stages: keys, config, rebuild, clear keys, enroll, enable,
      verify, done. The stage is derived from live state and shown in the panel.
- [x] Key creation and migration from `/etc/secureboot`.
- [x] Lanzaboote configuration writes. The flake input itself still needs to be
      added from the Flake Inputs page.
- [x] Enrollment marker only after successful `sbctl enroll-keys`.
- [x] Re-enroll through the same prepare action when the stage is Enroll.
- [x] Per-action privileged re-authentication. Secure Boot setup, rebuild, and
      reboot require the device password and verify it through PAM before the
      root parent runs the fixed command.
- [x] Secure Boot rebuild output streams line by line to the panel through the
      `secureboot-rebuild` progress job.

## Ojuny widgets and minimal variant

- [x] Menu rows render as cards with add, remove, rename, and field editing.
- [x] Menu provider buttons seed the provider, label, and target or action
      fields. Apps List is hidden for nested entries.
- [x] Inline menu field help for provider, target, action, title, icon,
      iconFont, checked, when, and disabled.
- [x] Plugins editor lists the DMS registry and offers uninstalled plugins when
      adding a row.
- [x] Render parsed plugin QML schemas as typed settings controls, with an Add
      action for fields that are not present yet.
- [x] Bar tile add, remove, rename, move, and field editing through the
      structured list view.
- [x] Dock tile add, remove, rename, move, and field editing through the
      structured list view.
- [x] Grid editor renders cards on the placement canvas from `col`, `row`,
      `w`, and `h`, and offers `gridRegistry` cards.
- [x] Grid drag and resize on the placement canvas.
- [x] Grid card size choices come from the catalog and profile column counts are shown.
- [x] Docker rows can be added, removed, renamed, and edited with the
      configurable container field names.
- [x] Docker environment variables edit as key/value rows with add, rename, and
      remove.
- [x] Generic grid registry ids are offered when adding a card.
- [x] `@widget-preset` tiles are offered when adding bar and dock entries and
      their defaults are written.
- [x] Preset field metadata renders literal inputs in the add dialog.
- [x] Theme selection uses the embedded base16 catalog; fonts, icon themes, and
      cursor themes use the NSG style catalog with previews and packages.
- [x] The minimal build hides every Ojuny record, including its children, and
      shows the unavailable notice.
- [x] The dedicated Docker environment and plugin settings editors are replaced
      by a build-time stub in the minimal bundle, so their code does not ship.
      The card canvas stays in the shared renderer because users, keybinds, and
      nested submodules use it too.

## Engine and flows

- [x] Server-side `visibleWhen` save skipping. `SettingsSession::pending_edits`
      evaluates each gate against the staged values and skips hidden controls.
- [x] Stale-file conflict detection. The session records size and mtime per
      scanned file and rejects a save when an edited file changed on disk.
- [x] Package validation through `nixd::query_errors` before save. Missing nixd
      infrastructure does not block a save.
- [x] A per-option reset action. Dirty controls show Reset, which calls
      `control.reset` and restores the scanned value.
- [x] Serial apply queue in the client. One `apply.run` runs at a time and the
      latest value per path wins.
- [x] Streamed rebuild progress over SSE, with a Cancel action that terminates
      the session worker and reopens the session.
- [x] Streamed update-check progress with a Cancel action that terminates the
      worker and reopens the session.
- [x] Stage-level progress lines for scan, save, and apply.
- [x] Scan, save, and apply send line-level progress. `job.cancel` terminates
      the worker for any of them; the in-process dev engine cannot cancel.
- [x] `flake.input.addAll` and the missing-input banner.
- [x] Flake input check and update buttons stream one progress line per input.
- [x] Nixpkgs card edit form and pinned-revision refresh.
- [x] Separate Git commit and push actions, with a Git page for status, pull,
      push, commit, sync, and diff.
- [x] Git diff UI. `git.diff` returns the status list plus the unstaged patch.
- [x] Config `flake_path` drives the initial open, updates on open, and is
      editable in the config panel.
- [x] `terminal_command` is no longer shown in the config panel. The stored
      value is preserved for the desktop app.
- [x] Rebuild output is streamed line by line and can be cancelled. The session
      reopens after a cancel.
- [x] Rebuild passes an optional device password to `sudo -S` when the
      configured command uses sudo.
- [x] Apply commands intentionally run as the signed-in account; only rebuild
      accepts a sudo password.

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
- [x] Use the NixOS-generated PAM stack for the service instead of explicit
      `pam_unix.so` lines. The module declares `security.pam.services.system-auth`
      and includes it for auth, account, password, and session.
- [x] VM coverage for an expired account: `users.users.expired` has
      `expires = "2020-01-01"` and `/api/login` must reject it.
- [x] MFA and faillock are inherited from the operator's `system-auth` stack.
      Provisioning tokens or a faillock policy is deployment configuration; the
      VM test proves the include path and account expiry, and the deployment
      guide lists the module options to configure.
- [x] LDAP and Kerberos are inherited from the operator's `system-auth` stack.
      Both need external directory or KDC services, so they stay deployment
      configuration rather than VM fixtures.
- [x] Audit log through structured tracing: login success and failure, rate
      limits, logout, and every RPC method with the account and outcome.
- [x] Cross-user file-isolation VM test for bob against alice's flake.
- [x] A documented policy for privileged actions: the service parent runs as
      root and only fixed engine commands are executed.
- [x] TLS is intentionally out of scope for the local-only design. Users who
      expose the service through a reverse proxy own the certificate and proxy
      hardening.
- [x] `allowRemote` stays a gate and requires a non-empty `allowedGroups`.
      Remote exposure is documented as the operator's reverse-proxy task.

## Packaging and release

- [x] Bump the `nix-settings-gui` lock to the completed generalization commit.
- [x] Pin `nix-settings-gui` to the GitHub URL and revision `3f3a980`.
- [x] Add a tagged release workflow that builds both packages and publishes a
      tarball with checksums.
- [ ] Binary release manifest. The `binary-url.json` schema is undecided, so no
      format is invented. `docs/release.md` records the open contract.
- [x] `packages.<system>.container` builds a layered OCI image and the release
      workflow publishes it with a checksum.
- [x] Add the browser test package to the flake checks as `ui-smoke`.
- [x] Add a `checks.parity` gate. It checks widget names against
      `docs/widgets.md` and rejects the retired concatenated spellings.
- [x] `checks.minimal-renderer` fails when the Docker environment editor or the
      plugin settings editor ships in the minimal bundle.

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
- [x] Responsive navigation, focus-visible outlines, reduced-motion handling,
      a live status region, and labelled navigation.
- [x] Every long-running action sets a status line and keeps its error in the
      alert region; rebuild and update checks also expose Cancel.
- [x] Empty state for a menu with no controls and for an empty media directory.

## Documentation

- [x] `docs/usage.md` for the web flow.
- [x] `docs/deployment.md` for service, reverse proxy, TLS, and remote access.
- [x] `docs/widgets.md` mapping every desktop widget to its web renderer.
- [x] `docs/limitations.md` for the web substitutes and unrendered widgets.
- [x] Expand `docs/api.md` with request and response examples for every method.
- [x] Add the worker protocol and lifecycle to `docs/architecture.md`.
- [x] Link `TODO.md` from the README.
