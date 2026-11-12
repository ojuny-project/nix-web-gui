# Architecture

The repository is split so `nix-fleet-manager` can reuse the settings logic and
the option renderers without inheriting this product's login flow.

```text
engine/                  reusable Rust library
  src/paths.rs           root confinement
  src/theme.rs           base16 name, file watch, broadcast
  src/session.rs         settings session, scan, staged values (next slice)
  src/api/               request and response types (next slice)

server/                  nix-web-gui product
  src/http.rs            axum router, cookies, CSRF, SSE, static frontend
  src/auth/              PAM and browser sessions
  src/config.rs          CLI and runtime configuration
  src/main.rs            serve and popout entry points

frontend/packages/ui/    reusable Svelte renderer
  src/lib/components/    Control, Field, Panel
  src/lib/controls/      switch, select, radio, number, text, color
  src/lib/theme.ts       base16 palette application
  src/lib/theme.css      semantic variables

frontend/packages/app/   nix-web-gui product
  src/api.ts             RPC client
  src/theme.ts           fetch and SSE theme follow
  src/pages/             Login, Shell
```

## Boundaries

- `engine` has no axum, PAM, tower-http, or frontend code. It is the crate a
  fleet manager can depend on.
- `server` owns authentication, browser sessions, transport, and static files.
  It maps HTTP to engine calls.
- `frontend/packages/ui` has no auth, router, or RPC client. It renders a
  `ControlDescriptor` and reports changes through a callback.
- `frontend/packages/app` owns the login flow and product layout.

## Theme path

`ojuny.style.palette` names a tinted base16 scheme. `modules/ojuny-theme.nix`
projects that name into `services.nix-web-gui.theme`. The server writes the
name to `/etc/nix-web-gui/theme`, serves it at `/api/theme`, and watches the
file. The UI resolves the name from the same base16 collection as `ojuny-site`
and applies it through CSS variables. A live palette change pushes one
`theme.changed` SSE event, so the page updates without a reload.

## Documented limitation

The scanner binds an `@option` comment to the next assignment in the same
container. It does not descend into a list of attribute sets. An annotation
inside `lib.mkMerge [...]`, a list literal, `map`, `forEach` or
`lib.optionalAttrs` is ignored. `lib.mkIf` is supported, and options may sit
directly inside a `lib.mkIf` attribute set.

The fix for an invisible option is always to change the module shape. This
repository does not modify the scanner to add list or apply-node descent.
