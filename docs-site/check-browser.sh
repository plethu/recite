#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
cd "$repo_root"
just web setup
mkdir -p docs-site/.browser-artifacts

LOCAL_UID="$(id -u)"
LOCAL_GID="$(id -g)"
export LOCAL_UID LOCAL_GID
docker compose -f docs-site/compose.yaml run --build --quiet-build --rm browsers \
  pnpm exec playwright test "$@"
