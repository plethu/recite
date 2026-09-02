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

copy_gate() {
  cp "$repo_root/scripts/check-maintainability.sh" "$test_root/repo/scripts/check-maintainability.sh"
  mkdir -p "$test_root/repo/scripts/maintainability"
  cp "$repo_root/scripts/maintainability"/*.sh "$test_root/repo/scripts/maintainability/"
  chmod +x "$test_root/repo/scripts/check-maintainability.sh" "$test_root/repo/scripts/maintainability"/*.sh
}

write_lines() {
  local path="$1"
  local count="$2"
  mkdir -p "$(dirname "$test_root/repo/$path")"
  awk -v count="$count" 'BEGIN { for (line = 1; line <= count; line++) print "// fixture" }' \
    > "$test_root/repo/$path"
}

new_multilang_fixture() {
  cleanup
  test_root="$(mktemp -d)"
  mkdir -p "$test_root/repo/docs" "$test_root/repo/editors/vscode/src" \
    "$test_root/repo/editors/vscode/test" "$test_root/repo/scripts" \
    "$test_root/repo/tests" "$test_root/repo/.agents/skills/demo/scripts"
  copy_gate
  # These literals intentionally contain Markdown code ticks.
  # shellcheck disable=SC2016
  printf '%s\n' \
    '# Maintainability fixture baseline' '' '## Inventory' '' \
    '| Path | Lines | Kind | Owner | Disposition | Issue/reason |' \
    '| --- | ---: | --- | --- | --- | --- |' \
    > "$test_root/repo/docs/maintainability-baseline.md"
  local extension
  for extension in js mjs cjs lua py sh; do
    # Each extension is exercised in all three policy categories.
    printf '%s\n' \
      "| \`editors/vscode/src/large.$extension\` | 401 | production | editor-runtime | cohesive | fixture $extension production |" \
      "| \`scripts/large.$extension\` | 401 | tooling | scripts | cohesive | fixture $extension tooling |" \
      "| \`tests/large.$extension\` | 501 | test/support | tests | cohesive | fixture $extension test support |" \
      >> "$test_root/repo/docs/maintainability-baseline.md"
    write_lines "editors/vscode/src/large.$extension" 401
    write_lines "scripts/large.$extension" 401
    write_lines "tests/large.$extension" 501
  done
  printf '%s\n' \
    "| \`editors/vscode/dist/handwritten.js\` | 401 | production | editor-distribution | cohesive | force-tracked dist lookalike |" \
    >> "$test_root/repo/docs/maintainability-baseline.md"
  # Generated boundaries are explicit, and therefore remain outside the
  # handwritten inventory even when they use a supported extension.
  write_lines editors/vscode/src/messages.generated.js 501
  write_lines editors/vscode/dist/handwritten.js 401
  write_lines editors/recite-neovim/lua/recite_messages.lua 501
  write_lines editors/recite-tree-sitter/src/parser.c 501
  write_lines editors/recite-tree-sitter/src/grammar.json 501
  write_lines editors/recite-tree-sitter/src/node-types.json 501
  git -C "$test_root/repo" init -q -b main
  git -C "$test_root/repo" config user.name Fixture
  git -C "$test_root/repo" config user.email fixture@example.invalid
  git -C "$test_root/repo" config commit.gpgsign false
}

new_symlink_fixture() {
  cleanup
  test_root="$(mktemp -d)"
  mkdir -p "$test_root/repo/docs" "$test_root/repo/editors/vscode/src" "$test_root/repo/scripts"
  copy_gate
  # These rows intentionally exist so failure comes from the object mode,
  # rather than from a missing inventory entry or an accidental line count.
  # shellcheck disable=SC2016
  printf '%s\n' \
    '# Maintainability symlink fixture baseline' '' '## Inventory' '' \
    '| Path | Lines | Kind | Owner | Disposition | Issue/reason |' \
    '| --- | ---: | --- | --- | --- | --- |' \
    '| `scripts/large.sh` | 501 | tooling | scripts | cohesive | symlink target source |' \
    '| `editors/vscode/src/linked.js` | 1 | production | editor-runtime | cohesive | symlink to large source |' \
    '| `editors/vscode/src/escape.sh` | 1 | production | editor-runtime | cohesive | escaping symlink |' \
    > "$test_root/repo/docs/maintainability-baseline.md"
  write_lines scripts/large.sh 501
  ln -s ../../../scripts/large.sh "$test_root/repo/editors/vscode/src/linked.js"
  ln -s ../../../../outside.sh "$test_root/repo/editors/vscode/src/escape.sh"
  git -C "$test_root/repo" init -q -b main
  git -C "$test_root/repo" config user.name Fixture
  git -C "$test_root/repo" config user.email fixture@example.invalid
  git -C "$test_root/repo" config commit.gpgsign false
}

commit_fixture() {
  local message="$1"
  git -C "$test_root/repo" add .
  # Dist is commonly ignored by editor tooling; force-track this hostile
  # source lookalike so the maintainability gate must govern it.
  if [[ -e "$test_root/repo/editors/vscode/dist/handwritten.js" ]]; then
    git -C "$test_root/repo" add -f editors/vscode/dist/handwritten.js
  fi
  git -C "$test_root/repo" commit --allow-empty -q -m "$message"
}

update_baseline_lines() {
  local path="$1"
  local old_lines="$2"
  local new_lines="$3"
  sed -i "\#\`$path\` | $old_lines |#s#| $old_lines |#| $new_lines |#" \
    "$test_root/repo/docs/maintainability-baseline.md"
}

run_check() {
  local base_ref="$1"
  (
    cd "$test_root/repo"
    env -u RECITE_BASE_REF -u RECITE_HEAD_REF \
      scripts/check-maintainability.sh "$base_ref" HEAD
  )
}

run_full_check() {
  (
    cd "$test_root/repo"
    env -u RECITE_BASE_REF -u RECITE_HEAD_REF \
      scripts/check-maintainability.sh --full
  )
}

expect_pass() {
  local name="$1"
  if ! run_check "$(git -C "$test_root/repo" rev-parse HEAD^)" >/dev/null; then
    echo "maintainability fixture failed: $name" >&2
    exit 1
  fi
  echo "passed: $name"
}

expect_fail() {
  local name="$1"
  if run_check "$(git -C "$test_root/repo" rev-parse HEAD^)" >/dev/null 2>&1; then
    echo "maintainability fixture unexpectedly passed: $name" >&2
    exit 1
  fi
  echo "rejected: $name"
}

expect_fail_contains() {
  local name="$1"
  local needle="$2"
  local output
  if output="$(run_check "$(git -C "$test_root/repo" rev-parse HEAD^)" 2>&1)"; then
    echo "maintainability fixture unexpectedly passed: $name" >&2
    exit 1
  fi
  if [[ "$output" != *"$needle"* ]]; then
    echo "maintainability fixture failed to report $needle: $name" >&2
    echo "$output" >&2
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

expect_full_fail_contains() {
  local name="$1"
  local needle="$2"
  local output
  if output="$(run_full_check 2>&1)"; then
    echo "full maintainability fixture unexpectedly passed: $name" >&2
    exit 1
  fi
  if [[ "$output" != *"$needle"* ]]; then
    echo "full maintainability fixture failed to report $needle: $name" >&2
    echo "$output" >&2
    exit 1
  fi
  echo "rejected: $name"
}

new_multilang_fixture
commit_fixture baseline
expect_full_pass all supported extensions and explicit generated exclusions

new_multilang_fixture
commit_fixture base
write_lines editors/vscode/dist/handwritten.js 402
update_baseline_lines editors/vscode/dist/handwritten.js 401 402
commit_fixture grow
expect_fail growing force-tracked VS Code dist source

new_multilang_fixture
commit_fixture base
sed -i 's/| 401 | tooling | scripts |/| 401 | production | scripts |/' \
  "$test_root/repo/docs/maintainability-baseline.md"
expect_full_fail incorrect tooling classification

new_multilang_fixture
commit_fixture base
write_lines editors/vscode/src/large.js 402
update_baseline_lines editors/vscode/src/large.js 401 402
commit_fixture grow
expect_fail growing production JavaScript file

new_multilang_fixture
commit_fixture base
write_lines scripts/large.py 402
update_baseline_lines scripts/large.py 401 402
commit_fixture grow
expect_fail growing tooling Python file

new_multilang_fixture
commit_fixture base
write_lines tests/large.lua 502
update_baseline_lines tests/large.lua 501 502
commit_fixture grow
expect_fail growing test/support Lua file

new_multilang_fixture
commit_fixture base
mv "$test_root/repo/scripts/large.py" "$test_root/repo/scripts/renamed.py"
sed -i 's#scripts/large.py#scripts/renamed.py#' \
  "$test_root/repo/docs/maintainability-baseline.md"
commit_fixture rename-tooling
expect_pass unchanged oversized tooling rename

new_symlink_fixture
commit_fixture symlinks
expect_full_fail_contains "eligible source symlinks fail closed" 'not a regular file'

new_fixture() {
  cleanup
  test_root="$(mktemp -d)"
  mkdir -p "$test_root/repo/crates/demo/src" "$test_root/repo/crates/demo/tests" "$test_root/repo/docs" "$test_root/repo/scripts"
  copy_gate
  # These literals intentionally contain Markdown code ticks.
  # shellcheck disable=SC2016
  printf '%s\n' \
    '# Maintainability fixture baseline' '' '## Inventory' '' \
    '| Path | Lines | Kind | Owner | Disposition | Issue/reason |' \
    '| --- | ---: | --- | --- | --- | --- |' \
    '| `crates/demo/src/large.rs` | 401 | production | demo | cohesive | fixture |' \
    '| `crates/demo/tests/large.rs` | 501 | test/support | demo/tests | cohesive | fixture |' \
    > "$test_root/repo/docs/maintainability-baseline.md"
  write_lines crates/demo/src/large.rs 401
  write_lines crates/demo/tests/large.rs 501
  git -C "$test_root/repo" init -q -b main
  git -C "$test_root/repo" config user.name Fixture
  git -C "$test_root/repo" config user.email fixture@example.invalid
  git -C "$test_root/repo" config commit.gpgsign false
}

new_fixture
commit_fixture base
mv "$test_root/repo/crates/demo/tests/large.rs" "$test_root/repo/crates/demo/src/renamed.rs"
sed -i 's#crates/demo/tests/large.rs#crates/demo/src/renamed.rs#' \
  "$test_root/repo/docs/maintainability-baseline.md"
sed -i 's#| 501 | test/support | demo/tests | cohesive | fixture |#| 501 | production | demo | cohesive | fixture |#' \
  "$test_root/repo/docs/maintainability-baseline.md"
commit_fixture rename-stricter-category
expect_fail_contains "unchanged test-to-production rename is newly governed" \
  'policy transition trigger: crates/demo/src/renamed.rs'

new_multilang_fixture
commit_fixture base
mv "$test_root/repo/editors/vscode/src/messages.generated.js" \
  "$test_root/repo/editors/vscode/src/renamed-generated.js"
printf '%s\n' \
  "| \`editors/vscode/src/renamed-generated.js\` | 501 | production | editor-runtime | cohesive | generated-to-handwritten transition fixture |" \
  >> "$test_root/repo/docs/maintainability-baseline.md"
commit_fixture rename-generated-to-handwritten
expect_fail_contains "generated-to-handwritten rename is newly governed" \
  'policy transition trigger: editors/vscode/src/renamed-generated.js'

echo "cross-language maintainability fixtures passed"
