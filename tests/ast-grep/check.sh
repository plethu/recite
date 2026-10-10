#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/tools"
cp -R "$repo_root/tools/ast-grep" "$scratch/tools/"
cd "$scratch"

# A fresh file must be checked without Git metadata, across every Rust workspace.
for scope in crates/demo/src crates/demo/benches crates/demo/examples \
  apps/writer/crates/demo/src apps/writer/crates/demo/benches \
  apps/writer/crates/demo/examples editors/zed/src tools/demo/src; do
  mkdir -p "$scope"
  cat >"$scope/lib.rs" <<'RUST'
fn classifier(value: usize) -> usize {
    if value == 0 { 0 }
    else if value == 1 { 1 }
    else if value == 2 { 2 }
    else if value == 3 { 3 }
    else { 4 }
}
RUST
  if ast-grep scan --config tools/ast-grep/sgconfig.yml; then
    echo "structural scan missed fresh source in $scope" >&2
    exit 1
  fi
  rm "$scope/lib.rs"
done

# Test bodies remain governed by test organization, not production complexity rules.
mkdir -p crates/demo/src
cat >crates/demo/src/tests.rs <<'RUST'
fn fixture(value: usize) -> usize {
    if value == 0 { 0 }
    else if value == 1 { 1 }
    else if value == 2 { 2 }
    else if value == 3 { 3 }
    else { 4 }
}
RUST
ast-grep scan --config tools/ast-grep/sgconfig.yml

# Role/state piles in a fresh Writer control fail without relying on Git files.
mkdir -p apps/writer/crates/demo/src/design
cat >apps/writer/crates/demo/src/design/control.rs <<'RUST'
struct Control {
    selected: Option<bool>,
    checked: Option<bool>,
}
RUST
if ast-grep scan --config tools/ast-grep/sgconfig.yml; then
  echo "structural scan missed optional state fields in a fresh control" >&2
  exit 1
fi
cp apps/writer/crates/demo/src/design/control.rs apps/writer/crates/demo/src/design/tests.rs
rm apps/writer/crates/demo/src/design/control.rs

# Independent optional configuration overrides and private fixtures are not controls.
mkdir -p crates/demo/src/config
cat >crates/demo/src/config/preferences.rs <<'RUST'
struct Preferences {
    reduced_motion: Option<bool>,
    shortcut_hints: Option<bool>,
}
RUST
ast-grep scan --config tools/ast-grep/sgconfig.yml
echo "Complete Rust structural scan boundaries passed."
