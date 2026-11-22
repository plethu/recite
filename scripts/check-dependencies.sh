#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
if ! command -v cargo-deny >/dev/null 2>&1; then
  echo "missing required tool: cargo-deny; run mise install" >&2
  exit 2
fi
cd "$repo_root"
cargo deny --locked --all-features check advisories bans licenses sources
# The shared config's advisory ignores name crates only the core workspace locks; the core run
# above still fails when one stops matching, so the other workspaces tolerate unmatched ignores.
cargo deny --locked --manifest-path editors/zed/Cargo.toml --all-features \
  check --allow advisory-not-detected advisories bans licenses sources
cargo deny --locked --manifest-path apps/writer/Cargo.toml --all-features \
  check --allow advisory-not-detected advisories bans licenses sources
