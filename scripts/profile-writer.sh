#!/usr/bin/env bash
# Profile the same generated workload used by bench-writer; never profile Cargo.
set -euo pipefail
mode="${1:-cpu}"
passages="${2:-10000}"
output="${3:-/tmp/recite-writer-profile}"
case "$mode" in
  cpu) features=benchmarks ;;
  heap) features=heap-profile ;;
  *) echo "usage: profile-writer.sh [cpu|heap] [passages] [output-directory]" >&2; exit 2 ;;
esac
if [[ ! "$passages" =~ ^[1-9][0-9]*$ ]]; then
  echo "passages must be positive" >&2; exit 2
fi
if [[ "$mode" == cpu ]]; then command -v perf >/dev/null; fi
repo_root="$(git rev-parse --show-toplevel)"
mkdir -p "$output"
output="$(cd "$output" && pwd)"
# Refuse to replace evidence from an earlier run.
if [[ -e "$output/run.json" || -e "$output/perf.data" || -e "$output/dhat-heap.json" ]]; then
  echo "choose a fresh output directory" >&2; exit 2
fi
cargo bench --locked --manifest-path "$repo_root/apps/writer/Cargo.toml" \
  -p recite-writer-model --features "$features" --bench large_project \
  --no-run --message-format=json > "$output/build.json"
executable="$(python3 - "$output/build.json" <<'PY'
import json, sys
for line in open(sys.argv[1]):
    message = json.loads(line)
    if message.get('executable') and message.get('target', {}).get('name') == 'large_project':
        print(message['executable'])
PY
)"
[[ -x "$executable" ]]
{
  git -C "$repo_root" rev-parse HEAD
  git -C "$repo_root" status --short
  rustc --version
  uname -sm
} > "$output/environment.txt"
cd "$output"
if [[ "$mode" == cpu ]]; then
  perf record -o perf.data --call-graph dwarf -- "$executable" \
    --passages "$passages" --output run.json > run.log
  perf report -i perf.data --stdio --no-children > perf-report.txt
else
  "$executable" --passages "$passages" --output run.json > run.log
fi
printf 'Profile saved to %s\n' "$output"
