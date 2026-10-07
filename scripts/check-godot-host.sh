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
  -h | --help)
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
tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/recite-godot-host.XXXXXX")"
package_tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/recite-godot-package-check.XXXXXX")"
watch_pid=""
cleanup() {
  if [[ -n "$watch_pid" ]]; then
    kill "$watch_pid" 2>/dev/null || true
    wait "$watch_pid" 2>/dev/null || true
  fi
  rm -rf "$tmpdir" "$package_tmpdir"
}
trap cleanup EXIT

mkdir -p "$tmpdir/dialogue" "$tmpdir/home" "$tmpdir/cache" "$tmpdir/config" "$tmpdir/data"
cp -R "$repo_root/tests/godot-host/." "$tmpdir/"

echo "== package addon in clean consumer ==" >&2
RECITE_GODOT_PROFILE="${RECITE_GODOT_PROFILE:-debug}" "$repo_root/scripts/package-godot-addon.sh" "$package_tmpdir/first" >&2
printf 'old package file\n' >"$package_tmpdir/first/addons/recite/obsolete-addon.gd"
RECITE_GODOT_PROFILE="${RECITE_GODOT_PROFILE:-debug}" "$repo_root/scripts/package-godot-addon.sh" "$package_tmpdir/first" >&2
test ! -e "$package_tmpdir/first/addons/recite/obsolete-addon.gd"
RECITE_GODOT_PROFILE="${RECITE_GODOT_PROFILE:-debug}" "$repo_root/scripts/package-godot-addon.sh" "$package_tmpdir/second" >&2
if ! cmp -s "$package_tmpdir/first/recite-godot-addon.tar.gz" "$package_tmpdir/second/recite-godot-addon.tar.gz"; then
  echo "Independent Godot package builds produced different archives." >&2
  exit 1
fi
mkdir -p "$package_tmpdir/extracted" "$tmpdir/addons"
tar -xzf "$package_tmpdir/first/recite-godot-addon.tar.gz" -C "$package_tmpdir/extracted"
test -f "$package_tmpdir/extracted/addons/recite/README.md"
test -f "$package_tmpdir/extracted/examples/basic-dialogue/dialogue/basic.recite"
cp -R "$package_tmpdir/extracted/addons/recite" "$tmpdir/addons/"

echo "== compile host fixture ==" >&2
cargo run --locked --quiet --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  compile --output "$tmpdir/dialogue/basic.recitec" \
  "$repo_root/examples/godot/basic-dialogue/dialogue/basic.recite"
cargo run --locked --quiet --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  compile --output "$tmpdir/dialogue/plural.recitec" \
  "$repo_root/fixtures/recite/valid/adapter_conformance/plural_runtime.recite"
cargo run --locked --quiet --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  compile --output "$tmpdir/dialogue/runtime.recitec" \
  "$repo_root/fixtures/recite/valid/adapter_conformance/runtime_surface.recite"
cargo run --locked --quiet --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  compile --output "$tmpdir/dialogue/runtime_changed.recitec" \
  "$repo_root/fixtures/recite/valid/adapter_conformance/runtime_surface_changed.recite"
cat >"$tmpdir/schema_a.toml" <<'EOF'
schema_version = 1
[producer]
id = "godot-host-schema"
[types.route]
kind = "enum"
values = ["left"]
EOF
cat >"$tmpdir/schema_b.toml" <<'EOF'
schema_version = 1
[producer]
id = "godot-host-schema"
[types.route]
kind = "enum"
values = ["right"]
EOF
cargo run --locked --quiet --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  export-schema --schema "$tmpdir/schema_a.toml" --output "$tmpdir/schema_a.json"
cargo run --locked --quiet --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  export-schema --schema "$tmpdir/schema_b.toml" --output "$tmpdir/schema_b.json"
cargo run --locked --quiet --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  compile --schema "$tmpdir/schema_a.json" --output "$tmpdir/dialogue/schema_prompt.recitec" \
  "$tmpdir/dialogue/schema_prompt.recite"
cp "$tmpdir/dialogue/schema_prompt.recitec" "$tmpdir/dialogue/schema_a.recitec"
cargo run --locked --quiet --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  compile --schema "$tmpdir/schema_b.json" --output "$tmpdir/dialogue/schema_prompt.recitec" \
  "$tmpdir/dialogue/schema_prompt.recite"
cp "$tmpdir/dialogue/schema_prompt.recitec" "$tmpdir/dialogue/schema_b.recitec"

