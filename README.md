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

## Development

```sh
nix develop
cargo test --workspace
cd frontend
npm run check
npm run build
cd ../..
nix build
```

Run the server with a local development password:

```sh
cargo run -p nix-web-gui-server -- serve \
  --dev-auth --dev-password secret \
  --static-dir frontend/packages/app/dist \
  --theme-file /tmp/nix-web-gui-theme
```

The production service uses PAM service `nix-web-gui` and the device accounts.
The NixOS module is `services.nix-web-gui`.

## Theme

The browser follows the device base16 scheme. `modules/ojuny-theme.nix`
projects `ojuny.style.palette` into `services.nix-web-gui.theme`, and the server
serves the name at `/api/theme`. The UI resolves it from the same base16
collection as `ojuny-site` and applies it through CSS variables. A theme change
arrives over SSE without a reload.

## Limits

The scanner keeps the documented module-shape rule. An `@option` inside
`lib.mkMerge`, a list literal, `map`, `forEach`, or `lib.optionalAttrs` is
ignored. `lib.mkIf` is supported. The fix is always to change the module shape,
never to add list or apply-node descent to the scanner.
