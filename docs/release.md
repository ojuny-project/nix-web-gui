# Release

This document describes what a tagged release needs and what it produces. It
does not change the flake. The maintainer edits `flake.nix` and `flake.lock` and
the workflow in `/home/den/repos/ojuny-future-repos/nix-web-gui/.github/workflows/release.yml`
does the rest.

Nothing here runs in this repository today: `flake.nix` still pins the engine
with a local path and a `v*` tag builds only what the flake exposes.

## Release inputs

### Remote `nix-settings-gui` input

The flake input is a local path:

```nix
nix-settings-gui.url = "git+file:///home/den/repos/nix-settings-gui";
```

A release build must replace it with a remote input before the tag is pushed,
because the GitHub runner has no `/home/den/repos/nix-settings-gui`:

```nix
nix-settings-gui.url = "github:ojuny-project/nix-settings-gui";
```

`nix/package.nix` copies `${nixSettingsGui}` into `vendor/nix-settings-gui` and
rewrites `engine/Cargo.toml` from `path = "../../../nix-settings-gui"` to
`path = "../vendor/nix-settings-gui"`. Any input whose source tree contains the
`engine` crate works, so the URL may be a GitHub repository, another git forge,
or a tarball.

### Revision pin

The input is pinned by `flake.lock`, not by the URL. The lock currently records:

| Input | Locked value |
|---|---|
| `nix-settings-gui` | `git` at `file:///home/den/repos/nix-settings-gui`, rev `95779de003c77f7e90b7477f27e9b7096de8789a`, ref `refs/heads/main`, 111 commits |

After changing the URL, regenerate the lock and commit it with the flake:

```sh
nix flake lock --update-input nix-settings-gui
git add flake.nix flake.lock
```

The tag is reproducible only when `flake.lock` is committed. A tag that points
at a commit whose lock still names `file:///home/den/repos/nix-settings-gui`
fails on the runner.

There is a conflict between the lock and the prose in
`/home/den/repos/ojuny-future-repos/nix-web-gui/README.md`,
`/home/den/repos/ojuny-future-repos/nix-web-gui/docs/deployment.md`, and
`/home/den/repos/ojuny-future-repos/nix-web-gui/docs/parity.md`, which name NSG
The lock and prose now name `95779de`. Check the locked rev before cutting a tag
instead of trusting the prose.

## Tagged artifacts

The `release` workflow runs on `v*` tags on `ubuntu-latest` and publishes:

| Artifact | Contents |
|---|---|
| `nix-web-gui-<tag>-x86_64-linux.tar.gz` | `default/` and `minimal/`, each with `bin/nix-web-gui` and `share/nix-web-gui` |
| `nix-web-gui-<tag>-x86_64-linux-container.tar.gz` | Optional OCI image tarball, present only when `packages.x86_64-linux.container` exists |
| `SHA256SUMS` | One line per published tarball, generated with `sha256sum` |

`default/` is `packages.default` (`withOjunyWidgets = true`). `minimal/` is
`packages.nix-web-gui-minimal` (`--no-default-features --features pam`,
`VITE_OJUNY_WIDGETS=0`). The tarball is not a Nix closure: it contains the
wrapped binary and the static assets, so a consumer copies it into place rather
than substituting it for the package.

The image step probes the flake first:

```sh
nix eval --raw .#packages.x86_64-linux.container.name
```

When the attribute is missing the step prints a skip message and the release
keeps the tagged tarball and `SHA256SUMS` behaviour. When it exists the tarball
is added to `SHA256SUMS` and to the `gh release create` asset list. The workflow
references `packages.<system>.container` but does not create it.

## Optional container

`/home/den/repos/ojuny-future-repos/nix-web-gui/nix/container.nix` is a
`dockerTools.buildLayeredImage` expression with the arguments `pkgs`, `package`,
and optional `name` and `tag`. It runs:

```sh
nix-web-gui serve --listen 0.0.0.0:8787 --static-dir <package>/share/nix-web-gui/dist
```

and exposes `8787/tcp`. The maintainer wires it into the per-system output:

```nix
packages = {
  default = mkPackage { };
  nix-web-gui = mkPackage { };
  nix-web-gui-minimal = mkPackage {
    withOjunyWidgets = false;
    pname = "nix-web-gui-minimal";
  };
  container = import ./nix/container.nix {
    inherit pkgs;
    package = self.packages.${system}.default;
  };
};
```

Then build and load it by hand:

```sh
nix build .#packages.x86_64-linux.container
docker load < result
docker run --rm -p 127.0.0.1:8787:8787 nix-web-gui:latest
```

PAM does not work inside the container without host PAM configuration. The
image carries no `/etc/pam.d`, no account database beyond `dockerTools.fakeNss`,
and no shadow files, so the default PAM-backed login fails. The image is for
`--dev-auth` experiments or for deployments where an authenticating reverse
proxy fronts the listener. Per-user workers, the group allowlist, and every
root-parent action depend on the host system and do not cross into the
container. For PAM-backed use, deploy `services.nix-web-gui` on NixOS instead.

The container also has no initial flake. Pass `--flake`, or bind-mount the
configuration directory and use the UI's Open flake control. Nothing in the
image persists between runs, so theme writes, staged edits, and git state are
lost with the container.

## Missing `binary-url.json` contract

`binary-url.json` is not implemented. There is no schema, no published file, and
no in-app update check against it.

`/home/den/repos/ojuny-future-repos/nix-web-gui/TODO.md` lists
`Add binary-url.json and in-app update checks for prebuilt binaries` as open.
`/home/den/repos/ojuny-future-repos/nix-web-gui/README.md` lists both under "Not
yet implemented". The update check that exists today is a `nix flake update`
dry run against the open flake, not a prebuilt-binary check.

The schema is undecided, so no release document, workflow step, or artifact
defines one. Until the format is fixed, do not add a `binary-url.json` to a
release, do not parse one, and do not point an updater at the tarballs above.
The tarball names, the `SHA256SUMS` format, and the image tarball are release
assets, not an update protocol. A consumer that needs to know the current
version reads the tag and the checksum file.
