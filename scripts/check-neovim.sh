#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-neovim.sh [repo-root]

Checks the plugin-manager-neutral Neovim integration. Static package and
query checks always run. Headless filetype, Tree-sitter, LSP diagnostic, and
shutdown checks run when Neovim and the required local tools are available;
otherwise the unavailable platform/tool check is reported as skipped.
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

if ! command -v nvim >/dev/null 2>&1; then
  echo "Neovim headless checks skipped: nvim is not installed"
  exit 0
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "Neovim headless checks skipped: cargo is not installed"
  exit 0
fi

scratch="$(mktemp -d "${TMPDIR:-/tmp}/recite-neovim.XXXXXX")"
cleanup() {
  rm -rf "$scratch"
}
trap cleanup EXIT
project="$scratch/project"
mkdir -p "$project"
cp "$repo_root/fixtures/recite/valid/language_pressure.recite" "$project/language_pressure.recite"
cp "$repo_root/fixtures/recite/invalid/parser_marker_leading_prose.recite" "$project/invalid.recite"
printf '%s\n' 'format_version = 1' > "$project/recite.project.toml"

echo "== Neovim headless filetype/LSP checks =="
cargo build --locked -q -p recite-lsp

parser_available=0
parser_root="$scratch/parser-runtime"
if command -v tree-sitter >/dev/null 2>&1; then
  mkdir -p "$parser_root/parser"
  if XDG_CACHE_HOME="$scratch/tree-sitter-cache" tree-sitter build \
    "$repo_root/editor/recite-tree-sitter" \
    --output "$parser_root/parser/recite.so"; then
    parser_available=1
  else
    echo "Tree-sitter parser build skipped after a failed platform build"
  fi
else
  echo "Tree-sitter parser smoke skipped: tree-sitter is not installed"
fi

RECITE_PLUGIN="$plugin_root" \
RECITE_PARSER_ROOT="$parser_root" \
RECITE_LSP="$repo_root/target/debug/recite-lsp" \
RECITE_TEST_PROJECT="$project" \
RECITE_PARSER_AVAILABLE="$parser_available" \
  env -u RECITE_CONFIG XDG_STATE_HOME="$scratch/state" nvim --headless -u NONE -i NONE -n \
    -c 'lua vim.opt.rtp:prepend(vim.env.RECITE_PLUGIN)' \
    -c 'lua vim.opt.rtp:prepend(vim.env.RECITE_PARSER_ROOT)' \
    -c 'lua require("recite").setup({ lsp = { cmd = { vim.env.RECITE_LSP } } })' \
    -l "$repo_root/tests/neovim/check.lua"

echo "Neovim headless checks passed"
