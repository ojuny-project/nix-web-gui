# DRAFT - nix-web-gui

Status: draft scope. Nothing is built. This file describes what the repository
is supposed to contain, not what it contains today.

## Purpose

A web-based variant of `nix-settings-gui`. It renders the same comment-annotated
Nix configuration as a browser interface instead of a GTK4 window, so a flake
can be edited from another device or embedded in another web product.

It is a general Nix tool, not an Ojuny-only component. Ojuny uses it for remote
management. `ojuny-fleet-manager` (OFM) is the enterprise-geared version of this
repository and shares its backend, so both present the same editing model.

## What the repository will contain

```text
nix-web-gui/
  flake.nix                  package, module and app outputs
  server/                    HTTP server over the nix-settings-gui backend
    api/                     request and response schema
    auth/                    token and session handling
    root/                    configured configuration root
    apply/                   applyCommand execution
  frontend/                  browser interface
    pages/                   menu, editor, diff, settings
    components/              widgets mapped from the same widget names
  docs/                      API, deployment, security, limitations
  tests/                     backend, frontend and end-to-end checks
```

The scanner, DSL, model, metadata, value, edit, git, descriptor and template
layers come from the `nix-settings-gui` backend. This repository does not fork
them. It adds the HTTP server, authentication, the browser client and the
service packaging.

### Flake outputs

- `packages.nix-web-gui`: the server and bundled frontend.
- `nixosModules.default`: installs the service behind a local address.
- `apps.run`: starts the server against a chosen flake directory.
- `packages.container`: an optional container image for self-hosting.
- Checks that evaluate the module and run the backend tests.

### Feature parity

- Scan a flake or configuration directory and list every menu and page.
- Parse `@menu`, `@option` and `@template` comments.
- Read type, description, default and enum choices from nixd, with the
  `nix eval` fallback.
- Render the same widget set: switch, dropdown, radio, spin, slider, text,
  multiline, path, package, packages, list, color, submodule, raw and the
  Ojuny-specific widgets where enabled.
- Stage edits until Save; save all files or none.
- Parse-check with `rnix` and format with `nixfmt`.
- Add a file from a template and add an option to a module body.
- Descriptor sidecars for freeform settings.
- Git pull, diff, commit and push.
- Optional apply commands for values that can change without a rebuild.
- Host and system filtering that follows the module import graph.

### Differences from the desktop GUI

- No GTK or libadwaita dependency.
- A browser client and an HTTP API.
- Remote access behind explicit configuration.
- Authentication and session handling.
- A server-side file root that cannot be escaped.
- Multiple concurrent browser sessions, with conflict handling still to be
  decided.
- No desktop-specific integrations such as wallpaper previews that depend on a
  local GTK widget; these become preview endpoints or are omitted.

### Security

- Localhost binding by default.
- A token or session login before any file access.
- All paths resolved under the configured root; no traversal.
- CSRF protection and strict content security policy.
- No arbitrary command execution except an explicitly configured apply command.
- Secrets and hashes are never rendered in diffs or logs.
- TLS is terminated by a reverse proxy, not implemented ad hoc.
- The server refuses to start without an authentication method when bound to a
  non-loopback address.

### Documentation

- The HTTP API and its versioning.
- How to run it locally, as a service and behind a proxy.
- The authentication and permission model.
- The widget mapping and which widgets are unavailable in the browser.
- The descriptor sidecar format.
- The module shape rules and the list limitation below.

### Tests

- Scanner, DSL, model and value tests shared with the desktop implementation.
- Metadata lookup with nixd and the eval fallback.
- Span-patch save, parse check and format tests.
- Path traversal and authentication tests.
- A full browser flow: open, edit, save, diff and rebuild.
- A save failure leaves every file unchanged.
- A descriptor for a freeform value appears and saves.

## Documented limitation

The scanner binds an `@option` comment to the next assignment in the same
container. It does not descend into a list of attribute sets. An annotation
inside `lib.mkMerge [...]`, a list literal, `map`, `forEach` or
`lib.optionalAttrs` is ignored. `lib.mkIf` is supported, and options may sit
directly inside a `lib.mkIf` attribute set.

The fix for an invisible option is always to change the module shape. The web
GUI must not modify the scanner to add list or apply-node descent. The
authoritative documents are `ojuny-template/docs/05-settings-gui-limitations.md`
and `nix-settings-gui/docs/comment-syntax.md`.

## Limits and non-goals

- No live collaborative editing in the first scope.
- No remote Nix build service. Rebuilds still run on the host that owns the
  flake, or the UI only writes files.
- No code-server or terminal replacement.
- No editing outside the configured root.
- No bypass of the module shape rules.
- No built-in TLS certificate authority.
- No public multi-tenant deployment.
- No guarantee that every custom desktop widget is available in a browser.

## Open questions

- Frontend framework and whether to share components with `ojuny-fleet-manager`.
- Authentication model: single token or accounts declared in Nix.
- Whether rebuilds are triggered from the browser or left to the user.
- How concurrent edits are detected and resolved.
- Whether the Ojuny-specific widgets are included by default or behind a
  feature flag.
- Whether a container image is part of the first release.

## Sources

- `nix-settings-gui/README.md`, `docs/architecture.md`,
  `docs/comment-syntax.md`, `docs/descriptor-schema.md`,
  `docs/gui-design.md`, `docs/usage.md`.
- `ojuny-template/docs/05-settings-gui-limitations.md`.
