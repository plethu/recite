#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-neovim.sh [--static] [--host-evidence] [repo-root]

Checks the plugin-manager-neutral Neovim integration. Static package and
query checks always run. The default lane is authoritative and requires
Neovim, Cargo, and Tree-sitter. Use NVIM=/path/to/nvim to select an explicit
Neovim binary. The static mode is for parity fixtures only. The host-evidence
mode additionally runs the installed-host keyboard workflow and checks that
the Neovim process group is empty after clean shutdown.
EOF
}

static_only=0
host_evidence=0
input_root=""
while (( $# > 0 )); do
  case "$1" in
    --static) static_only=1 ;;
    --host-evidence) host_evidence=1 ;;
    -h|--help|help)
      usage
      exit 0
      ;;
    -*)
      usage >&2
      exit 2
      ;;
    *)
      if [[ -n "$input_root" ]]; then
        usage >&2
        exit 2
      fi
      input_root="$1"
      ;;
  esac
  shift
done
if [[ -n "$input_root" ]]; then
  repo_root="$(git -C "$input_root" rev-parse --show-toplevel)"
else
  repo_root="$(git rev-parse --show-toplevel)"
fi

plugin_root="$repo_root/editors/recite-neovim"
grammar_query="$repo_root/editors/recite-tree-sitter/queries/highlights.scm"
neovim_query="$plugin_root/queries/recite/highlights.scm"
for required_file in \
  "$plugin_root/plugin/recite.lua" \
  "$plugin_root/lua/recite.lua" \
  "$plugin_root/lua/recite/lifecycle.lua" \
  "$plugin_root/lua/recite/material.lua" \
  "$plugin_root/lua/recite/health.lua" \
  "$plugin_root/lua/recite/command_json.lua" \
  "$plugin_root/lua/recite/command_protocol.lua" \
  "$plugin_root/lua/recite/diagnostic_protocol.lua" \
  "$plugin_root/lua/recite/finite_protocol.lua" \
  "$plugin_root/lua/recite/command_process.lua" \
  "$plugin_root/lua/recite/timer.lua" \
  "$plugin_root/lua/recite/command_diagnostics.lua" \
  "$plugin_root/lua/recite/command_inputs.lua" \
  "$plugin_root/lua/recite/finite_controller.lua" \
  "$plugin_root/lua/recite/trace_protocol.lua" \
  "$plugin_root/lua/recite/watch_protocol.lua" \
  "$plugin_root/lua/recite/watch.lua" \
  "$plugin_root/lua/recite/commands.lua" \
  "$plugin_root/ftdetect/recite.lua" \
  "$plugin_root/scripts/message-projections.mjs" \
  "$plugin_root/scripts/diagnostic-projections.mjs" \
  "$plugin_root/lua/recite_messages.lua" \
  "$plugin_root/lua/recite_diagnostics.lua" \
  "$neovim_query" \
  "$repo_root/tests/neovim/check.lua" \
  "$repo_root/tests/neovim/recovery.lua" \
  "$repo_root/tests/neovim/material.lua" \
  "$repo_root/tests/neovim/commands_protocol.lua" \
  "$repo_root/tests/neovim/commands_lifecycle.lua" \
  "$repo_root/tests/neovim/commands.lua"; do
  if [[ ! -f "$required_file" ]]; then
    echo "missing Neovim integration file: $required_file" >&2
    exit 2
  fi
done
if (( host_evidence )) && [[ ! -f "$repo_root/tests/editor-hosts/neovim/keyboard-workflow.lua" ]]; then
  echo "missing Neovim installed-host evidence file: $repo_root/tests/editor-hosts/neovim/keyboard-workflow.lua" >&2
  exit 2
fi

if [[ -e "$plugin_root/health/recite.lua" ]]; then
  echo "legacy Neovim health path is not part of the runtimepath package: $plugin_root/health/recite.lua" >&2
  exit 1
fi

if ! cmp -s "$grammar_query" "$neovim_query"; then
  echo "Neovim highlight query diverges from the host-neutral Tree-sitter query" >&2
  diff -u "$grammar_query" "$neovim_query" | sed -n '1,120p' >&2 || true
  exit 1
fi
echo "Neovim package and query checks passed"

