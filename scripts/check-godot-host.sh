#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/check-godot-host.sh

Runs the Godot GDExtension host conformance project. Requires the official
Godot 4.6.3 stable Linux x86_64 standard build, available as `godot` or via
GODOT=/path/to/Godot_v4.6.3-stable_linux.x86_64.

CARGO_TARGET_DIR may point to an absolute directory or a path relative to the
repository root.
EOF
}

case "${1:-}" in
  -h|--help)
    usage
    exit 0
    ;;
  "") ;;
  *)
    echo "unknown argument: $1" >&2
    usage >&2
    exit 2
    ;;
esac

repo_root="$(git rev-parse --show-toplevel)"
godot_bin="${GODOT:-}"
required_godot_version="4.6.3.stable.official.7d41c59c4"
required_godot_url="https://godotengine.org/download/archive/4.6.3-stable/"

if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
  echo "Godot host checks currently require Linux x86_64 because the test extension is a .so." >&2
  exit 2
fi

if [[ -z "$godot_bin" ]]; then
  godot_bin="$(command -v godot || true)"
fi
if [[ -z "$godot_bin" || ! -x "$godot_bin" ]]; then
  echo "Godot host checks require official Godot ${required_godot_version}." >&2
  echo "Install the Linux x86_64 standard build from ${required_godot_url}" >&2
  echo "or set GODOT=/path/to/Godot_v4.6.3-stable_linux.x86_64." >&2
  exit 2
fi

godot_version="$("$godot_bin" --headless --version 2>/dev/null | tail -n 1)"
if [[ "$godot_version" != "$required_godot_version" ]]; then
  echo "Godot host checks require ${required_godot_version}; found ${godot_version:-unknown}." >&2
  echo "Use the official stable build from ${required_godot_url}." >&2
  exit 2
fi

command -v cargo >/dev/null 2>&1 || {
  echo "Godot host checks require cargo to build recite-godot and compile the fixture." >&2
  exit 2
}
command -v timeout >/dev/null 2>&1 || {
  echo "Godot host checks require the Linux coreutils timeout command." >&2
  exit 2
}

if [[ -n "${CARGO_TARGET_DIR:-}" && "${CARGO_TARGET_DIR}" != /* ]]; then
  cargo_target_dir="$repo_root/${CARGO_TARGET_DIR}"
else
  cargo_target_dir="${CARGO_TARGET_DIR:-$repo_root/target}"
fi
export CARGO_TARGET_DIR="$cargo_target_dir"
tmpdir="$(mktemp -d /tmp/recite-godot-host.XXXXXX)"
trap 'rm -rf "$tmpdir"' EXIT

mkdir -p "$tmpdir/bin" "$tmpdir/dialogue" "$tmpdir/home" "$tmpdir/cache" "$tmpdir/config" "$tmpdir/data"
cp -R "$repo_root/tests/godot-host/." "$tmpdir/"

echo "== build Godot GDExtension ==" >&2
cargo build --locked -p recite-godot --quiet
extension="$CARGO_TARGET_DIR/debug/librecite_godot.so"
if [[ ! -f "$extension" ]]; then
  echo "missing built Godot extension: $extension" >&2
  exit 1
fi
cp "$extension" "$tmpdir/bin/librecite_godot.so"

echo "== compile host fixture ==" >&2
cargo run --locked --quiet --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  compile --output "$tmpdir/dialogue/basic.recitec" \
  "$repo_root/examples/godot/basic-dialogue/dialogue/basic.recite"

godot_env=(
  env
  "HOME=$tmpdir/home"
  "XDG_CACHE_HOME=$tmpdir/cache"
  "XDG_CONFIG_HOME=$tmpdir/config"
  "XDG_DATA_HOME=$tmpdir/data"
)

run_godot() {
  local log_file="$1"
  shift
  # Run through a child shell so a Godot first-scan abort is captured in the
  # log instead of producing a misleading shell-level core-dump diagnostic.
  bash -c 'exec "$@"' recite-godot-host "$@" >"$log_file" 2>&1 &
  local process_id=$!
  wait "$process_id"
}

echo "== initialize Godot extension registry ==" >&2
if ! run_godot "$tmpdir/editor.log" timeout 30s "${godot_env[@]}" "$godot_bin" \
  --headless --editor --path "$tmpdir" --quit-after 10; then
  # Godot 4.6.3 can abort once while creating a fresh project's class cache;
  # a second scan is stable and is required to succeed before conformance.
  echo "Godot editor registry scan did not finish; retrying once." >&2
  if ! run_godot "$tmpdir/editor-retry.log" timeout 30s "${godot_env[@]}" "$godot_bin" \
    --headless --editor --path "$tmpdir" --quit-after 10; then
    cat "$tmpdir/editor.log" >&2
    cat "$tmpdir/editor-retry.log" >&2
    exit 1
  fi
fi

echo "== run Godot host conformance ==" >&2
if ! run_godot "$tmpdir/runtime.log" timeout 30s "${godot_env[@]}" "$godot_bin" \
  --headless --path "$tmpdir" --quit-after 10; then
  cat "$tmpdir/runtime.log" >&2
  exit 1
fi

if ! grep -Fqx "Godot host conformance passed" "$tmpdir/runtime.log"; then
  echo "Godot host process exited without the conformance success sentinel." >&2
  cat "$tmpdir/runtime.log" >&2
  exit 1
fi
if grep -Eq "SCRIPT ERROR|Parse Error|GDScript backtrace|Godot host conformance failed" "$tmpdir/runtime.log"; then
  echo "Godot host log contains a script or parse error." >&2
  cat "$tmpdir/runtime.log" >&2
  exit 1
fi

cat "$tmpdir/runtime.log"
echo "Godot host checks passed (Godot ${required_godot_version})."
