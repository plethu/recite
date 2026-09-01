#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-vscode.sh [repo-root]

Checks the shared VS Code/VSCodium client with its package contract and the
real language server. The live test is required here rather than skipped when
the language-server binary is unavailable.
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

if [[ -n "${RECITE_LSP_BIN:-}" ]]; then
  lsp_bin="$RECITE_LSP_BIN"
else
  echo "== build recite-lsp for VS Code live checks =="
  (
    cd "$repo_root"
    cargo build --locked -q -p recite-lsp
  )
  lsp_bin="$repo_root/target/debug/recite-lsp"
fi
if [[ ! -x "$lsp_bin" ]]; then
  echo "VS Code live checks require an executable recite-lsp: $lsp_bin" >&2
  echo "Build it with: cargo build --locked -q -p recite-lsp" >&2
  exit 2
fi

hash_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

projection_paths=(
  editors/vscode/src/messages.generated.js
  editors/vscode/package.nls.json
)
declare -A projection_hashes=()
for relative in "${projection_paths[@]}"; do
  file="$repo_root/$relative"
  if [[ ! -f "$file" ]]; then
    echo "VS Code check requires tracked projection: $relative" >&2
    exit 2
  fi
  projection_hashes["$relative"]="$(hash_file "$file")"
done

check_projection_unchanged() {
  local status=$?
  local failures=0
  for relative in "${projection_paths[@]}"; do
    file="$repo_root/$relative"
    if [[ ! -f "$file" ]] || [[ "$(hash_file "$file")" != "${projection_hashes["$relative"]}" ]]; then
      echo "VS Code check mutated a tracked message projection: $relative" >&2
      failures=1
    fi
  done
  if (( failures > 0 )); then
    exit 1
  fi
  exit "$status"
}
trap check_projection_unchanged EXIT

if ! command -v pnpm >/dev/null 2>&1; then
  echo "VS Code checks require pnpm from the repository toolchain" >&2
  exit 2
fi

echo "== VS Code/VSCodium package and live checks =="
(
  cd "$repo_root"
  RECITE_LSP_BIN="$lsp_bin" pnpm editor:check
  pnpm editor:package
)
echo "VS Code/VSCodium checks passed"
