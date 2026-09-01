#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-editor-parity.sh [repo-root]

Validates the editor parity contract, canonical fixture references, and honest
status/artifact claims shared by the documentation and JSON matrix.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi
if [[ $# -gt 1 ]]; then
  usage >&2
  exit 2
fi

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  repo_root="$(git -C "$input_root" rev-parse --show-toplevel)"
else
  repo_root="$(git rev-parse --show-toplevel)"
fi

fixture="$repo_root/fixtures/editor-parity/contract.json"
document="$repo_root/docs/editor-parity-contract.md"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "$script_dir/editor_parity/check.py" "$repo_root" "$fixture" "$document"
