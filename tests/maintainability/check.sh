#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root=""
trap '[[ -z "$test_root" ]] || rm -rf "$test_root"' EXIT

fixture() {
  [[ -z "$test_root" ]] || rm -rf "$test_root"
  test_root="$(mktemp -d)"
  mkdir -p "$test_root/repo/scripts/maintainability" "$test_root/repo/crates/demo/src" "$test_root/repo/crates/demo/tests"
  cp "$repo_root/scripts/check-maintainability.sh" "$test_root/repo/scripts/"
  cp "$repo_root/scripts/maintainability"/{*.sh,*.py} "$test_root/repo/scripts/maintainability/"
  cat > "$test_root/repo/scripts/maintainability/exceptions.toml" <<'TOML'
[[exceptions]]
path = "crates/demo/src/excepted.rs"
max_lines = 403
issue = "#164"
reason = "Fixture growth allowance while the decoder is split"
TOML
  lines crates/demo/src/large.rs 401
  lines crates/demo/src/excepted.rs 401
  lines crates/demo/tests/large.rs 501
  git -C "$test_root/repo" init -q -b main
  git -C "$test_root/repo" config user.name Fixture
  git -C "$test_root/repo" config user.email fixture@example.invalid
  git -C "$test_root/repo" config commit.gpgsign false
}

lines() {
  mkdir -p "$(dirname "$test_root/repo/$1")"
  awk -v count="$2" 'BEGIN { for (i=0; i<count; i++) print "// fixture" }' > "$test_root/repo/$1"
}

save() {
  git -C "$test_root/repo" add .
  git -C "$test_root/repo" commit --allow-empty -qm fixture
}

check() {
  (cd "$test_root/repo" && env -u RECITE_BASE_REF -u RECITE_HEAD_REF scripts/check-maintainability.sh "$1" HEAD)
}

assert_check() {
  local expected="$1" label="$2" base="$3" result
  if result="$(check "$base" 2>&1)"; then
    [[ "$expected" == pass ]] || { echo "unexpected pass: $label" >&2; exit 1; }
  else
    [[ "$expected" == fail ]] || { echo "unexpected failure: $label" >&2; echo "$result" >&2; exit 1; }
  fi
  echo "$expected: $label"
}

previous() { git -C "$test_root/repo" rev-parse HEAD^; }

fixture; save; save
assert_check pass 'unchanged oversized files' "$(previous)"

fixture; save; lines crates/demo/src/large.rs 400; save
assert_check pass 'shrinking oversized file' "$(previous)"

fixture; save; lines crates/demo/src/large.rs 402; save
assert_check fail 'growing oversized file' "$(previous)"

fixture; save; lines crates/demo/src/excepted.rs 402; save
assert_check pass 'bounded exception growth' "$(previous)"

fixture; save; lines crates/demo/src/excepted.rs 404; save
assert_check fail 'exception maximum exceeded' "$(previous)"

fixture; save; lines crates/demo/src/excepted.rs 400; save
assert_check fail 'expired exception below follow-up threshold' "$(previous)"

fixture; save; sed -i 's/#164/#0/' "$test_root/repo/scripts/maintainability/exceptions.toml"; save
assert_check fail 'malformed issue' "$(previous)"

fixture; save; cat >> "$test_root/repo/scripts/maintainability/exceptions.toml" <<'TOML'
[[exceptions]]
path = "crates/demo/src/excepted.rs"
max_lines = 403
issue = "#164"
reason = "duplicate"
TOML
save
assert_check fail 'duplicate exception path' "$(previous)"

fixture; save; mv "$test_root/repo/crates/demo/src/large.rs" "$test_root/repo/crates/demo/src/renamed.rs"; save
assert_check pass 'unchanged rename' "$(previous)"

fixture; save; mv "$test_root/repo/crates/demo/src/large.rs" "$test_root/repo/crates/demo/src/renamed.rs"; lines crates/demo/src/renamed.rs 402; save
assert_check fail 'growing rename' "$(previous)"

fixture; save; lines crates/demo/src/new.rs 401; save
assert_check fail 'new oversized production file' "$(previous)"

fixture; save; lines crates/demo/tests/large.rs 502; save
assert_check fail 'growing test support file' "$(previous)"

fixture; save; lines crates/demo/src/large.rs 402; save
assert_check fail 'hostile refs ignored when explicit refs supplied' "$(previous)"

fixture; save; lines editors/vscode/src/messages.generated.js 501; save
assert_check pass 'generated source excluded' "$(previous)"

fixture; save; lines crates/demo/src/new.rs 300; save
assert_check pass 'scrutiny threshold is advisory' "$(previous)"

fixture; lines crates/demo/src/large.rs 400; lines crates/demo/tests/large.rs 500; save; lines crates/demo/src/excepted.rs 402; save
assert_check pass 'initial push exception' 0000000000000000000000000000000000000000

echo 'maintainability fixtures passed'
