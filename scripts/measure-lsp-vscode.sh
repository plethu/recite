#!/usr/bin/env bash
set -euo pipefail

# The caller supplies a verified official host, generated project and built VSIX.
if [[ $# != 4 ]]; then
  echo "Usage: measure-lsp-vscode.sh HOST_BINARY PROJECT RELEASE_LSP OUTPUT_JSON" >&2
  exit 2
fi
repo_root="$(git rev-parse --show-toplevel)"
host="$(realpath "$1")"
project="$(realpath "$2")"
binary="$(realpath "$3")"
output="$(realpath -m "$4")"
profile="$(mktemp -d "${TMPDIR:-/tmp}/recite-vscode-perf.XXXXXX")"
runner=""
cleanup() {
  if [[ -n "$runner" ]] && kill -0 -- "-$runner" 2>/dev/null; then
    kill -TERM -- "-$runner" 2>/dev/null || true
    sleep 1
    kill -KILL -- "-$runner" 2>/dev/null || true
  fi
  rm -rf "$profile"
}
trap cleanup EXIT
mkdir -p "$profile/runtime" "$profile/probe" "$(dirname "$output")"
chmod 700 "$profile/runtime"
cp "$repo_root/tests/editor-hosts/vscode/latency-probe.cjs" "$profile/probe/latency-probe.cjs"
cp "$repo_root/tests/editor-hosts/vscode/render-probe.cjs" "$profile/probe/render-probe.cjs"
render_args=()
if [[ "${RECITE_PERF_RENDER:-0}" == 1 ]]; then
  export RECITE_PERF_CDP_PORT_FILE="$profile/user-data/DevToolsActivePort"
  render_args=(--remote-debugging-port=0 --remote-debugging-address=127.0.0.1)
else
  unset RECITE_PERF_CDP_PORT_FILE
fi
printf '%s\n' '{"name":"recite-latency-probe","version":"0.0.0","engines":{"vscode":"^1.89.0"}}' > "$profile/probe/package.json"
export XDG_CONFIG_HOME="$profile/config" XDG_CACHE_HOME="$profile/cache"
export XDG_STATE_HOME="$profile/state" XDG_RUNTIME_DIR="$profile/runtime"
export VSCODE_PORTABLE="$profile/portable"
"$(dirname "$host")/bin/$(basename "$host")" --no-sandbox --user-data-dir="$profile/user-data" --extensions-dir="$profile/extensions" \
  --install-extension "$repo_root/editors/vscode/recite-vscode-0.1.0.vsix" --force
setsid env -u DISPLAY -u WAYLAND_DISPLAY WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_XWAYLAND=0 \
  RECITE_PERF_ROOT="$project" RECITE_PERF_BINARY="$binary" RECITE_PERF_OUTPUT="$output" \
  timeout --kill-after=10s 180s cage -- "$host" "${render_args[@]}" --no-sandbox --disable-gpu --disable-updates \
  --disable-telemetry --disable-crash-reporter --skip-welcome --skip-release-notes \
  --password-store=basic --user-data-dir="$profile/user-data" --extensions-dir="$profile/extensions" \
  --extensionDevelopmentPath="$profile/probe" --extensionTestsPath="$profile/probe/latency-probe.cjs" \
  --disable-workspace-trust "$project" &
runner=$!
wait "$runner"
test -s "$output"
