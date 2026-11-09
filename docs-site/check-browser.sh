#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
cd "$repo_root"
just web setup
just web wasm
mkdir -p docs-site/.browser-artifacts

LOCAL_UID="$(id -u)"
LOCAL_GID="$(id -g)"
MISE_GITHUB_TOKEN="${MISE_GITHUB_TOKEN:-${GITHUB_TOKEN:-}}"
export LOCAL_UID LOCAL_GID MISE_GITHUB_TOKEN
docker compose -f docs-site/compose.yaml run --build --quiet-build --rm browsers \
  bun run playwright test "$@"
