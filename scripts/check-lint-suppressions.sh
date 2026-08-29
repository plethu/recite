#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-lint-suppressions.sh [base-ref [head-ref]] [--full] [--policy-revision ref]

Inventories handwritten Rust #[allow]/#[expect] attributes and rejects only
new or expanded production suppressions that do not follow the local policy.
Use --full for a reporting-only inventory of all tracked Rust source.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  echo "unable to resolve Git repository root" >&2
  exit 2
}
parser="$repo_root/scripts/check-lint-suppressions.py"
if [[ ! -f "$parser" ]]; then
  echo "missing lint suppression parser: $parser" >&2
  exit 2
fi
cd "$repo_root"
exec python3 "$parser" "$@"
