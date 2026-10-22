#!/usr/bin/env bash
# Frozen upstream policy fixtures share the parent suite assertion helpers.
check_upstream_suppressions() (
  local fixture_root="$1"
  local ignored_parent="$fixture_root/ignored-parent"
  mkdir -p "$ignored_parent/probe"
  printf '%s\n' '*.rs' >"$ignored_parent/.ignore"
  printf '%s\n' '#![allow(dead_code)]' 'fn ignored_probe() {}' >"$ignored_parent/probe/source.rs"
  local hidden
  hidden="$(ast-grep scan --inline-rules 'id: parent-ignore-control
language: Rust
rule:
  kind: inner_attribute_item' --json=stream --no-ignore hidden "$ignored_parent/probe")"
  [[ -z "$hidden" ]] || {
    echo 'parent-ignore negative control did not hide the probe' >&2
    exit 1
  }
  export TMPDIR="$ignored_parent"
  upstream_base="$(git -C "$fixture_root/repo" rev-parse HEAD)"
  upstream_path=apps/writer/vendor/xilem/masonry/src/lib.rs
  mkdir -p "$fixture_root/repo/$(dirname "$upstream_path")"
  printf '%s\n' '#![allow(dead_code)]' 'fn upstream() {}' >"$fixture_root/repo/$upstream_path"
  python3 - "$fixture_root/repo" "$upstream_path" <<'PY'
import hashlib, json, sys
from pathlib import Path
root, source = Path(sys.argv[1]), sys.argv[2]
entry = {
    "path": source,
    "upstream_path": source.removeprefix("apps/writer/vendor/xilem/"),
    "sha256": hashlib.sha256((root / source).read_bytes()).hexdigest(),
    "revision": "271a27a6d4a930f7878d404f9014e3c50a3a9b88",
}
(root / "scripts/lint-suppressions-upstream.json").write_text(json.dumps([entry]))
PY
  git -C "$fixture_root/repo" add .
  git -C "$fixture_root/repo" add -f "$upstream_path"
  git -C "$fixture_root/repo" commit -qm frozen-upstream
  frozen_sha="$(git -C "$fixture_root/repo" rev-parse HEAD)"
  check_passes "$upstream_base" "$frozen_sha"
  # Even an unrelated whitespace change invalidates approval of the whole file.
  printf '\n' >>"$fixture_root/repo/$upstream_path"
  git -C "$fixture_root/repo" add .
  git -C "$fixture_root/repo" commit -qm changed-frozen-source
  check_fails "$frozen_sha" HEAD "frozen upstream lint baseline changed"
  git -C "$fixture_root/repo" checkout "$frozen_sha" -- "$upstream_path"
  printf '%s\n' '#![allow(dead_code)]' 'fn new_vendor_file() {}' > \
    "$fixture_root/repo/$(dirname "$upstream_path")/new.rs"
  git -C "$fixture_root/repo" add .
  git -C "$fixture_root/repo" add -f "$(dirname "$upstream_path")/new.rs"
  git -C "$fixture_root/repo" commit -qm unapproved-vendor-source
  check_fails "$frozen_sha" HEAD "crate/module-wide suppressions are not permitted"
  git -C "$fixture_root/repo" rm -q "$(dirname "$upstream_path")/new.rs"
  git -C "$fixture_root/repo" commit -qm remove-unapproved
  for invalid in path sha256 revision; do
    python3 - "$fixture_root/repo" "$frozen_sha" "$invalid" <<'PY'
import json, subprocess, sys
from pathlib import Path
root, revision, field = sys.argv[1:]
manifest = "scripts/lint-suppressions-upstream.json"
entries = json.loads(subprocess.check_output(["git", "-C", root, "show", revision + ":" + manifest]))
entries[0][field] = "crates/demo/src/lib.rs" if field == "path" else "not-a-digest"
(Path(root) / manifest).write_text(json.dumps(entries))
PY
    git -C "$fixture_root/repo" add .
    git -C "$fixture_root/repo" commit -qm invalid-upstream-manifest
    check_fails "$frozen_sha" HEAD "invalid upstream lint baseline path or digest"
  done
  echo 'frozen upstream suppression fixtures passed'
)
