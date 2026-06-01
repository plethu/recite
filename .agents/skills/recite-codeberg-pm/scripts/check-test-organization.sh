#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if ! repo_root="$(git -C "$script_dir" rev-parse --show-toplevel 2>/dev/null)"; then
  echo "unable to resolve repository root from $script_dir" >&2
  exit 2
fi

exec "$repo_root/scripts/check-test-organization.sh" "$@"
