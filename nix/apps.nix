# Launchers for `nix run`.
#
# Both scripts `exec` the wrapped server binary, so the per-user worker sees the
# real binary through `/proc/self/exe` instead of this launcher.
{
  pkgs,
  package,
}:
let
  staticDir = "${package}/share/nix-web-gui/dist";
  binary = "${package}/bin/nix-web-gui";
in
{
  production = pkgs.writeShellScriptBin "nix-web-gui" ''
    set -euo pipefail

    has_flag() {
      local name=$1
      shift
      for arg in "$@"; do
        [ "$arg" = "$name" ] && return 0
        case "$arg" in "$name="*) return 0;; esac
      done
      return 1
    }

    if [ "$(id -u)" -ne 0 ]; then
      echo "nix-web-gui: production mode needs root for PAM and per-user workers; run it with sudo" >&2
      exit 1
    fi

    args=(serve)
    if ! has_flag --static-dir "$@"; then
      args+=(--static-dir ${staticDir})
    fi

    if ! has_flag --pam-service "$@"; then
      if [ -f /etc/pam.d/nix-web-gui ]; then
        args+=(--pam-service nix-web-gui)
      else
        args+=(--pam-service login)
      fi
    fi

    exec ${binary} "''${args[@]}" "$@"
  '';

  demo = pkgs.writeShellScriptBin "nix-web-gui-demo" ''
    set -euo pipefail

    has_flag() {
      local name=$1
      shift
      for arg in "$@"; do
        [ "$arg" = "$name" ] && return 0
        case "$arg" in "$name="*) return 0;; esac
      done
      return 1
    }

    args=(serve)
    if ! has_flag --static-dir "$@"; then
      args+=(--static-dir ${staticDir})
    fi
    if ! has_flag --dev-auth "$@"; then
      args+=(--dev-auth)
    fi
    if ! has_flag --dev-password "$@"; then
      args+=(--dev-password demo)
    fi

    exec ${binary} "''${args[@]}" "$@"
  '';
}
