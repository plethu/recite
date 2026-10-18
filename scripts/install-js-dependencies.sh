#!/usr/bin/env bash
set -euo pipefail

if (($# > 1)); then
  echo "usage: install-js-dependencies.sh [repo-root]" >&2
  exit 2
fi
repo_root="$(git -C "${1:-.}" rev-parse --show-toplevel)"
if ! command -v bun >/dev/null 2>&1; then
  echo "missing required tool: bun; run mise install" >&2
  exit 2
fi
cd "$repo_root"
if [[ ! -f bun.lock ]]; then
  echo "missing committed lockfile: bun.lock" >&2
  exit 2
fi
bun install --frozen-lockfile
