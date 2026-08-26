#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  verify.sh [repo-root]

Runs the complete local verification suite:
  1. scripts/check-git-policy.sh
  2. scripts/check-project-gates.sh
  3. scripts/check-docs.sh
  4. scripts/benchmark-smoke.sh

Use `mise run verify` from the repository root when mise is available.
`mise install` provisions the pinned Rust, Node, pnpm, .NET, and cbindgen tools.
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
  if ! repo_root="$(git -C "$input_root" rev-parse --show-toplevel 2>/dev/null)"; then
    echo "repo root is not a git checkout: $input_root" >&2
    exit 2
  fi
else
  if ! repo_root="$(git rev-parse --show-toplevel 2>/dev/null)"; then
    echo "unable to resolve git repo root from current directory" >&2
    exit 2
  fi
fi

for gate in check-git-policy.sh check-project-gates.sh check-docs.sh benchmark-smoke.sh; do
  if [[ ! -x "$repo_root/scripts/$gate" ]]; then
    echo "missing executable verification gate: $repo_root/scripts/$gate" >&2
    exit 2
  fi
done

echo "== Git workflow policy =="
"$repo_root/scripts/check-git-policy.sh" "$repo_root"

echo "== Rust and adapter gates =="
"$repo_root/scripts/check-project-gates.sh" "$repo_root"

echo
echo "== documentation gates =="
"$repo_root/scripts/check-docs.sh" "$repo_root"

echo
echo "== benchmark smoke =="
"$repo_root/scripts/benchmark-smoke.sh" "$repo_root"

echo
echo "Recite verification passed."
