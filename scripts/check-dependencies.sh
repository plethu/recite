#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
if ! command -v cargo-deny >/dev/null 2>&1; then
  echo "missing required tool: cargo-deny; run mise install" >&2
  exit 2
fi
cd "$repo_root"
cargo deny --locked --all-features check advisories bans licenses sources
cargo deny --locked --manifest-path editors/zed/Cargo.toml --all-features \
  --config "$repo_root/deny.toml" check advisories bans licenses sources
