#!/usr/bin/env bash
set -euo pipefail

if (( $# != 1 )) || [[ "$1" != "vscode" && "$1" != "neovim" && "$1" != "zed" ]]; then
  echo "Usage: scripts/run-editor-host-check.sh {vscode|neovim|zed}" >&2
  exit 2
fi

host="$1"
repo_root="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
artifact_parent="${RECITE_EDITOR_EVIDENCE_DIR:-$repo_root/target/editor-host-evidence}"
mkdir -p "$artifact_parent"
artifact_dir="$(mktemp -d "$artifact_parent/$host.XXXXXX")"
runner="$repo_root/scripts/check-$host-host.sh"
started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
set +e
"$runner" "$repo_root" 2>&1 | tee "$artifact_dir/run.log"
result="${PIPESTATUS[0]}"
set -e
finished="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
python3 - "$artifact_dir/run.json" "$host" "$started" "$finished" "$result" <<'PY'
import json
import pathlib
import sys
path, host, started, finished, result = sys.argv[1:]
pathlib.Path(path).write_text(json.dumps({
    "host": host,
    "runner": f"scripts/check-{host}-host.sh",
    "started_utc": started,
    "finished_utc": finished,
    "exit_code": int(result),
    "log": "run.log"
}, indent=2) + "\n")
PY
printf 'Editor host run artifact: %s\n' "$artifact_dir"
exit "$result"
