#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-docs.sh [repo-root]

Installs the pinned workspace packages from pnpm-lock.yaml and verifies the
documentation site with its type check and production build.
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

"$repo_root/scripts/install-js-dependencies.sh" "$repo_root"

echo
echo "== generated schema fixtures =="
(
  cd "$repo_root"
  node scripts/check-schema-manifest.mjs "$repo_root"
)

echo
echo "== documentation verification =="
(
  cd "$repo_root"
  pnpm docs:verify
)

echo
echo "Recite documentation checks passed."
