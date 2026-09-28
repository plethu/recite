#!/usr/bin/env bash
set -euo pipefail
repo_root="${1:-$(git rev-parse --show-toplevel)}"
RECITE_UNITY_PERF=1 "$repo_root/scripts/check-unity-adapter.sh" "$repo_root"
