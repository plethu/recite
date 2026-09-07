#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
if ! command -v pnpm >/dev/null 2>&1; then
  echo "missing required tool: pnpm; run mise install" >&2
  exit 2
fi
cd "$repo_root"
pnpm install --frozen-lockfile
