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
# A control that predates trailing stable IDs (#234) cannot read the current
# fixtures. This waiver expires once every comparison base has the new format.
compare_with_base=true
if [[ ! -f "$scratch/crates/recite-core/src/statement_header.rs" ]]; then
  compare_with_base=false
  echo "Base $base predates the trailing stable-ID source format; skipping the base comparison."
  printf '{"status":"waived","reason":"control predates the trailing stable-ID source format (#234)","control":"%s"}\n' \
    "$base" >"$output/comparison.json"
else
  CARGO_TARGET_DIR="$output/base-target" cargo build --locked --release \
    --manifest-path "$scratch/Cargo.toml" -p recite-lsp
  cp "$output/base-target/release/recite-lsp" "$output/control-lsp"
fi
python3 - "$base" "$output/revisions.json" <<'PY'
import json, subprocess, sys
from pathlib import Path
Path(sys.argv[2]).write_text(json.dumps({"control": sys.argv[1],
    "candidate": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()}, indent=2) + "\n")
PY
target/release/recite-fixturegen --profile large \
  --output "$output/large" --summaries "$output/fixture.json"
if [[ "$compare_with_base" == true ]]; then
  mise -E lsp exec -- uv run --locked python scripts/lsp.py compare --control "$output/control-lsp" \
    --candidate "$output/candidate-lsp" --project "$output/large" \
    --output "$output/comparison.json"
fi
mise -E lsp exec -- uv run --locked python scripts/lsp.py session "$output/large" --edits 300 \
  --output "$output/session.json"
mise -E lsp exec -- uv run --locked python scripts/lsp.py session "$output/large" --edits 300 --ranged \
  --output "$output/session-ranged.json"
mise -E lsp exec -- uv run --locked python scripts/lsp.py session "$output/large" --edits 300 --ranged \
  --interval-ms 5 --burst-every 25 --pause-ms 150 --recovery-ms 500 \
  --output "$output/session-bursts.json"
