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

for required_file in package.json pnpm-lock.yaml; do
  if [[ ! -f "$repo_root/$required_file" ]]; then
    echo "missing required documentation package file: $repo_root/$required_file" >&2
    exit 2
  fi
done

if ! command -v pnpm >/dev/null 2>&1; then
  cat >&2 <<'EOF'
missing required tool: pnpm

Install the repository toolchain with:
  mise install
EOF
  exit 2
fi

echo "== install documentation packages (frozen lockfile) =="
(
  cd "$repo_root"
  pnpm install --frozen-lockfile
)

echo
echo "== documentation verification =="
(
  cd "$repo_root"
  pnpm docs:verify
)

echo
echo "Recite documentation checks passed."
