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
# Palette ownership covers fresh Writer files and ignores its exact owners/tests.
writer_source=apps/writer/crates/freya/src
mkdir -p "$writer_source/design/palette" "$writer_source/nested"
for source in edge.rs nested/palette.rs design/palette.rs design/material.rs design/palette/tests.rs; do
  cat >"$writer_source/$source" <<'RUST'
fn colour(red: u8) { Color::from_rgb(red, 0, 0); }
RUST
  if [[ "$source" == edge.rs || "$source" == nested/palette.rs ]]; then
    if ast-grep scan --config tools/ast-grep/sgconfig.yml --filter '^rust-writer-palette$'; then
      echo "palette scan missed fresh Writer source" >&2
      exit 1
    fi
    rm "$writer_source/$source"
  else
    ast-grep scan --config tools/ast-grep/sgconfig.yml --filter '^rust-writer-palette$'
  fi
done

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
# Numeric identity policy covers fresh production source in every owned workspace.
# No Git metadata or pre-existing file list may be required to enforce it.
for scope in crates/demo/src crates/demo/src/nested \
  apps/writer/crates/freya/src apps/writer/crates/xilem/src/nested \
  apps/writer/crates/session/src apps/writer/crates/authoring/src/nested \
  editors/zed/src tools/demo/src; do
  mkdir -p "$scope"
  cat >"$scope/identity.rs" <<'RUST'
struct Pending { generation: Option<u64> }
type RecoverySequence = u64;
fn accept(expected_revision: i64) {}
RUST
  status=0
  ast-grep scan --config tools/ast-grep/sgconfig.yml --filter '^rust-numeric-identities$' || status=$?
  if [[ "$status" != 1 ]]; then
    echo "numeric identity scan must reject fresh source in $scope (exit $status)" >&2
    exit 1
  fi
  cat >"$scope/identity.rs" <<'RUST'
struct CommandGeneration(u64);
struct Pending { generation: Option<CommandGeneration>, count: u64, index: usize }
fn accept(expected_revision: CommandGeneration) {}
RUST
  ast-grep scan --config tools/ast-grep/sgconfig.yml --filter '^rust-numeric-identities$'
  # Fixture data is intentionally outside the production domain-type rule.
  cat >"$scope/tests.rs" <<'RUST'
struct Fixture { generation: u64 }
RUST
  mkdir -p "$scope/tests"
  cp "$scope/tests.rs" "$scope/tests/support.rs"
  ast-grep scan --config tools/ast-grep/sgconfig.yml --filter '^rust-numeric-identities$'
done
echo "Complete Rust structural scan boundaries passed."
