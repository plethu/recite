#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root=""

cleanup() {
  if [[ -n "$test_root" ]]; then
    rm -rf "$test_root"
  fi
}
trap cleanup EXIT

new_fixture() {
  cleanup
  test_root="$(mktemp -d)"
  mkdir -p "$test_root/repo/crates/demo/src" "$test_root/repo/crates/demo/tests" "$test_root/repo/docs" "$test_root/repo/scripts"
  cp "$repo_root/scripts/check-maintainability.sh" "$test_root/repo/scripts/check-maintainability.sh"
  chmod +x "$test_root/repo/scripts/check-maintainability.sh"
  # These literals intentionally contain Markdown code ticks.
  # shellcheck disable=SC2016
  printf '%s\n' \
    '# Maintainability fixture baseline' \
    '' \
    '| Path | Lines | Owner | Disposition | Issue/reason |' \
    '| --- | ---: | --- | --- | --- |' \
    '| `crates/demo/src/large.rs` | 401 | demo | cohesive | fixture |' \
    '| `crates/demo/tests/large.rs` | 501 | demo/tests | cohesive | fixture |' \
    '| `crates/demo/src/new.rs` | 300 | demo | cohesive | fixture |' \
    '| `crates/demo/src/exception.rs` | 401 | demo | exception | #164: fixture exception reason |' \
    '| `crates/demo/src/malformed-exception.rs` | 401 | demo | exception | |' \
    '| `crates/demo/src/tests.rs` | 351 | demo/tests | cohesive | fixture sidecar |' \
    '| `crates/demo/src/tests/support.rs` | 351 | demo/tests | cohesive | fixture support |' \
    > "$test_root/repo/docs/maintainability-baseline.md"
  git -C "$test_root/repo" init -q -b main
  git -C "$test_root/repo" config user.name Fixture
  git -C "$test_root/repo" config user.email fixture@example.invalid
  git -C "$test_root/repo" config commit.gpgsign false
}

write_lines() {
  local path="$1"
  local count="$2"
  mkdir -p "$(dirname "$test_root/repo/$path")"
  awk -v count="$count" 'BEGIN { for (line = 1; line <= count; line++) print "// fixture" }' \
    > "$test_root/repo/$path"
}

commit_fixture() {
  local message="$1"
  git -C "$test_root/repo" add .
  git -C "$test_root/repo" commit --allow-empty -q -m "$message"
}

run_check() {
  git -C "$test_root/repo" -c core.pager=cat show-ref --verify --quiet refs/heads/main
  (
    cd "$test_root/repo"
    scripts/check-maintainability.sh "$(git rev-parse HEAD^)" HEAD
  )
}

expect_pass() {
  local name="$1"
  if ! run_check >/dev/null; then
    echo "maintainability fixture failed: $name" >&2
    exit 1
  fi
  echo "passed: $name"
}

expect_fail() {
  local name="$1"
  if run_check >/dev/null 2>&1; then
    echo "maintainability fixture unexpectedly passed: $name" >&2
    exit 1
  fi
  echo "rejected: $name"
}

new_fixture
write_lines crates/demo/src/large.rs 401
commit_fixture base
commit_fixture unchanged
expect_pass unchanged legacy production file

new_fixture
write_lines crates/demo/src/large.rs 401
commit_fixture base
write_lines crates/demo/src/large.rs 400
commit_fixture shrink
expect_pass shrinking oversized production file

new_fixture
write_lines crates/demo/src/large.rs 401
commit_fixture base
write_lines crates/demo/src/large.rs 402
commit_fixture grow
expect_fail growing oversized production file

new_fixture
write_lines crates/demo/src/large.rs 399
commit_fixture base
write_lines crates/demo/src/large.rs 401
commit_fixture cross
expect_fail crossing production follow-up threshold

new_fixture
commit_fixture base
write_lines crates/demo/src/brand_new.rs 401
commit_fixture new
expect_fail new oversized production file

new_fixture
commit_fixture base
write_lines crates/demo/src/new.rs 300
commit_fixture new
expect_pass new scrutiny-only production file with baseline row

new_fixture
commit_fixture base
write_lines crates/demo/src/undocumented.rs 300
commit_fixture new
expect_fail new scrutiny-only file without baseline row

new_fixture
write_lines crates/demo/tests/large.rs 501
commit_fixture base
write_lines crates/demo/tests/large.rs 502
commit_fixture grow
expect_fail growing oversized test/support file

new_fixture
write_lines crates/demo/src/exception.rs 401
commit_fixture base
write_lines crates/demo/src/exception.rs 402
commit_fixture grow
expect_pass issue-linked local exception

new_fixture
write_lines crates/demo/src/malformed-exception.rs 401
commit_fixture base
write_lines crates/demo/src/malformed-exception.rs 402
commit_fixture grow
expect_fail malformed exception without issue/reason

new_fixture
write_lines crates/demo/src/tests.rs 351
write_lines crates/demo/src/tests/support.rs 351
commit_fixture base
write_lines crates/demo/src/tests.rs 352
write_lines crates/demo/src/tests/support.rs 352
commit_fixture grow
expect_pass source-side test sidecars use test thresholds

echo "maintainability fixtures passed"
