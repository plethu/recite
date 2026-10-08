#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/tools"
cp -R "$repo_root/tools/ast-grep" "$scratch/tools/"
cd "$scratch"

# A fresh file must be checked without Git metadata, across every Rust workspace.
for scope in crates/demo/src apps/writer/crates/demo/src editors/zed/src; do
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
echo "Complete Rust structural scan boundaries passed."
