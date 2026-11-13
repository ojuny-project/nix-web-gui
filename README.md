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

- Device-account login through PAM, browser sessions, CSRF, SSE theme updates.
- Root parent plus per-user worker isolation. The worker runs with the account's
  uid, gid, groups, HOME, and XDG paths.
- Scan, menu categories, groups, search, system-only host filtering, advanced
  and hidden controls, visibleWhen gates, imported-from origins.
- Switch, dropdown, radio, spin, slider, text, multiline, path, color, package,
  packages, list, module list, theme, facter, font, icon, cursor, submodule,
  table, wallpaper, and file controls.
- Save, diff, discard, live apply, Save & rebuild.
- Add file, add option, and insert snippet.
- Flake inputs, nixpkgs card, Git sync, config, startup page, and automatic Git
  pull or commit-and-push when configured.
- Font, icon, and cursor package sidecars.
- User password hashing into an existing `hashedPassword` field.
- Media picker for wallpaper and file directories.
- Secure Boot status card.
- Default and minimal packages with a minimal-build Ojuny notice.

Not yet implemented:

- Dedicated keybind, monitor, user-card, wallpaper upload, icon upload, and
  Ojuny bar, dock, menu, plugins, grid, and docker editors. These currently use
  the generic nested editor or a raw Nix fallback.
- Secure Boot setup actions that create keys, enroll keys, rebuild, or reboot.
- Playwright tests and the full `nix flake check` matrix.

The NixOS VM auth test passes. It verifies service startup, alice login,
wrong-password rejection, bob allowlist rejection, worker RPC, scan, staged
edit, and save as the authenticated account.

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
