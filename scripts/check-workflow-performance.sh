#!/usr/bin/env bash
# The current Rust harness drives both revisions; Gungraun owns measurement and budgets.
set -euo pipefail
if [[ $# -lt 1 || $# -gt 3 ]]; then
  echo 'Usage: check-workflow-performance.sh BASE_COMMIT [pr|full|writer|authoring] [OUTPUT_DIRECTORY]' >&2
  exit 2
fi
scope="${2:-pr}"
case "$scope" in
  pr)
    fixtures=small,realistic:v1-pack
    parameters=(small realistic)
    benches=(workflows writer bound_text authoring)
    ;;
  full)
    fixtures=small,medium,realistic:v1-pack
    parameters=(small realistic medium)
    benches=(workflows writer bound_text authoring)
    ;;
  writer)
    fixtures=small,realistic:v1-pack
    parameters=()
    benches=(writer)
    ;;
  authoring)
    fixtures=small
    parameters=()
    benches=(authoring)
    ;;
  *)
    echo 'Expected pr, full, writer, or authoring scope' >&2
    exit 2
    ;;
esac
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
base="$(git rev-parse --verify "${1}^{commit}")"
output="${3:-$repo_root/target/workflow-performance-$scope}"
[[ ! -e "$output" ]] || {
  echo 'Choose a fresh evidence directory' >&2
  exit 2
}
mkdir -p "$output/evidence"
output="$(cd "$output" && pwd)"
evidence="$output/evidence"
if [[ "$scope" == pr || "$scope" == full ]]; then
  cargo metadata --locked --no-deps --format-version 1 >"$evidence/candidate-workspace.json"
  jq -S '[.packages[] | select(.name == "recite-perf") | .targets[] | select(.kind | index("bench")) | .name] | sort' "$evidence/candidate-workspace.json" >"$evidence/declared-benches.json"
  printf '%s\n' "${benches[@]}" | jq -Rs 'split("\n")[:-1] | sort' >"$evidence/selected-benches.json"
  diff -u "$evidence/declared-benches.json" "$evidence/selected-benches.json"
fi
build_root="${RECITE_PERF_BUILD_DIR:-$output}"
mkdir -p "$build_root"
build_root="$(cd "$build_root" && pwd)"
control="$output/control-source"
mkdir -p "$control"
trap 'rm -rf "$control"' EXIT
git archive "$base" | tar -x -C "$control"
# Fetch both locked graphs before the overlay needs offline resolution. A clean
# runner may lack dependencies unused by the harness helper binary.
cp "$control/Cargo.lock" "$evidence/control-original-Cargo.lock"
cargo fetch --locked --manifest-path "$control/Cargo.toml"
cargo fetch --locked --manifest-path "$repo_root/Cargo.toml"
harness=(tools/recite-perf crates/recite-benchmarks fixtures
  crates/recite-lsp/src/bench_support/driver.rs
  crates/recite-compiler/src/bench_support.rs)
for path in "${harness[@]}"; do
  rm -rf "${control:?}/$path"
  mkdir -p "$control/$(dirname "$path")"
  cp -R "$repo_root/$path" "$control/$path"
