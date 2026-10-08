#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$repo_root/target}"
artifact_dir="$target_dir/recite-bevy-probe"
mkdir -p "$artifact_dir"
source_file="$repo_root/fixtures/recite/valid/adapter_conformance/runtime_surface.recite"
temporary="$(mktemp -d "${TMPDIR:-/tmp}/recite-bevy-perf.XXXXXX")"
trap 'rm -rf "$temporary"' EXIT

CARGO_TARGET_DIR="$target_dir" cargo build -p recite-bevy --example performance_probe --offline
CARGO_TARGET_DIR="$target_dir" cargo run --quiet -p recite-cli --offline -- compile --output "$artifact_dir/runtime.recitec" "$source_file"
sed 's/Intro line\./Intro line changed./' "$source_file" >"$temporary/changed.recite"
CARGO_TARGET_DIR="$target_dir" cargo run --quiet -p recite-cli --offline -- compile --output "$artifact_dir/changed.recitec" "$temporary/changed.recite"

probe="$target_dir/debug/examples/performance_probe"
report="$artifact_dir/performance.txt"
{
  date -u '+observed_utc=%Y-%m-%dT%H:%M:%SZ'
  rustc --version
  cargo --version
  printf 'profile=debug engine=bevy-0.19.1 platform=%s\n' "$(uname -sm)"
  if [[ -r /proc/cpuinfo ]]; then
    rg -m1 '^model name' /proc/cpuinfo || true
  fi
  printf 'timer=Python-perf_counter-and-wait4 units=seconds,KiB thresholds=none\n'
  python3 - "$probe" "$artifact_dir" <<'PY'
import os
import subprocess
import sys
import time

probe, artifacts = sys.argv[1:]
for mode, count in (("load", 10000), ("idle", 10000), ("active", 1000), ("retained", 1)):
    command = [probe, mode, f"{artifacts}/runtime.recitec", str(count)]
    if mode == "retained":
        command.append(f"{artifacts}/changed.recitec")
    start = time.perf_counter()
    process = subprocess.Popen(command)
    _, status, usage = os.wait4(process.pid, 0)
    elapsed = time.perf_counter() - start
    if os.waitstatus_to_exitcode(status) != 0:
        raise SystemExit(f"{mode} probe failed")
    print(f"{mode} elapsed={elapsed:.3f} user={usage.ru_utime:.3f} peak_rss_kib={usage.ru_maxrss}", flush=True)
PY
} 2>&1 | tee "$report"
printf 'Informational report: %s\n' "$report"
