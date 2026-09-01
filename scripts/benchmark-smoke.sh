#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  benchmark-smoke.sh [repo-root]

Runs Recite's fast, non-comparative benchmark smoke:
  1. RECITE_BENCH_SCALES=tiny cargo bench --locked -p recite-benchmarks --bench compiler -- 'compiler/.*/tiny' --test
  2. RECITE_BENCH_SCALES=tiny cargo bench --locked -p recite-benchmarks --bench runtime -- 'runtime/.*/tiny' --test
  3. RECITE_BENCH_SCALES=tiny cargo bench --locked -p recite-benchmarks --bench preview -- 'preview/.*/tiny' --test

The smoke only proves that the tiny compiler, runtime, and preview Criterion
benchmarks build and execute. It does not compare timings or enforce regression
thresholds.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi

if [[ $# -gt 1 ]]; then
  usage >&2
  exit 2
fi

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  if ! repo_root="$(git -C "$input_root" rev-parse --show-toplevel 2>/dev/null)"; then
    echo "repo root is not a git checkout: $input_root" >&2
    exit 2
  fi
else
  if ! repo_root="$(git rev-parse --show-toplevel 2>/dev/null)"; then
    echo "unable to resolve git repo root from current directory" >&2
    exit 2
  fi
fi

cargo_cmd="${CARGO:-cargo}"

if [[ -n "${RECITE_BENCH_SCALES:-}" && "${RECITE_BENCH_SCALES}" != "tiny" ]]; then
  echo "benchmark smoke ignores RECITE_BENCH_SCALES=${RECITE_BENCH_SCALES}; using tiny" >&2
fi
export RECITE_BENCH_SCALES=tiny

run_smoke_target() {
  local bench_name="$1"
  local filter="$2"

  echo
  echo "== ${bench_name} benchmark smoke =="
  (
    cd "$repo_root"
    "$cargo_cmd" bench --locked -p recite-benchmarks --bench "$bench_name" -- "$filter" --test
  )
}

run_smoke_target compiler 'compiler/.*/tiny'
run_smoke_target runtime 'runtime/.*/tiny'
run_smoke_target preview 'preview/.*/tiny'

echo
echo "Recite benchmark smoke passed."