godot_env=(
  env
  "HOME=$tmpdir/home"
  "XDG_CACHE_HOME=$tmpdir/cache"
  "XDG_CONFIG_HOME=$tmpdir/config"
  "XDG_DATA_HOME=$tmpdir/data"
  "RECITE_CLI=$CARGO_TARGET_DIR/debug/recite"
)

run_godot() {
  local log_file="$1"
  shift
  # Keep each Godot invocation's output in its own log and propagate its
  # exit status, including signal failures, to the host gate.
  bash -c 'exec "$@"' recite-godot-host "$@" >"$log_file" 2>&1 &
  local process_id=$!
  wait "$process_id"
}

retain_editor_log() {
  local label="$1"
  local log_file="$2"
  local diagnostics_root="$cargo_target_dir/godot-host-diagnostics"
  mkdir -p "$diagnostics_root"
  local diagnostics_dir
  diagnostics_dir="$(mktemp -d "$diagnostics_root/run.XXXXXX")"
  cp "$log_file" "$diagnostics_dir/$label.log"
  echo "Godot $label log retained at $diagnostics_dir/$label.log" >&2
}

run_editor_scan() {
  local label="$1"
  local project_dir="$2"
  local log_file="$3"
  local status

  # A short grace period lets Godot finish deferred extension documentation
  # work before the fresh editor shuts down (godotengine/godot#111645).
  if run_godot "$log_file" timeout 30s "${godot_env[@]}" "$godot_bin" \
    --headless --editor --path "$project_dir" --quit-after 60 --frame-delay 10; then
    return 0
  else
    status=$?
  fi

  retain_editor_log "$label" "$log_file"
  echo "Godot $label editor scan failed (exit $status)." >&2
  cat "$log_file" >&2
  return 1
}

echo "== initialize Godot extension registry ==" >&2
run_editor_scan "registry" "$tmpdir" "$tmpdir/editor.log"

echo "== run Godot host conformance ==" >&2
if ! run_godot "$tmpdir/runtime.log" timeout 30s "${godot_env[@]}" "$godot_bin" \
  --headless --path "$tmpdir" --quit-after 10; then
  cat "$tmpdir/editor.log" >&2
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

echo "== reject changed compiled import and check last-good cache ==" >&2
printf '\001\002\003' >"$tmpdir/dialogue/basic.recitec"
# This project has completed its first editor scan. Godot's --import waits for
# the changed asset's import to finish before exiting; --quit-after counts
# frames and can stop the scan early on a slower host.
if run_godot "$tmpdir/reject-editor.log" timeout 30s "${godot_env[@]}" "$godot_bin" \
  --headless --import --path "$tmpdir"; then
  :
else
  import_status=$?
  retain_editor_log "reject-import" "$tmpdir/reject-editor.log"
  echo "Godot rejected import scan failed (exit $import_status)." >&2
  cat "$tmpdir/reject-editor.log" >&2
  exit 1
fi
if ! grep -Fq "Recite import" "$tmpdir/reject-editor.log"; then
  retain_editor_log "reject-import" "$tmpdir/reject-editor.log"
  echo "Godot editor did not report the rejected compiled candidate." >&2
  cat "$tmpdir/reject-editor.log" >&2
  exit 1
fi
if grep -Fq "Scan thread aborted" "$tmpdir/reject-editor.log"; then
  retain_editor_log "reject-import" "$tmpdir/reject-editor.log"
  echo "Godot changed-asset import scan aborted." >&2
  cat "$tmpdir/reject-editor.log" >&2
  exit 1
fi
if ! run_godot "$tmpdir/rejected-runtime.log" timeout 30s "${godot_env[@]}" "$godot_bin" \
  --headless --path "$tmpdir" --script res://check_rejected_import.gd; then
  cat "$tmpdir/reject-editor.log" >&2
  cat "$tmpdir/rejected-runtime.log" >&2
  exit 1
fi
if ! grep -Fqx "Godot rejected import retained last-good" "$tmpdir/rejected-runtime.log"; then
  cat "$tmpdir/rejected-runtime.log" >&2
  exit 1
fi
if grep -Eq "SCRIPT ERROR|Parse Error|GDScript backtrace" "$tmpdir/rejected-runtime.log"; then
  cat "$tmpdir/rejected-runtime.log" >&2
  exit 1
fi
cat "$tmpdir/rejected-runtime.log"

echo "== run packaged basic example in clean consumer ==" >&2
mkdir -p "$tmpdir/example/addons"
cp -R "$package_tmpdir/extracted/examples/basic-dialogue/." "$tmpdir/example/"
cp -R "$package_tmpdir/extracted/addons/recite" "$tmpdir/example/addons/"
cp "$repo_root/tests/godot-host/check_example.gd" "$tmpdir/example/"
# Wait for the protocol's completion event, rather than consuming a fixed sleep
# window. Parse complete JSON before stopping the producer, so a partial record
# cannot be mistaken for readiness. The final check validates terminal success.
timeout 30s "$CARGO_TARGET_DIR/debug/recite" watch --output-format structured \
  "$tmpdir/example" >"$tmpdir/example-watch.ndjson" 2>"$tmpdir/example-watch.stderr" &
