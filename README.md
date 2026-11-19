# nix-web-gui

Web UI variant of nix-settings-gui.

A browser frontend over the same comment-driven Nix settings engine. The web UI
targets capability parity with the GTK settings window: every action is
available in the browser with the same Nix writes. Authentication is the one
deliberate difference, because the web UI can be opened from another device.

## Layout

- `engine/`: reusable Rust library. Scanner, model, metadata, values, edits,
  git, flake, authoring, previews, catalogs, theme, and settings sessions.
- `server/`: the nix-web-gui product. HTTP, PAM, browser sessions, CSRF, SSE,
  CLI, and static frontend serving.
- `frontend/packages/ui/`: reusable Svelte control renderer and theme provider.
  No auth, router, or RPC client.
- `frontend/packages/app/`: the nix-web-gui application.

`nix-fleet-manager` can reuse `engine` and the UI package without inheriting
this product's login flow.

## Status

Implemented end to end:

- Device-account login through PAM, browser sessions, CSRF, login throttling,
  session caps, secure cookies, Origin checks, CSP, audit logging, and SSE theme
  updates.
- Root parent plus per-user worker isolation. The worker runs with the account's
  uid, gid, groups, HOME, and XDG paths.
- Scan, menu categories with collapse, global search with jump and highlight,
  system-only host filtering with current-host detection, advanced and hidden
  grouping, visibleWhen gates, imported-from origins.
- Switch, dropdown, radio, spin, slider, text, multiline, path, color, package,
  packages, list, module list, theme, facter, font, icon, cursor, submodule,
  table, wallpaper, and file controls.
- Save, diff, discard, per-option reset, live apply with a serial queue,
  package validation, stale-file conflict detection, and Save & rebuild.
- Add file, add option, insert snippet, reload from disk, and an unsaved-change
  guard.
- Flake inputs with missing-input declarations, per-input check and update,
  nixpkgs refresh, Git status, diff, pull, push, commit, sync, config, startup
  page, and automatic Git pull or commit-and-push when configured.
- Font previews, monospace and variant filtering, searchable theme swatches,
  media upload and delete, icon-column upload, and composite row add, remove,
  and rename. User rows validate the account invariant and follow
  `home-manager.users`.
- Secure Boot status plus key creation, migration, enrollment, config writes,
  rebuild, and reboot from the root service account.
- Default and minimal packages. The minimal UI hides Ojuny records entirely and
  shows the unavailable notice.
- `checks.parity`, `checks.boundaries`, `checks.minimal-flag`, and the NixOS
  worker/auth VM test.

Not yet implemented:

- Dedicated Ojuny menu, plugins, bar, dock, grid, and docker editors, tile
  catalogs, and the grid canvas. Existing values edit through the generic
  structured renderer.
- Dedicated keybind, monitor, and user cards. Their rows support add, remove,
  rename, and field editing through the generic renderer.
- Icon and cursor theme preview sheets and custom wallpaper paths.
- Streamed and cancellable scan, save, apply, and update-check output.
- Playwright and frontend unit tests, a portable release flake input,
  `binary-url.json`, and prebuilt binary update checks.

The NixOS VM auth test passes. It verifies service startup, alice login,
wrong-password rejection, bob login through an allowed group but refusal to read
alice's `0700` home, carol allowlist rejection, worker RPC, scan, staged edit,
successful save, and a failed save that leaves the file unchanged.

See [TODO.md](TODO.md) for the remaining widget, flow, security, packaging, and
documentation work.

## Development

```sh
nix develop
cargo test --workspace
cd frontend
npm run check
npm run build
cd ../..
nix build
nix build .#checks.x86_64-linux.nixos-test-auth
```

Run the server with a local development password:

```sh
cargo run -p nix-web-gui-server -- serve \
  --dev-auth --dev-password secret \
  --static-dir frontend/packages/app/dist \
  --theme-file /tmp/nix-web-gui-theme
```

The production service uses PAM service `nix-web-gui` and the device accounts.
The NixOS module is `services.nix-web-gui`. It binds to `127.0.0.1` and allows
`wheel` by default.

The service runs a root parent for PAM and session handling, then starts a
per-user worker for the settings engine. The worker runs with the authenticated
account's uid, gid, groups, HOME, and XDG directories. `--dev-auth` uses an
in-process engine for local development only.

`services.nix-web-gui.allowRemote` must be set explicitly before a non-loopback
address is accepted. `services.nix-web-gui.startupPage` passes the CLI `--page`
selection to the service.

## Variants

`packages.default` includes the Ojuny widgets. `packages.nix-web-gui-minimal`
builds the Rust feature set without them and sets `VITE_OJUNY_WIDGETS=0`, so the
UI marks Ojuny controls as unavailable instead of rendering a generic fallback.

## Checks

```sh
nix flake check --offline
```

The check set is:

- `default`, `minimal`: both package variants.
- `boundaries`: the engine, server, and UI dependency boundaries.
- `parity`: every emitted widget name is documented and kebab-cased.
- `minimal-flag`: the minimal bundle carries the Ojuny notice.
- `nixos-test-auth`: the VM test for PAM login, worker RPC, save, failed save,
  cross-user denial, and allowlist rejection.

Tagged releases run the workflow in `.github/workflows/release.yml`, which builds
both packages and publishes a tarball with `SHA256SUMS`.

## Theme

The browser follows the device base16 scheme. `modules/ojuny-theme.nix`
projects `ojuny.style.palette` into `services.nix-web-gui.theme`, and the server
serves the name at `/api/theme`. The UI resolves it from the same base16
collection as `ojuny-site` and applies it through CSS variables. A theme change
arrives over SSE without a reload.

## Limits

The scanner keeps the documented module-shape rule. An `@option` inside
`lib.mkMerge`, a list literal, `map`, `forEach`, `lib.optionalAttrs`, or a
`lib.mkIf` body is ignored. Keep annotated assignments in a plain attribute set.
The fix is always to change the module shape, never to add list or apply-node
descent to the scanner.
