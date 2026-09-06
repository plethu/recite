#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-vscode-host.sh [repo-root]

Downloads pinned official Linux x64 builds of VS Code and VSCodium into a
temporary directory, installs the local Recite VSIX into isolated profiles,
and runs the extension through each host's real extension-test API. Set
RECITE_VSCODE_HOST_BIN and/or RECITE_VSCODIUM_HOST_BIN to use an already
downloaded official extraction.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi
if (( $# > 1 )); then
  usage >&2
  exit 2
fi

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  repo_root="$(git -C "$input_root" rev-parse --show-toplevel)"
else
  repo_root="$(git rev-parse --show-toplevel)"
fi

if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
  echo "installed VS Code host evidence currently requires Linux x86_64" >&2
  exit 2
fi

for command in awk cargo cage chmod cp git node pnpm ps rm setsid sleep tar tr; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "VS Code host evidence requires $command" >&2
    exit 2
  fi
done
if ! command -v curl >/dev/null 2>&1; then
  echo "VS Code host evidence requires curl for official host archives" >&2
  exit 2
fi
if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
  echo "VS Code host evidence requires sha256sum or shasum" >&2
  exit 2
fi
if ! command -v cage >/dev/null 2>&1; then
  echo "VS Code host evidence requires cage for an isolated headless Wayland boundary" >&2
  exit 2
fi

hash_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

VSCODE_VERSION="1.136.1"
VSCODE_COMMIT="a44adf7f53e00964ab890f9f8758a334f1fc15bc"
VSCODE_URL="https://vscode.download.prss.microsoft.com/dbazure/download/stable/${VSCODE_COMMIT}/code-stable-x64-1788413682.tar.gz"
VSCODE_SHA256="9b4a54f0d49beaa413eda137d00c6541a639300d479efcac566ad13419409218"
VSCODIUM_VERSION="1.126.04524"
VSCODIUM_COMMIT="4c0b0c6cc561d2d3636d1ec250935431876ce4dc"
VSCODIUM_URL="https://github.com/VSCodium/vscodium/releases/download/${VSCODIUM_VERSION}/VSCodium-linux-x64-${VSCODIUM_VERSION}.tar.gz"
VSCODIUM_SHA256="adf3548df055d18e476cdee887488ba7486b879ad99a31a546c6b5c5ff296c24"

host_temp_root="${RECITE_HOST_TMPDIR:-/tmp}"
if [[ ! -d "$host_temp_root" || ! -w "$host_temp_root" ]]; then
  echo "host evidence temporary root is not writable: $host_temp_root" >&2
  exit 2
fi
run_root="$(mktemp -d "$host_temp_root/recite-vscode-host.XXXXXX")"
runner_pid=""
runner_pgid=""
script_pgid="$(ps -o pgid= -p "$$" | tr -d ' ')"

group_processes() {
  local process_group="$1"
  if [[ ! "$process_group" =~ ^[0-9]+$ || "$process_group" -le 1 ]]; then
    return 0
  fi
  ps -eo pid=,ppid=,pgid=,comm=,args= | awk \
    -v process_group="$process_group" -v self="$$" \
    '$3 == process_group && $1 != self { print }'
}

cleanup_process_group() {
  local process_group="$1"
  if [[ ! "$process_group" =~ ^[0-9]+$ || "$process_group" -le 1 ]]; then
    return 0
  fi
  if [[ "$process_group" == "$script_pgid" ]]; then
    echo "refusing to signal the harness process group $process_group" >&2
    return 1
  fi

  local remaining
  remaining="$(group_processes "$process_group")"
  if [[ -z "$remaining" ]]; then
    return 0
  fi

  echo "host phase left process-group descendants; sending TERM (pgid $process_group):" >&2
  echo "$remaining" >&2
  kill -TERM -- "-$process_group" 2>/dev/null || true
  for _ in {1..30}; do
    remaining="$(group_processes "$process_group")"
    [[ -z "$remaining" ]] && break
    sleep 0.1
  done
  if [[ -n "$remaining" ]]; then
    echo "host phase still has process-group descendants; sending KILL (pgid $process_group):" >&2
    echo "$remaining" >&2
    kill -KILL -- "-$process_group" 2>/dev/null || true
    for _ in {1..20}; do
      remaining="$(group_processes "$process_group")"
      [[ -z "$remaining" ]] && break
      sleep 0.1
    done
  fi
  if [[ -n "$remaining" ]]; then
    echo "unable to clear host process group $process_group:" >&2
    echo "$remaining" >&2
  else
    echo "recovered host process group $process_group; the phase remains failed" >&2
  fi
  return 1
}

cleanup() {
  local status=$?
  if [[ -n "$runner_pgid" ]]; then
    cleanup_process_group "$runner_pgid" || true
  fi
  rm -rf "$run_root"
  exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

download_host() {
  local name="$1"
  local url="$2"
  local expected_hash="$3"
  local host_name="$4"
  local archive="$run_root/${name}.tar.gz"
  local extraction="$run_root/$name"
  local actual_hash

  echo "== download $name from official source ==" >&2
  curl --fail --location --silent --show-error --retry 2 "$url" -o "$archive"
  actual_hash="$(hash_file "$archive")"
  if [[ "$actual_hash" != "$expected_hash" ]]; then
    echo "$name archive hash mismatch: expected $expected_hash, got $actual_hash" >&2
    return 1
  fi
  mkdir -p "$extraction"
  tar -xzf "$archive" --strip-components=1 -C "$extraction"
  if [[ ! -x "$extraction/$host_name" ]]; then
    echo "$name archive does not contain executable $host_name" >&2
    return 1
  fi
  printf '%s\n' "$extraction/$host_name"
}

host_bin_from_override() {
  local host_bin="$1"
  if [[ ! -x "$host_bin" ]]; then
    echo "configured host binary is not executable: $host_bin" >&2
    return 1
  fi
  printf '%s\n' "$host_bin"
}

host_version() {
  node -e 'const product = require(process.argv[1]); process.stdout.write(`${product.version}\n`)' "$1"
}

host_commit() {
  node -e 'const product = require(process.argv[1]); process.stdout.write(`${product.commit}\n`)' "$1"
}

host_product_path() {
  local host_bin="$1"
  printf '%s\n' "$(dirname "$host_bin")/resources/app/product.json"
}

if [[ -n "${RECITE_VSCODE_HOST_BIN:-}" ]]; then
  vscode_bin="$(host_bin_from_override "$RECITE_VSCODE_HOST_BIN")"
  vscode_archive_hash="caller-supplied-host-binary"
else
  vscode_bin="$(download_host vscode "$VSCODE_URL" "$VSCODE_SHA256" code)"
  vscode_archive_hash="$VSCODE_SHA256"
fi
if [[ -n "${RECITE_VSCODIUM_HOST_BIN:-}" ]]; then
  vscodium_bin="$(host_bin_from_override "$RECITE_VSCODIUM_HOST_BIN")"
  vscodium_archive_hash="caller-supplied-host-binary"
else
  vscodium_bin="$(download_host vscodium "$VSCODIUM_URL" "$VSCODIUM_SHA256" codium)"
  vscodium_archive_hash="$VSCODIUM_SHA256"
fi

vscode_product="$(host_product_path "$vscode_bin")"
vscodium_product="$(host_product_path "$vscodium_bin")"
for product in "$vscode_product" "$vscodium_product"; do
  if [[ ! -f "$product" ]]; then
    echo "host product metadata is missing: $product" >&2
    exit 1
  fi
done

echo "== build Recite binaries and deterministic VSIX =="
(
  cd "$repo_root"
  cargo build --locked -q -p recite-lsp -p recite-cli
  pnpm --filter recite-vscode run package
)
lsp_bin="${RECITE_LSP_BIN:-$repo_root/target/debug/recite-lsp}"
cli_bin="${RECITE_CLI_BIN:-$repo_root/target/debug/recite}"
vsix="$repo_root/editors/vscode/recite-vscode-0.1.0.vsix"
for executable in "$lsp_bin" "$cli_bin"; do
  if [[ ! -x "$executable" ]]; then
    echo "host evidence requires an executable: $executable" >&2
    exit 2
  fi
done
if [[ ! -f "$vsix" ]]; then
  echo "host evidence requires the packaged VSIX: $vsix" >&2
  exit 2
fi
vsix_hash="$(hash_file "$vsix")"

workspace="$run_root/workspace"
mkdir -p "$workspace/dialogue" "$workspace/scratch" "$workspace/compiled"
cp "$repo_root/fixtures/recite/valid/core_language_spike.recite" "$workspace/dialogue/main.recite"
cp "$repo_root/fixtures/recite/valid/language_pressure.recite" "$workspace/dialogue/pressure.recite"
node - "$workspace/dialogue/main.recite" "$workspace/dialogue/cross-file.recite" <<'EOF'
const fs = require("node:fs");
const [source, destination] = process.argv.slice(2);
fs.writeFileSync(destination, fs.readFileSync(source, "utf8").replace(
  "-> work", "-> dialogue/pressure.recite::letters"
));
EOF
cp "$repo_root/fixtures/recite/invalid/parser_marker_leading_prose.recite" "$workspace/scratch/invalid.recite"
cp "$repo_root/fixtures/recite/invalid/parser_marker_leading_prose.recite" "$workspace/scratch/overlay.recite"
cp "$repo_root/fixtures/recite/invalid/parser_marker_leading_prose.recite" "$workspace/scratch/unicode.recite"
node - "$workspace/scratch/unicode.recite" <<'EOF'
const fs = require("node:fs");
const file = process.argv[2];
const source = fs.readFileSync(file, "utf8").replace("-> East", "-> 😀East");
fs.writeFileSync(file, source.replaceAll("\n", "\r\n"));
EOF
node - "$workspace/dialogue/main.recite" "$workspace/dialogue/repair.recite" <<'EOF'
const fs = require("node:fs");
const [source, destination] = process.argv.slice(2);
fs.writeFileSync(destination, fs.readFileSync(source, "utf8").replace(
  "> intro_001@637b1854a7f3ed42f045 speaker=hazel mood=calm mood=alert", ">"
));
EOF
cat > "$workspace/scratch/runtime-fixture.toml" <<'EOF'
[choices]
start = 1

[effects]
auto_ack_blocking = true
EOF
cat > "$workspace/scratch/runtime-invalid.toml" <<'EOF'
[choices]
start = 0
EOF
cat > "$workspace/recite.project.toml" <<'EOF'
format_version = 1

[discovery]
source_roots = ["dialogue"]

[[scenes]]
id = "scene.start"
asset = "compiled/dialogue.recitec"
block = "start"
participants = ["hazel"]
EOF

asset="$workspace/compiled/dialogue.recitec"
fixture="$workspace/scratch/runtime-fixture.toml"
"$cli_bin" compile --output-format structured --invocation-id host-fixture \
  --output "$asset" "$workspace/dialogue/main.recite" >"$run_root/fixture-compile.jsonl"
node - "$run_root/fixture-compile.jsonl" <<'EOF'
const fs = require("node:fs");
const records = fs.readFileSync(process.argv[2], "utf8").trim().split("\n").map(JSON.parse);
const result = records.at(-1);
if (result?.event !== "command.result" || result.status !== "success") {
  throw new Error("the valid host fixture did not compile");
}
EOF

probe="$run_root/probe"
mkdir -p "$probe"
cp "$repo_root/tests/editor-hosts/vscode/host-probe.cjs" "$probe/host-probe.cjs"
cat > "$probe/package.json" <<'EOF'
{
  "name": "recite-installed-host-probe",
  "version": "0.0.0",
  "main": "./extension.cjs",
  "engines": { "vscode": "^1.89.0" },
  "activationEvents": []
}
EOF
cat > "$probe/extension.cjs" <<'EOF'
exports.activate = () => {};
exports.deactivate = () => {};
EOF

run_host_process() {
  local profile="$1"
  local result="$2"
  local log="$3"
  local host_bin="$4"
  local install_only="$5"
  local candidate_pgid=""

  runner_pgid=""
  set +e
  setsid env -u DISPLAY -u WAYLAND_DISPLAY \
    HOME="$profile/home" \
    XDG_CONFIG_HOME="$profile/config" \
    XDG_DATA_HOME="$profile/data" \
    XDG_CACHE_HOME="$profile/cache" \
    XDG_RUNTIME_DIR="$profile/runtime" \
    VSCODE_PORTABLE="$profile/portable" \
    WLR_BACKENDS=headless \
    WLR_LIBINPUT_NO_DEVICES=1 \
    WLR_XWAYLAND=0 \
    RECITE_HOST_PROBE_RESULT="$result" \
    RECITE_HOST_PROBE_VSIX="$vsix" \
    RECITE_HOST_PROBE_EXTENSIONS="$profile/extensions" \
    RECITE_HOST_PROBE_WORKSPACE="$workspace" \
    RECITE_HOST_PROBE_VALID="$workspace/dialogue/main.recite" \
    RECITE_HOST_PROBE_INVALID="$workspace/scratch/invalid.recite" \
    RECITE_HOST_PROBE_LSP="$lsp_bin" \
    RECITE_HOST_PROBE_CLI="$cli_bin" \
    RECITE_HOST_PROBE_INSTALL_ONLY="$install_only" \
    cage -- \
    "$host_bin" --no-sandbox --disable-gpu --disable-updates --skip-welcome \
      --skip-release-notes --disable-telemetry --disable-crash-reporter \
      --password-store=basic --user-data-dir="$profile/user-data" \
      --extensions-dir="$profile/extensions" --shared-data-dir="$profile/shared" \
      --extensionDevelopmentPath="$probe" --extensionTestsPath="$probe/host-probe.cjs" \
      --disable-workspace-trust "$workspace" >"$log" 2>&1 &
  runner_pid=$!
  for _ in {1..20}; do
    candidate_pgid="$(ps -o pgid= -p "$runner_pid" 2>/dev/null | tr -d ' ')"
    if [[ "$candidate_pgid" =~ ^[0-9]+$ && "$candidate_pgid" -gt 1 &&
      "$candidate_pgid" != "$script_pgid" ]]; then
      runner_pgid="$candidate_pgid"
      break
    fi
    sleep 0.05
  done
  local status=124
  local finished=0
  for _ in {1..3600}; do
    if ! kill -0 "$runner_pid" 2>/dev/null; then
      wait "$runner_pid"
      status=$?
      finished=1
      break
    fi
    sleep 0.05
  done
  if (( finished == 0 )); then
    echo "host phase exceeded the 180-second bound" >&2
  fi
  local process_group="$runner_pgid"
  local group_status=1
  if [[ -n "$process_group" ]]; then
    cleanup_process_group "$process_group"
    group_status=$?
  else
    echo "unable to capture an isolated host process group" >&2
  fi
  set -e
  runner_pid=""
  runner_pgid=""
  if (( finished == 0 )); then
    return 124
  fi
  if (( status != 0 )); then
    return "$status"
  fi
  return "$group_status"
}

run_host_phase() {
  local label="$1"
  local host_bin="$2"
  local product="$3"
  local host_version_value="$4"
  local profile="$run_root/$label"
  local result="$profile/result.json"
  local log="$profile/host.log"
  local status

  mkdir -p "$profile/home" "$profile/config" "$profile/data" "$profile/cache" \
    "$profile/runtime" "$profile/portable" "$profile/user-data" "$profile/extensions" "$profile/shared"
  chmod 700 "$profile/runtime"
  rm -f "$result"

  echo "== run $label host $host_version_value ($(host_commit "$product")) =="
  if run_host_process "$profile" "$result" "$log" "$host_bin" 1; then
    :
  else
    status=$?
    echo "$label host test failed with exit status $status" >&2
    tail -120 "$log" >&2 || true
    return 1
  fi
  if [[ ! -s "$result" ]]; then
    echo "$label host test exited without a result" >&2
    tail -120 "$log" >&2 || true
    return 1
  fi
  node -e '
    const result = require(process.argv[1]);
    if (result.installed !== true) throw new Error("VSIX installation did not complete");
  ' "$result"

  rm -f "$result"
  if run_host_process "$profile" "$result" "$log" "$host_bin" 0; then
    :
  else
    status=$?
    echo "$label host test failed with exit status $status" >&2
    tail -160 "$log" >&2 || true
    return 1
  fi
  if [[ ! -s "$result" ]]; then
    echo "$label host test exited without a result" >&2
    tail -160 "$log" >&2 || true
    return 1
  fi
  node - "$result" <<'EOF'
const fs = require("node:fs");
const assert = require("node:assert/strict");
const result = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
assert.equal(typeof result.host, "string", "host version is reported by the real host");
assert(result.host.length > 0, "host version is not empty");
assert.equal(result.extensionActive, true, "extension activated");
assert.equal(result.language, "recite", "Recite language is active");
assert.equal(result.hover, true, "hover is host-projected");
assert.equal(result.definition, true, "definition is host-projected");
assert.equal(result.references, "host-sorted", "references return a deterministic host projection");
assert.equal(result.crossFileDefinition, true, "cross-file definition is host-projected");
assert(result.lspDiagnostics.includes("RECITE_PARSE011"), "LSP diagnostics include stable parse code");
assert(result.lspDiagnostics.includes("RECITE_PARSE013"), "LSP diagnostics include all expected stable parse codes");
assert(result.diagnosticEvents > 0, "host observed diagnostic changes");
assert(result.completionCount > 0, "completion returned structured items");
assert.equal(result.validateSuccess, "success", "valid validation succeeded");
assert.equal(result.validateFailure, "content_diagnostics", "invalid validation reported content diagnostics");
assert.equal(result.compile, "content_diagnostics", "compile reported content diagnostics");
assert.equal(result.extract, "content_diagnostics", "extract reported content diagnostics");
assert.equal(result.run, "success", "run reported structured success");
assert.equal(result.trace, "success", "trace reported structured success");
assert.equal(result.runFailure, "failure", "run reported structured failure");
assert.equal(result.traceFailure, "failure", "trace reported structured failure");
assert.equal(result.watchStopped, true, "watch stopped");
assert.equal(result.watchExitCode, 0, "watch exited cleanly");
assert.equal(result.overlayRecovery, true, "malformed and incomplete overlays recovered");
assert.equal(result.utf16, "passed", "UTF-16/non-BMP ranges were projected");
assert.equal(result.codeAction, "stable-id-applied", "stable-ID code action applied");
assert.equal(result.nativeRename, "unsupported", "native rename limitation is explicit");
assert(["undefined", "empty-workspace-edit"].includes(result.nativeRenameShape), "native rename shape is recorded");
assert(["applied", "precondition-only"].includes(result.rename), "explicit rename result is recorded");
EOF
  node -e '
    const result = require(process.argv[1]);
    process.stdout.write(JSON.stringify(result) + "\n");
  ' "$result"
}

run_host() {
  local label="$1"
  local host_bin="$2"
  local product="$3"
  local archive_hash="$4"
  local pinned_version="$5"
  local actual_version
  local runtime_version
  actual_version="$(host_version "$product")"
  if [[ "$archive_hash" != caller-supplied-host-binary && "$actual_version" != "$pinned_version" ]]; then
    echo "$label version mismatch: expected $pinned_version, got $actual_version" >&2
    return 1
  fi
  run_host_phase "$label" "$host_bin" "$product" "$actual_version"
  runtime_version="$(node -e 'const result = require(process.argv[1]); process.stdout.write(result.host)' "$run_root/$label/result.json")"
  printf 'host=%s runtime_version=%s product_version=%s arch=x86_64 archive_sha256=%s vsix_sha256=%s install=passed activation=passed diagnostics=passed commands=passed watch_stop=passed shutdown=passed process_leak_check=passed\n' \
    "$label" "$runtime_version" "$actual_version" "$archive_hash" "$vsix_hash"
}

run_host vscode "$vscode_bin" "$vscode_product" "$vscode_archive_hash" "$VSCODE_VERSION"
run_host vscodium "$vscodium_bin" "$vscodium_product" "$vscodium_archive_hash" "$VSCODIUM_VERSION"
echo "installed VS Code/VSCodium host evidence passed"
