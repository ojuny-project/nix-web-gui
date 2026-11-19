#!/usr/bin/env bash
# Run the CDP browser smoke test against a built package and a writable
# fixture copy. Arguments: package chromium node fixture smoke-script.
set -euo pipefail

package=$1
chromium=$2
node=$3
fixture=$4
smoke=$5

work=$(mktemp -d)
port=$(( 20000 + $$ % 20000 ))
cdp=$(( port + 1 ))

cp -r "$fixture" "$work/flake"
chmod -R u+w "$work/flake"
mkdir -p "$work/home" "$work/tmp"

export HOME="$work/home"
export XDG_CACHE_HOME="$work/home/.cache"
export TMPDIR="$work/tmp"
export NIX_WEB_GUI_SKIP_METADATA=1

mkdir -p "$work/home/.config/nix-settings-gui"
printf '%s\n' '{"rebuild_command":"echo rebuilt","live_apply":true}' \
  >"$work/home/.config/nix-settings-gui/config.json"

server_log="$work/server.log"
chrome_log="$work/chrome.log"

"$package/bin/nix-web-gui" serve \
  --listen "127.0.0.1:$port" \
  --dev-auth \
  --dev-password secret \
  --flake "$work/flake" \
  --static-dir "$package/share/nix-web-gui/dist" \
  >"$server_log" 2>&1 &
server_pid=$!
chrome_pid=""

cleanup() {
  if [ -n "$chrome_pid" ]; then
    kill "$chrome_pid" 2>/dev/null || true
  fi
  kill "$server_pid" 2>/dev/null || true
}
trap cleanup EXIT

for _ in $(seq 1 100); do
  if curl -fsS "http://127.0.0.1:$port/api/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.2
done
if ! curl -fsS "http://127.0.0.1:$port/api/health" >/dev/null; then
  cat "$server_log"
  echo "the server did not start" >&2
  exit 1
fi

"$chromium/bin/chromium" \
  --headless=new \
  --no-sandbox \
  --disable-dev-shm-usage \
  --disable-gpu \
  --no-first-run \
  --user-data-dir="$work/chrome" \
  --remote-debugging-port="$cdp" \
  about:blank \
  >"$chrome_log" 2>&1 &
chrome_pid=$!

for _ in $(seq 1 150); do
  if curl -fsS "http://127.0.0.1:$cdp/json/version" >/dev/null 2>&1; then
    break
  fi
  sleep 0.2
done
if ! curl -fsS "http://127.0.0.1:$cdp/json/version" >/dev/null; then
  cat "$chrome_log"
  echo "chromium did not expose the debugging port" >&2
  exit 1
fi

NWG_BASE="http://127.0.0.1:$port" \
NWG_CDP="http://127.0.0.1:$cdp" \
NWG_FIXTURE="$work/flake" \
"$node/bin/node" "$smoke"
