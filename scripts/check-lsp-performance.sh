#!/usr/bin/env bash
set -euo pipefail

# Build both revisions before measuring; never build or test during sampling.
# The current harness and fixture generator drive both binaries.
if [[ $# != 1 ]]; then
  echo "Usage: scripts/check-lsp-performance.sh BASE_COMMIT" >&2
  exit 2
fi
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
base="$(git rev-parse --verify "${1}^{commit}")"
output="$repo_root/target/lsp-performance"
mkdir -p "$output"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/recite-lsp-base.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
git archive "$base" | tar -x -C "$scratch"
cargo build --locked --release -p recite-lsp -p recite-fixturegen
cp target/release/recite-lsp "$output/candidate-lsp"
CARGO_TARGET_DIR="$output/base-target" cargo build --locked --release \
  --manifest-path "$scratch/Cargo.toml" -p recite-lsp
cp "$output/base-target/release/recite-lsp" "$output/control-lsp"
python3 - "$base" "$output/revisions.json" <<'PY'
import json, subprocess, sys
from pathlib import Path
Path(sys.argv[2]).write_text(json.dumps({"control": sys.argv[1],
    "candidate": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()}, indent=2) + "\n")
PY
target/release/recite-fixturegen --profile large \
  --output "$output/large" --summaries "$output/fixture.json"
python3 scripts/check-lsp-performance.py --control "$output/control-lsp" \
  --candidate "$output/candidate-lsp" --project "$output/large" \
  --output "$output/comparison.json"
python3 scripts/measure-lsp-session.py "$output/large" --edits 300 \
  --output "$output/session.json"
python3 scripts/measure-lsp-session.py "$output/large" --edits 300 --ranged \
  --output "$output/session-ranged.json"
python3 scripts/measure-lsp-session.py "$output/large" --edits 300 --ranged \
  --interval-ms 5 --burst-every 25 --pause-ms 150 --recovery-ms 500 \
  --output "$output/session-bursts.json"
