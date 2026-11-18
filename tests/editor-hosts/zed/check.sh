#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
exec "$repo_root/scripts/editors/check-zed-host.sh" "$repo_root"