watch_pid=$!
watch_deadline=$((SECONDS + 30))
watch_ready=false
while kill -0 "$watch_pid" 2>/dev/null && ((SECONDS < watch_deadline)); do
  if jq -e -s 'any(.[]; .event == "watch.build.completed")' "$tmpdir/example-watch.ndjson" >/dev/null 2>&1; then
    watch_ready=true
    break
  fi
  sleep 0.1
done
kill "$watch_pid" 2>/dev/null || true
wait "$watch_pid" 2>/dev/null || true
watch_pid=""
if [[ "$watch_ready" != true || ! -s "$tmpdir/example/dialogue/basic.recitec" ]] \
  || ! jq -e -s 'map(select(.event == "watch.build.completed")) | length > 0 and all(.[]; .data.status == "succeeded")' "$tmpdir/example-watch.ndjson" >/dev/null; then
  echo "Packaged example watcher did not build its compiled dialogue." >&2
  cat "$tmpdir/example-watch.ndjson" >&2
  cat "$tmpdir/example-watch.stderr" >&2
  exit 1
fi
run_editor_scan "example" "$tmpdir/example" "$tmpdir/example-editor.log"
if ! run_godot "$tmpdir/example-runtime.log" timeout 30s "${godot_env[@]}" "$godot_bin" \
  --headless --path "$tmpdir/example" --script res://check_example.gd; then
  cat "$tmpdir/example-runtime.log" >&2
  exit 1
fi
if ! grep -Fqx "Godot packaged example started" "$tmpdir/example-runtime.log" \
  || grep -Eq "SCRIPT ERROR|Parse Error|GDScript backtrace" "$tmpdir/example-runtime.log"; then
  cat "$tmpdir/example-runtime.log" >&2
  exit 1
fi
cat "$tmpdir/example-runtime.log"

echo "== replace old addon while retaining authored project data ==" >&2
mkdir -p "$tmpdir/upgrade/addons"
cp -R "$package_tmpdir/extracted/examples/basic-dialogue/." "$tmpdir/upgrade/"
cp -R "$package_tmpdir/extracted/addons/recite" "$tmpdir/upgrade/addons/"
printf 'obsolete addon code\n' >"$tmpdir/upgrade/addons/recite/obsolete-addon.gd"
printf 'author-owned dialogue data\n' >"$tmpdir/upgrade/dialogue/owner-keep.txt"
cp "$repo_root/tests/godot-host/check_example.gd" "$tmpdir/upgrade/"
cargo run --locked --quiet --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  compile --output "$tmpdir/upgrade/dialogue/basic.recitec" \
  "$tmpdir/upgrade/dialogue/basic.recite"
authored_before="$(sha256sum "$tmpdir/upgrade/dialogue/owner-keep.txt" "$tmpdir/upgrade/dialogue/basic.recite" "$tmpdir/upgrade/dialogue/basic.recitec")"
rm -rf "$tmpdir/upgrade/addons/recite"
cp -R "$package_tmpdir/extracted/addons/recite" "$tmpdir/upgrade/addons/"
test ! -e "$tmpdir/upgrade/addons/recite/obsolete-addon.gd"
test "$(sha256sum "$tmpdir/upgrade/dialogue/owner-keep.txt" "$tmpdir/upgrade/dialogue/basic.recite" "$tmpdir/upgrade/dialogue/basic.recitec")" = "$authored_before"
run_editor_scan "upgrade" "$tmpdir/upgrade" "$tmpdir/upgrade-editor.log"
if ! run_godot "$tmpdir/upgrade-runtime.log" timeout 30s "${godot_env[@]}" "$godot_bin" \
  --headless --path "$tmpdir/upgrade" --script res://check_example.gd; then
  cat "$tmpdir/upgrade-runtime.log" >&2
  exit 1
fi
if ! grep -Fqx "Godot packaged example started" "$tmpdir/upgrade-runtime.log" \
  || grep -Eq "SCRIPT ERROR|Parse Error|GDScript backtrace" "$tmpdir/upgrade-runtime.log"; then
  cat "$tmpdir/upgrade-runtime.log" >&2
  exit 1
fi
cat "$tmpdir/upgrade-runtime.log"
echo "Godot host checks passed (Godot ${required_godot_version})."
