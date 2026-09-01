#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-neovim.sh [--static] [repo-root]

Checks the plugin-manager-neutral Neovim integration. Static package and
query checks always run. The default lane is authoritative and requires
Neovim, Cargo, and Tree-sitter. Use NVIM=/path/to/nvim to select an explicit
Neovim binary. The static mode is for parity fixtures only.
EOF
}

static_only=0
if [[ "${1:-}" == "--static" ]]; then
  static_only=1
  shift
fi
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

plugin_root="$repo_root/editor/recite-neovim"
grammar_query="$repo_root/editor/recite-tree-sitter/queries/highlights.scm"
neovim_query="$plugin_root/queries/recite/highlights.scm"
for required_file in \
  "$plugin_root/plugin/recite.lua" \
  "$plugin_root/lua/recite.lua" \
  "$plugin_root/ftdetect/recite.lua" \
  "$plugin_root/health/recite.lua" \
  "$neovim_query" \
  "$repo_root/tests/neovim/check.lua"; do
  if [[ ! -f "$required_file" ]]; then
    echo "missing Neovim integration file: $required_file" >&2
    exit 2
  fi
done

if ! cmp -s "$grammar_query" "$neovim_query"; then
  echo "Neovim highlight query diverges from the host-neutral Tree-sitter query" >&2
  diff -u "$grammar_query" "$neovim_query" | sed -n '1,120p' >&2 || true
  exit 1
fi
echo "Neovim package and query checks passed"

if (( static_only )); then
  exit 0
fi

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
  echo "Neovim $nvim_version is below the supported minimum 0.10.4" >&2
  exit 1
fi
echo "Neovim $nvim_version (minimum supported: 0.10.4; current pinned smoke: 0.12.5)"

scratch="$(mktemp -d "${TMPDIR:-/tmp}/recite-neovim.XXXXXX")"
cleanup() {
  rm -rf "$scratch"
}
trap cleanup EXIT
project="$scratch/project"
mkdir -p "$project"
cp "$repo_root/fixtures/recite/valid/core_language_spike.recite" "$project/core_language_spike.recite"
cp "$repo_root/fixtures/recite/invalid/parser_marker_leading_prose.recite" "$project/invalid.recite"
printf '%s\n' 'format_version = 1' > "$project/recite.project.toml"

echo "== Neovim headless filetype/LSP checks =="
"$cargo_bin" build --locked -q -p recite-lsp

parser_root="$scratch/parser-runtime"
mkdir -p "$parser_root/parser"
if XDG_CACHE_HOME="$scratch/tree-sitter-cache" "$tree_sitter_bin" build \
  "$repo_root/editor/recite-tree-sitter" \
  --output "$parser_root/parser/recite.so"; then
  parser_available=1
else
  echo "Tree-sitter parser build failed" >&2
  exit 1
fi

RECITE_PLUGIN="$plugin_root" \
RECITE_PARSER_ROOT="$parser_root" \
RECITE_LSP="$repo_root/target/debug/recite-lsp" \
RECITE_TEST_PROJECT="$project" \
RECITE_PARSER_AVAILABLE="$parser_available" \
  env -u RECITE_CONFIG XDG_CONFIG_HOME="$scratch/config" XDG_STATE_HOME="$scratch/state" \
    "$nvim_bin" --headless -u "$repo_root/tests/neovim/preload.lua" -i NONE -n \
    -l "$repo_root/tests/neovim/check.lua"

echo "Neovim headless checks passed"
