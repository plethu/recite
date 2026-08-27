#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  verify.sh [repo-root]

Runs the complete local verification suite:
  1. scripts/check-git-policy.sh
  2. tests/git-policy/check-integration.sh
  3. tests/maintainability/check.sh
  4. scripts/check-maintainability.sh
  5. scripts/check-ast-grep.sh
  6. tests/check-pr-review-gates/check-rollup-fixtures.sh
  7. scripts/check-project-gates.sh
  8. scripts/check-docs.sh
  9. scripts/benchmark-smoke.sh

Use `mise run verify` from the repository root when mise is available.
`mise install` provisions the pinned Rust, Node, pnpm, .NET, cbindgen, and
ast-grep tools.
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

for gate in check-git-policy.sh check-maintainability.sh check-ast-grep.sh check-project-gates.sh check-docs.sh benchmark-smoke.sh; do
  if [[ ! -x "$repo_root/scripts/$gate" ]]; then
    echo "missing executable verification gate: $repo_root/scripts/$gate" >&2
    exit 2
  fi
done
if [[ ! -x "$repo_root/tests/check-pr-review-gates/check-rollup-fixtures.sh" ]]; then
  echo "missing executable verification fixture gate: $repo_root/tests/check-pr-review-gates/check-rollup-fixtures.sh" >&2
  exit 2
fi
if [[ ! -f "$repo_root/tests/git-policy/check-integration.sh" ]]; then
  echo "missing Git policy integration fixture gate: $repo_root/tests/git-policy/check-integration.sh" >&2
  exit 2
fi

echo "== Git workflow policy =="
"$repo_root/scripts/check-git-policy.sh" "$repo_root"

echo
echo "== Git workflow integration policy fixtures =="
bash "$repo_root/tests/git-policy/check-integration.sh" "$repo_root"

echo
echo "== maintainability fixtures and changed-surface check =="
(
  cd "$repo_root"
  tests/maintainability/check.sh
  scripts/check-maintainability.sh
  scripts/check-ast-grep.sh
)

echo
echo "== pull-request check rollup fixtures =="
"$repo_root/tests/check-pr-review-gates/check-rollup-fixtures.sh" "$repo_root"

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
