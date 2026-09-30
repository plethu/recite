#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
cd "$repo_root"
mkdir -p docs-site/.browser-artifacts

export LOCAL_UID="$(id -u)"
export LOCAL_GID="$(id -g)"
docker compose -f docs-site/compose.yaml run --build --quiet-build --rm browsers \
  pnpm exec playwright test "$@"
