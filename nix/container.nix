# OCI image for nix-web-gui.
#
# The image runs the same `serve` subcommand as the NixOS module, with the
# static assets from the package closure and the listener on 0.0.0.0:8787 so a
# reverse proxy or `docker run -p 127.0.0.1:8787:8787` can reach it.
#
# PAM does not work inside the container unless the host PAM stack, the account
# database, and shadow files are bind-mounted and the container runs with the
# privileges PAM needs. `--pam-service` therefore fails at login in a plain
# container. This image is for `--dev-auth` experiments or for deployments where
# an authenticating reverse proxy fronts the listener. Use the NixOS module
# (`services.nix-web-gui`) for PAM-backed, per-user workers.
#
# The flake does not expose this file yet. A maintainer wires it as
# `packages.<system>.container = import ./nix/container.nix { inherit pkgs;
# package = self.packages.${system}.default; };`.
{
  pkgs,
  package,
  name ? "nix-web-gui",
  tag ? "latest",
}:

pkgs.dockerTools.buildLayeredImage {
  inherit name tag;

  contents = [
    pkgs.dockerTools.fakeNss
    pkgs.dockerTools.caCertificates
    package
  ];

  config = {
    Cmd = [
      "${package}/bin/nix-web-gui"
      "serve"
      "--listen"
      "0.0.0.0:8787"
      "--static-dir"
      "${package}/share/nix-web-gui/dist"
    ];

    ExposedPorts = {
      "8787/tcp" = { };
    };

    Env = [
      "SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt"
    ];

    WorkingDir = "/";
  };
}
