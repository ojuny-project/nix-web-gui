#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "boundary check failed: $*" >&2
  exit 1
}

if grep -Eq '^(axum|axum-extra|pam-client|tower-http)[[:space:]]*=' engine/Cargo.toml; then
  fail "engine/Cargo.toml depends on a server crate"
fi

if grep -R -Eq 'from "(@nix-web-gui/app|\.\./\.\./app)|/api/|new EventSource|fetch\(' frontend/packages/ui/src; then
  fail "frontend/packages/ui reaches into the app or the API"
fi

echo "engine and UI boundaries are clean"
