#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  echo 'Usage: check-project-gates.sh [repo-root]'
  echo 'Runs core, native editor, engine host and Writer checks; just check is the complete gate.'
  exit 0
fi
repo_root=${1:-$(git rev-parse --show-toplevel)}
cd "$repo_root"
just core-check
just editor-native-check
just host-check
just writer check
