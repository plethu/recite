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
  mkdir -p "$test_root/repo/crates/demo/src" "$test_root/repo/crates/demo/tests" "$test_root/repo/crates/demo/benches" "$test_root/repo/tests" "$test_root/repo/docs" "$test_root/repo/scripts"
  cp "$repo_root/scripts/check-maintainability.sh" "$test_root/repo/scripts/check-maintainability.sh"
  chmod +x "$test_root/repo/scripts/check-maintainability.sh"
  # These literals intentionally contain Markdown code ticks.
  # shellcheck disable=SC2016
  printf '%s\n' \
    '# Maintainability fixture baseline' \
    '' \
    '## Inventory' \
    '' \
    '| Path | Lines | Kind | Owner | Disposition | Issue/reason |' \
    '| --- | ---: | --- | --- | --- | --- |' \
    '| `crates/demo/src/large.rs` | 401 | production | demo | cohesive | fixture |' \
    '| `crates/demo/tests/large.rs` | 501 | test/support | demo/tests | cohesive | fixture |' \
    '| `crates/demo/benches/large.rs` | 501 | test/support | demo/benches | cohesive | fixture |' \
    '| `tests/large.rs` | 501 | test/support | top-level-tests | cohesive | fixture |' \
    '| `crates/demo/src/new.rs` | 300 | production | demo | cohesive | fixture |' \
    '| `crates/demo/src/exception.rs` | 401 | production | demo | exception | #164: fixture exception reason |' \
    '| `crates/demo/src/tests.rs` | 351 | test/support | demo/tests | cohesive | fixture sidecar |' \
    '| `crates/demo/src/tests/support.rs` | 351 | test/support | demo/tests | cohesive | fixture support |' \
    > "$test_root/repo/docs/maintainability-baseline.md"
  write_lines crates/demo/src/large.rs 401
  write_lines crates/demo/tests/large.rs 501
  write_lines crates/demo/benches/large.rs 501
  write_lines tests/large.rs 501
  write_lines crates/demo/src/new.rs 300
  write_lines crates/demo/src/exception.rs 401
  write_lines crates/demo/src/tests.rs 351
  write_lines crates/demo/src/tests/support.rs 351
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

initial_push_fixture() {
  cleanup
  test_root="$(mktemp -d)"
  mkdir -p "$test_root/repo/crates/demo/src" "$test_root/repo/docs" "$test_root/repo/scripts"
  cp "$repo_root/scripts/check-maintainability.sh" "$test_root/repo/scripts/check-maintainability.sh"
  chmod +x "$test_root/repo/scripts/check-maintainability.sh"
  # This literal intentionally contains Markdown code ticks.
  # shellcheck disable=SC2016
  printf '%s\n' \
    '# Maintainability fixture baseline' \
    '' \
    '## Inventory' \
    '' \
    '| Path | Lines | Kind | Owner | Disposition | Issue/reason |' \
    '| --- | ---: | --- | --- | --- | --- |' \
    '| `crates/demo/src/new.rs` | 300 | production | demo | cohesive | initial push fixture |' \
    > "$test_root/repo/docs/maintainability-baseline.md"
  git -C "$test_root/repo" init -q -b main
  git -C "$test_root/repo" config user.name Fixture
  git -C "$test_root/repo" config user.email fixture@example.invalid
  git -C "$test_root/repo" config commit.gpgsign false
  write_lines crates/demo/src/new.rs 300
}

commit_fixture() {
  local message="$1"
  git -C "$test_root/repo" add .
  git -C "$test_root/repo" commit --allow-empty -q -m "$message"
}

update_baseline_lines() {
  local path="$1"
  local old_lines="$2"
  local new_lines="$3"
  sed -i "\#\`$path\` | $old_lines |#s#| $old_lines |#| $new_lines |#" \
    "$test_root/repo/docs/maintainability-baseline.md"
}

run_check_with_base() {
  local base_ref="$1"
  git -C "$test_root/repo" -c core.pager=cat show-ref --verify --quiet refs/heads/main
  (
    cd "$test_root/repo"
    scripts/check-maintainability.sh "$base_ref" HEAD
  )
}

run_check() {
  run_check_with_base "$(git -C "$test_root/repo" rev-parse HEAD^)"
}