resolve_tool() {
  local requested="$1"
  if [[ "$requested" == */* ]]; then
    if [[ -x "$requested" ]]; then
      printf '%s\n' "$requested"
    fi
  else
    command -v "$requested" || true
  fi
  return 0
}

node_bin="$(resolve_tool "${NODE:-node}")"
if [[ -z "$node_bin" ]]; then
  echo "Neovim UI projection checks require node; install the pinned tool or set NODE=/path/to/node" >&2
  exit 2
fi

"$node_bin" "$plugin_root/scripts/message-projections.mjs" --check
"$node_bin" --test "$plugin_root/test/message-projections.test.mjs"
"$node_bin" "$plugin_root/scripts/diagnostic-projections.mjs"
echo "Neovim UI message projection checks passed"

if (( static_only )); then
  exit 0
fi

nvim_bin="$(resolve_tool "${NVIM:-nvim}")"
cargo_bin="$(resolve_tool "${CARGO:-cargo}")"
tree_sitter_bin="$(resolve_tool "${TREE_SITTER:-tree-sitter}")"
if [[ -z "$nvim_bin" ]]; then
  echo "Neovim headless checks require nvim; install the pinned tool or set NVIM=/path/to/nvim" >&2
  exit 2
fi
if [[ -z "$cargo_bin" ]]; then
  echo "Neovim headless checks require cargo; install the pinned Rust toolchain or set CARGO=/path/to/cargo" >&2
  exit 2
fi
if [[ -z "$tree_sitter_bin" ]]; then
  echo "Neovim headless checks require tree-sitter; install the pinned tool or set TREE_SITTER=/path/to/tree-sitter" >&2
  exit 2
fi
nvim_version="$($nvim_bin --headless --version | sed -n '1s/^NVIM v//p')"
if [[ -z "$nvim_version" ]]; then
  echo "unable to determine Neovim version from $nvim_bin" >&2
  exit 2
fi
if [[ "$(printf '%s\n' '0.10.4' "$nvim_version" | sort -V | head -n1)" != "0.10.4" ]]; then
  echo "Neovim $nvim_version is below the 0.10.4 compatibility target" >&2
  exit 1
fi
echo "Neovim $nvim_version (compatibility target: 0.10.4; current pinned smoke: 0.12.5)"

scratch="$(mktemp -d "${TMPDIR:-/tmp}/recite-neovim.XXXXXX")"
cleanup() {
  rm -rf "$scratch"
}
trap cleanup EXIT
project="$scratch/project"
second_project="$scratch/second-project"
invalid_project="$scratch/invalid-project"
missing_project="$scratch/missing-project"
unicode_project="$scratch/unicode-project"
config_home="$scratch/config"
config_dirs="$scratch/config-dirs"
data_home="$scratch/data"
data_dirs="$scratch/data-dirs"
state_home="$scratch/state"
cache_home="$scratch/cache"
mkdir -p "$project" "$second_project" "$invalid_project" "$missing_project" "$unicode_project" \
  "$config_home" "$config_dirs" "$data_home" "$data_dirs" "$state_home" "$cache_home"
cp "$repo_root/fixtures/recite/valid/core_language_spike.recite" "$project/core_language_spike.recite"
cp "$repo_root/fixtures/recite/invalid/parser_marker_leading_prose.recite" "$invalid_project/invalid.recite"
sed 's/> intro_001@637b1854a7f3ed42f045 speaker=hazel mood=calm mood=alert/>/' \
  "$repo_root/fixtures/recite/valid/core_language_spike.recite" > "$missing_project/missing.recite"
printf '%s\n' 'format_version = 1' > "$project/recite.project.toml"
printf '%s\r\n' \
  ':: marker_probe default' \
  '> sign@88990011223344556677' \
  '  -> 😀East, if you can read it.' \
  '  :if this is a sentence, not a branch.' \
  '  # ash marks the lintel.' \
  '  ? ask@99aabbccddeeff001122' \
    '    Ask what the sign means.' \
    '    -> END' > "$unicode_project/unicode.recite"
cp "$repo_root/fixtures/recite/valid/core_language_spike.recite" "$second_project/core_language_spike.recite"
printf '%s\n' 'format_version = 1' > "$second_project/recite.project.toml"
for isolated_project in "$invalid_project" "$missing_project" "$unicode_project"; do
  printf '%s\n' 'format_version = 1' > "$isolated_project/recite.project.toml"
done

delayed_lsp="$scratch/delayed-recite-lsp"
# The wrapper must expand this variable when it runs, not while it is generated.
# shellcheck disable=SC2016
printf '%s\n' \
  '#!/usr/bin/env bash' \
  'set -euo pipefail' \
  'sleep 2.1' \
  'exec "${RECITE_LSP_TARGET:?}" "$@"' > "$delayed_lsp"
chmod +x "$delayed_lsp"

echo "== Neovim headless filetype/LSP/structured-command checks =="
"$cargo_bin" build --locked -q -p recite-lsp -p recite-cli

parser_root="$scratch/parser-runtime"
mkdir -p "$parser_root/parser"
if XDG_CACHE_HOME="$scratch/tree-sitter-cache" "$tree_sitter_bin" build \
  "$repo_root/editors/recite-tree-sitter" \
  --output "$parser_root/parser/recite.so"; then
  parser_available=1
else
  echo "Tree-sitter parser build failed" >&2
  exit 1
fi

run_nvim() {
  RECITE_PLUGIN="$plugin_root" \
  RECITE_PARSER_ROOT="$parser_root" \
  RECITE_LSP="$repo_root/target/debug/recite-lsp" \
  RECITE_TEST_PROJECT="$project" \
  RECITE_SECOND_PROJECT="$second_project" \
  RECITE_INVALID_PROJECT="$invalid_project" \
  RECITE_MISSING_PROJECT="$missing_project" \
  RECITE_UNICODE_PROJECT="$unicode_project" \
  RECITE_DELAYED_LSP="$delayed_lsp" \
  RECITE_LSP_TARGET="$repo_root/target/debug/recite-lsp" \
  RECITE_CLI="$repo_root/target/debug/recite" \
  RECITE_REPO_ROOT="$repo_root" \
  RECITE_PARSER_AVAILABLE="$parser_available" \
    env -u RECITE_CONFIG -u NVIM_APPNAME -u VIMINIT -u EXINIT -u VIMRUNTIME \
      XDG_CONFIG_HOME="$config_home" XDG_CONFIG_DIRS="$config_dirs" \
      XDG_DATA_HOME="$data_home" XDG_DATA_DIRS="$data_dirs" \
      XDG_STATE_HOME="$state_home" XDG_CACHE_HOME="$cache_home" "$@"
}

run_headless() {
  local output
  local status=0
  if (( host_evidence )); then
    local output_file="$scratch/nvim-output.$$.${RANDOM}"
    if ! command -v setsid >/dev/null 2>&1 || ! command -v ps >/dev/null 2>&1; then
      echo "Neovim host evidence requires setsid and ps for process-group cleanup checks" >&2
      return 1
    fi
    run_nvim setsid "$nvim_bin" --headless -u "$repo_root/tests/neovim/preload.lua" -i NONE -n \
      -l "$1" >"$output_file" 2>&1 &
    local nvim_pid=$!
    if wait "$nvim_pid"; then
      status=0
    else
      status=$?
    fi
    output="$(<"$output_file")"
    rm -f "$output_file"
    if (( status == 0 )); then
      local leaked_processes
      leaked_processes="$(ps -eo pid=,pgid=,args= | awk -v group="$nvim_pid" '$2 == group && $1 != group { print }')"
      if [[ -n "$leaked_processes" ]]; then
        echo "Neovim host lane leaked processes from process group $nvim_pid:" >&2
        printf '%s\n' "$leaked_processes" >&2
        status=1
      fi
    fi
  else
    if ! output=$(run_nvim "$nvim_bin" --headless -u "$repo_root/tests/neovim/preload.lua" -i NONE -n \
      -l "$1" 2>&1); then
      status=1
    fi
  fi
  if (( status != 0 )); then
    printf '%s\n' "$output"
    return 1
  fi
  printf '%s\n' "$output"
  if printf '%s\n' "$output" | rg -n 'E[0-9]{4}:|Error detected while processing|stack traceback|^lua:' >/dev/null; then
    echo "Neovim headless lane emitted a Lua/API error" >&2
    return 1
  fi
}

run_headless "$repo_root/tests/neovim/check.lua"
run_headless "$repo_root/tests/neovim/recovery.lua"
run_headless "$repo_root/tests/neovim/material.lua"
run_headless "$repo_root/tests/neovim/commands.lua"
if (( host_evidence )); then
  run_headless "$repo_root/tests/editor-hosts/neovim/keyboard-workflow.lua"
fi

echo "Neovim headless checks passed"
