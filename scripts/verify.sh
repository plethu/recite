#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  verify.sh [repo-root]

Installs frozen JavaScript dependencies, then checks formatting, spelling,
unused dependencies, and dependency policy before the existing verification lanes:
  1. scripts/check-git-policy.sh
  2. tests/git-policy/check-integration.sh
  3. tests/maintainability/check.sh
  4. tests/ast-grep/check.sh
  5. scripts/check-maintainability.sh
  6. scripts/check-ast-grep.sh
  7. tests/lint-suppressions/check.sh
  8. scripts/check-lint-suppressions.sh
  9. tests/trusted-policy/check.sh
 10. tests/editor-parity/check.sh
 11. tests/check-pr-review-gates/check-rollup-fixtures.sh
 12. scripts/check-vscode.sh
 13. scripts/check-helix.sh
 14. tests/editor-hosts/helix/check.sh
 15. scripts/check-project-gates.sh (including editors and the Godot host)
 16. scripts/check-docs.sh
 17. scripts/benchmark-smoke.sh

Use `mise exec -- just check` (or `mise run verify`) from the repository root.
The check loads the scoped `maintainability` mise environment for ast-grep;
ordinary project commands do not install it.
`mise install` provisions the pinned toolchain. JavaScript dependencies are
installed from the frozen lockfile before any package checks.
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

for gate in check-git-policy.sh check-maintainability.sh check-ast-grep.sh check-lint-suppressions.sh check-vscode.sh check-helix.sh check-project-gates.sh check-docs.sh benchmark-smoke.sh; do
  if [[ ! -x "$repo_root/scripts/$gate" ]]; then
    echo "missing executable verification gate: $repo_root/scripts/$gate" >&2
    exit 2
  fi
done
if [[ ! -x "$repo_root/tests/check-pr-review-gates/check-rollup-fixtures.sh" ]]; then
  echo "missing executable verification fixture gate: $repo_root/tests/check-pr-review-gates/check-rollup-fixtures.sh" >&2
  exit 2
fi
if [[ ! -x "$repo_root/tests/ast-grep/check.sh" ]]; then
  echo "missing ast-grep verification fixture gate: $repo_root/tests/ast-grep/check.sh" >&2
  exit 2
fi
if [[ ! -x "$repo_root/tests/lint-suppressions/check.sh" ]]; then
  echo "missing lint suppression verification fixture gate: $repo_root/tests/lint-suppressions/check.sh" >&2
  exit 2
fi
if [[ ! -x "$repo_root/tests/trusted-policy/check.sh" ]]; then
  echo "missing trusted policy verification fixture gate: $repo_root/tests/trusted-policy/check.sh" >&2
  exit 2
fi
if [[ ! -x "$repo_root/tests/editor-parity/check.sh" ]]; then
  echo "missing editor parity verification fixture gate: $repo_root/tests/editor-parity/check.sh" >&2
  exit 2
fi
if [[ ! -x "$repo_root/tests/editor-hosts/helix/check.sh" ]]; then
  echo "missing Helix verification fixture gate: $repo_root/tests/editor-hosts/helix/check.sh" >&2
  exit 2
fi
if [[ ! -f "$repo_root/tests/git-policy/check-integration.sh" ]]; then
  echo "missing Git policy integration fixture gate: $repo_root/tests/git-policy/check-integration.sh" >&2
  exit 2
fi

echo "== workspace dependencies =="
"$repo_root/scripts/install-js-dependencies.sh" "$repo_root"

echo "== developer tool checks =="
(
  cd "$repo_root"
  just fmt-check
  just spelling
  just unused-deps
  just supply-chain
)

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
  tests/ast-grep/check.sh
  scripts/check-maintainability.sh
  scripts/check-ast-grep.sh
  tests/lint-suppressions/check.sh
  scripts/check-lint-suppressions.sh
)

echo
echo "== trusted pull-request policy fixtures =="
bash "$repo_root/tests/trusted-policy/check.sh" "$repo_root"

echo
echo "== editor parity contract fixtures =="
bash "$repo_root/tests/editor-parity/check.sh" "$repo_root"

echo
echo "== VS Code/VSCodium client =="
"$repo_root/scripts/check-vscode.sh" "$repo_root"

echo
echo "== Helix language configuration =="
"$repo_root/scripts/check-helix.sh" "$repo_root"

echo
echo "== Helix checker hostile fixtures =="
"$repo_root/tests/editor-hosts/helix/check.sh" "$repo_root"

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