run_full_check() {
  (
    cd "$test_root/repo"
    scripts/check-maintainability.sh --full
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

expect_full_pass() {
  local name="$1"
  if ! run_full_check >/dev/null; then
    echo "full maintainability fixture failed: $name" >&2
    exit 1
  fi
  echo "passed: $name"
}

expect_full_fail() {
  local name="$1"
  if run_full_check >/dev/null 2>&1; then
    echo "full maintainability fixture unexpectedly passed: $name" >&2
    exit 1
  fi
  echo "rejected: $name"
}

expect_initial_push_pass() {
  local name="$1"
  local zero_sha="0000000000000000000000000000000000000000"
  if ! run_check_with_base "$zero_sha" >/dev/null; then
    echo "maintainability fixture failed: $name" >&2
    exit 1
  fi
  echo "passed: $name"
}

new_fixture
commit_fixture baseline
expect_full_pass complete baseline inventory

new_fixture
commit_fixture baseline
write_lines tests/large.rs 502
update_baseline_lines tests/large.rs 501 502
commit_fixture grow
expect_fail growing oversized top-level test/support file

new_fixture
commit_fixture baseline
# This literal intentionally contains Markdown code ticks.
# shellcheck disable=SC2016
printf '%s\n' '| `crates/demo/src/new.rs` | 300 | production | demo | cohesive | duplicate fixture |' \
  >> "$test_root/repo/docs/maintainability-baseline.md"
expect_full_fail duplicate baseline row

new_fixture
commit_fixture baseline
sed -i 's#crates/demo/src/new.rs#crates/demo/src/missing.rs#' \
  "$test_root/repo/docs/maintainability-baseline.md"
expect_full_fail baseline path missing at head

new_fixture
commit_fixture baseline
sed -i '\#crates/demo/src/large.rs#d' \
  "$test_root/repo/docs/maintainability-baseline.md"
commit_fixture docs-only
expect_fail docs-only removal of unchanged oversized row

new_fixture
write_lines crates/demo/src/new.rs 200
update_baseline_lines crates/demo/src/new.rs 300 200
commit_fixture baseline
expect_full_fail stale row below scrutiny threshold

new_fixture
commit_fixture baseline
update_baseline_lines crates/demo/src/new.rs 300 299
expect_full_fail mismatched baseline line count

new_fixture
commit_fixture baseline
sed -i 's/| 300 | production | demo | cohesive | fixture |/| 300 | test\/support | demo | cohesive | fixture |/' \
  "$test_root/repo/docs/maintainability-baseline.md"
expect_full_fail incorrect baseline classification

new_fixture
commit_fixture baseline
sed -i 's/| 300 | production | demo | cohesive | fixture |/| 300 | production | demo | unknown | fixture |/' \
  "$test_root/repo/docs/maintainability-baseline.md"
expect_full_fail unknown baseline disposition

new_fixture
commit_fixture baseline
sed -i 's/#164: fixture exception reason/#164/' \
  "$test_root/repo/docs/maintainability-baseline.md"
expect_full_fail exception issue without reason

new_fixture
commit_fixture baseline
sed -i 's/#164: fixture exception reason/#0: malformed issue reference/' \
  "$test_root/repo/docs/maintainability-baseline.md"
expect_full_fail malformed issue/reason reference

initial_push_fixture
commit_fixture initial
expect_initial_push_pass initial push empty-tree fallback

new_fixture
write_lines crates/demo/src/large.rs 401
commit_fixture base
commit_fixture unchanged
expect_pass unchanged legacy production file

new_fixture
write_lines crates/demo/src/large.rs 401
commit_fixture base
write_lines crates/demo/src/large.rs 400
update_baseline_lines crates/demo/src/large.rs 401 400
commit_fixture shrink
expect_pass shrinking oversized production file

new_fixture
commit_fixture base
mv "$test_root/repo/crates/demo/src/large.rs" "$test_root/repo/crates/demo/src/renamed.rs"
sed -i 's#crates/demo/src/large.rs#crates/demo/src/renamed.rs#' \
  "$test_root/repo/docs/maintainability-baseline.md"
commit_fixture rename
expect_pass unchanged oversized source rename with updated inventory

new_fixture
commit_fixture base
mv "$test_root/repo/crates/demo/src/large.rs" "$test_root/repo/crates/demo/src/renamed.rs"
sed -i 's#crates/demo/src/large.rs#crates/demo/src/renamed.rs#' \
  "$test_root/repo/docs/maintainability-baseline.md"
write_lines crates/demo/src/renamed.rs 402
update_baseline_lines crates/demo/src/renamed.rs 401 402
commit_fixture rename-grow
expect_fail renamed and grown oversized source file

new_fixture
commit_fixture base
rm "$test_root/repo/crates/demo/src/new.rs"
sed -i '\#crates/demo/src/new.rs#d' \
  "$test_root/repo/docs/maintainability-baseline.md"
commit_fixture deletion
expect_pass explicit source deletion with removed inventory row

new_fixture
write_lines crates/demo/src/large.rs 401
commit_fixture base
write_lines crates/demo/src/large.rs 402
update_baseline_lines crates/demo/src/large.rs 401 402
commit_fixture grow
expect_fail growing oversized production file

new_fixture
write_lines crates/demo/src/large.rs 399
update_baseline_lines crates/demo/src/large.rs 401 399
commit_fixture base
write_lines crates/demo/src/large.rs 401
update_baseline_lines crates/demo/src/large.rs 399 401
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
update_baseline_lines crates/demo/tests/large.rs 501 502
commit_fixture grow
expect_fail growing oversized test/support file

new_fixture
write_lines crates/demo/benches/large.rs 501
commit_fixture base
write_lines crates/demo/benches/large.rs 502
update_baseline_lines crates/demo/benches/large.rs 501 502
commit_fixture grow
expect_fail growing oversized benchmark support file

new_fixture
write_lines crates/demo/src/exception.rs 401
commit_fixture base
write_lines crates/demo/src/exception.rs 402
update_baseline_lines crates/demo/src/exception.rs 401 402
commit_fixture grow
expect_pass issue-linked local exception

new_fixture
write_lines crates/demo/src/exception.rs 401
commit_fixture base
write_lines crates/demo/src/exception.rs 402
update_baseline_lines crates/demo/src/exception.rs 401 402
sed -i 's/#164: fixture exception reason/#0: malformed issue reference/' \
  "$test_root/repo/docs/maintainability-baseline.md"
commit_fixture grow
expect_fail malformed exception without issue/reason

new_fixture
write_lines crates/demo/src/tests.rs 351
write_lines crates/demo/src/tests/support.rs 351
commit_fixture base
write_lines crates/demo/src/tests.rs 352
write_lines crates/demo/src/tests/support.rs 352
update_baseline_lines crates/demo/src/tests.rs 351 352
update_baseline_lines crates/demo/src/tests/support.rs 351 352
commit_fixture grow
expect_pass source-side test sidecars use test thresholds

echo "maintainability fixtures passed"