done
{
  printf 'control=%s\ncandidate=%s\nscope=%s\n' "$base" "$(git -C "$repo_root" rev-parse HEAD)" "$scope"
  printf 'valgrind_command=%s\n' "${GUNGRAUN_VALGRIND_BIN:-valgrind}"
  printf 'build_root=%s\n' "$build_root"
  rustc -Vv
  uname -a
  lscpu
  valgrind --version
  gungraun-runner --version
  git -C "$repo_root" status --short
} >"$evidence/environment.txt"
git -C "$repo_root" diff --binary HEAD >"$evidence/candidate.patch"
git ls-files --others --exclude-standard -z -- '*.rs' '*.toml' '*.sh' '*.just' >"$evidence/untracked-source-files"
tar --null -czf "$evidence/untracked-sources.tar.gz" --files-from "$evidence/untracked-source-files"
tar -czf "$evidence/harness.tar.gz" "${harness[@]}"
cp mise.perf.toml scripts/check-workflow-performance.sh "$evidence/"
cargo build --locked --release -p recite-perf
checker="${CARGO_TARGET_DIR:-$repo_root/target}/release/recite-perf"
"$checker" prepare "$repo_root" "$control"
cp "$control/crates/recite-lsp/src/bench_support/driver.rs" "$evidence/control-lsp-driver.rs"
cp "$control/tools/recite-perf/benches/support/completion.rs" "$evidence/control-completion.rs"
# Preserve the base lock; resolve only the current harness's additions offline.
CARGO_TARGET_DIR="$build_root/control-target" cargo metadata --offline --manifest-path "$control/Cargo.toml" --format-version 1 >"$evidence/control-metadata.json"
cp "$control/Cargo.lock" "$evidence/control-Cargo.lock"
cp "$repo_root/Cargo.lock" "$evidence/candidate-Cargo.lock"
for side in control candidate; do
  root="$repo_root"
  [[ "$side" != control ]] || root="$control"
  targets=()
  for bench in "${benches[@]}"; do targets+=(--bench "$bench"); done
  CARGO_TARGET_DIR="$build_root/$side-target" cargo bench --locked --manifest-path "$root/Cargo.toml" -p recite-perf "${targets[@]}" --features writer --no-run --message-format=json >"$evidence/$side-build.jsonl"
  for bench in "${benches[@]}"; do
    binary="$(jq -er --arg target "$bench" 'select(.reason == "compiler-artifact" and .target.name == $target and .executable != null) | .executable' "$evidence/$side-build.jsonl")"
    cp "$binary" "$output/$side-$bench"
  done
  if [[ "$scope" == pr || "$scope" == full ]]; then
    CARGO_TARGET_DIR="$build_root/$side-target" cargo build --locked --release --manifest-path "$root/Cargo.toml" -p recite-benchmarks --example compiler_allocations --example workflow_evidence
  fi
done
: >"$evidence/binaries.sha256"
for bench in "${benches[@]}"; do
  sha256sum "$output/control-$bench" "$output/candidate-$bench" >>"$evidence/binaries.sha256"
done
# Both binaries and fixtures are ready. No Cargo builds run in this measurement window.
export RECITE_BENCH_SCALES="$fixtures"
failure=0
for side in control candidate; do
  for bench in "${benches[@]}"; do
    selected=("${parameters[@]}")
    [[ "$bench" != workflows ]] || selected+=(sources_400)
    # Run whole authoring targets: a newly declared case must reach the
    # fail-closed coverage checker even when its expected-case list was not updated.
    [[ "$bench" != writer && "$bench" != authoring ]] || selected=('')
    [[ "$bench" != bound_text ]] || selected=(authored_64)
    : >"$evidence/$side-$bench.jsonl"
    for parameter in "${selected[@]}"; do
      baseline=(--save-baseline=control)
      [[ "$side" != candidate ]] || baseline=(--baseline=control)
      filter="*::$parameter"
      [[ "$bench" != writer && "$bench" != authoring ]] || filter='*'
      "$output/$side-$bench" --test=false --home "$evidence/profiles" "${baseline[@]}" --output-format=json --save-summary=json "$filter" >>"$evidence/$side-$bench.jsonl" || failure=$?
    done
  done
  if [[ "$scope" == pr || "$scope" == full ]]; then
    "$build_root/$side-target/release/examples/compiler_allocations" >"$evidence/$side-compiler-allocations.json"
    "$build_root/$side-target/release/examples/workflow_evidence" >"$evidence/$side-preview.json"
    jq -S '[.[] | {fixture, asset_hash, inspection_json_hash}] | sort_by(.fixture)' "$evidence/$side-compiler-allocations.json" >"$evidence/$side-assets.json"
    jq -S 'sort_by(.fixture)' "$evidence/$side-preview.json" >"$evidence/$side-events.json"
  fi
done
if [[ "$scope" == pr || "$scope" == full ]]; then
  diff -u "$evidence/control-assets.json" "$evidence/candidate-assets.json"
  diff -u "$evidence/control-events.json" "$evidence/candidate-events.json"
  "$checker" check "$scope" "$evidence/candidate-workflows.jsonl"
  "$checker" check bound-text "$evidence/candidate-bound_text.jsonl"
fi
if [[ "$scope" != writer ]]; then
  "$checker" check authoring "$evidence/candidate-authoring.jsonl"
fi
if [[ "$scope" != authoring ]]; then
  "$checker" check writer "$evidence/candidate-writer.jsonl"
fi
exit "$failure"
