#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d)"
cleanup() {
  rm -rf "$test_root"
}
trap cleanup EXIT

mkdir -p "$test_root/repo/crates/demo/src" \
  "$test_root/repo/crates/demo/benches" \
  "$test_root/repo/tests" \
  "$test_root/repo/scripts"
cp "$repo_root/scripts/check-lint-suppressions.sh" "$test_root/repo/scripts/"
cp "$repo_root/scripts/check-lint-suppressions.py" "$test_root/repo/scripts/"
chmod +x "$test_root/repo/scripts/check-lint-suppressions.sh"

git -C "$test_root/repo" init -q -b main
git -C "$test_root/repo" config user.name Fixture
git -C "$test_root/repo" config user.email fixture@example.invalid
git -C "$test_root/repo" config commit.gpgsign false

cat > "$test_root/repo/crates/demo/src/lib.rs" <<'EOF'
// These are data, not attributes. The lexer must not report either one.
const COMMENTED: &str = "#[allow(clippy::unwrap_used)]";
const RAW: &str = r##"#[expect(dead_code)]"##;
// #[allow(dead_code)]

#[allow(
    dead_code,
    reason = "baseline helper remains part of the fixture"
)]
fn baseline_helper() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m initial
initial_sha="$(git -C "$test_root/repo" rev-parse HEAD)"

check_passes() {
  local base="$1" head="$2" output
  output="$(cd "$test_root/repo" && scripts/check-lint-suppressions.sh "$base" "$head" 2>&1)"
  if [[ "$output" != *"lint suppression policy passed"* ]]; then
    echo "lint suppression policy fixture unexpectedly failed" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
}

check_fails() {
  local base="$1" head="$2" expected="$3" output result
  set +e
  output="$(cd "$test_root/repo" && scripts/check-lint-suppressions.sh "$base" "$head" 2>&1)"
  result=$?
  set -e
  if (( result == 0 )); then
    echo "lint suppression policy fixture unexpectedly passed" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  if [[ "$output" != *"$expected"* ]]; then
    echo "lint suppression policy fixture missed expected diagnostic: $expected" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
}

# A moved existing attribute is baseline, while comments and strings remain
# invisible to the source lexer.
sed -i '1i\// unrelated movement before the baseline attribute' \
  "$test_root/repo/crates/demo/src/lib.rs"
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m move-baseline
moved_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_passes "$initial_sha" "$moved_sha"

# Exercise multiline attributes, multiple lints, a test exception, an FFI
# boundary, a compatibility boundary, and a benchmark exception together.
cat > "$test_root/repo/crates/demo/src/new_items.rs" <<'EOF'
#[allow(
    unused_variables,
    clippy::needless_borrow,
    reason = "the fixture intentionally demonstrates a narrow item boundary"
)]
fn production_fixture() {}

#[expect(dead_code, reason = "the fixture checks expect reason syntax")]
fn expected_fixture() {}
EOF
cat > "$test_root/repo/tests/support.rs" <<'EOF'
#[allow(dead_code)]
fn test_only_helper() {}
EOF
cat > "$test_root/repo/crates/demo/src/ffi_bridge.rs" <<'EOF'
#[allow(dead_code, reason = "ffi: preserve the C bridge symbol shape")]
pub fn bridge_symbol() {}
EOF
cat > "$test_root/repo/crates/demo/src/compatibility.rs" <<'EOF'
#[allow(dead_code, reason = "compatibility: retain the v0 public symbol")]
pub fn old_symbol() {}
EOF
cat > "$test_root/repo/crates/demo/benches/fixture.rs" <<'EOF'
#[allow(dead_code)]
fn benchmark_helper() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m valid-suppressions
valid_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_passes "$moved_sha" "$valid_sha"

# A second identical lint on another item is still new debt; matching uses the
# lexical target as well as the lint list so it cannot consume an unrelated
# baseline attribute.
cat > "$test_root/repo/crates/demo/src/duplicate.rs" <<'EOF'
#[allow(dead_code)]
fn unrelated_item() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m duplicate-baseline-lint
duplicate_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$valid_sha" "$duplicate_sha" "non-empty literal reason"

git -C "$test_root/repo" rm -q crates/demo/src/duplicate.rs
git -C "$test_root/repo" commit -q -m remove-duplicate-lint
clean_before_exceptions="$(git -C "$test_root/repo" rev-parse HEAD)"

# Exceptional production categories still need a rationale scoped to the
# boundary; a plain suppression does not become acceptable merely by moving
# into an FFI or compatibility-named file.
cat > "$test_root/repo/crates/demo/src/ffi_bad.rs" <<'EOF'
#[allow(dead_code)]
pub fn unowned_bridge_symbol() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m missing-ffi-scope
ffi_bad_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$clean_before_exceptions" "$ffi_bad_sha" "FFI-boundary suppressions must carry"

git -C "$test_root/repo" rm -q crates/demo/src/ffi_bad.rs
cat > "$test_root/repo/crates/demo/src/compatibility_bad.rs" <<'EOF'
#[allow(dead_code)]
pub fn unowned_compatibility_symbol() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m missing-compatibility-scope
compatibility_bad_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$ffi_bad_sha" "$compatibility_bad_sha" "public compatibility suppressions must carry"

git -C "$test_root/repo" rm -q crates/demo/src/compatibility_bad.rs
git -C "$test_root/repo" commit -q -m remove-unscoped-exceptions
clean_sha="$(git -C "$test_root/repo" rev-parse HEAD)"

# Crate- and module-wide production allows are rejected even when they have a
# reason: the scope itself is the quality failure.
cat > "$test_root/repo/crates/demo/src/broad.rs" <<'EOF'
#![allow(dead_code, reason = "broad production allow must be rejected")]

#[allow(unused_variables, reason = "module-wide production allow must be rejected")]
pub mod broad_module {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m broad-production-allow
broad_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$clean_sha" "$broad_sha" "crate/module-wide allows are not permitted"

git -C "$test_root/repo" rm -q crates/demo/src/broad.rs
cat > "$test_root/repo/crates/demo/src/no_reason.rs" <<'EOF'
#[allow(clippy::too_many_arguments)]
fn no_reason() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m missing-reason
no_reason_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$broad_sha" "$no_reason_sha" "non-empty literal reason"

# An existing one-lint suppression expanded to two lints is new policy debt,
# even though the attribute remains on the same function.
git -C "$test_root/repo" rm -q crates/demo/src/no_reason.rs
sed -i 's/^    dead_code,$/    dead_code,\n    unused_variables,/' \
  "$test_root/repo/crates/demo/src/lib.rs"
sed -i '/reason = "baseline helper remains part of the fixture"/d' \
  "$test_root/repo/crates/demo/src/lib.rs"
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m expanded-suppression
expanded_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$no_reason_sha" "$expanded_sha" "non-empty literal reason"

# A full inventory is intentionally reporting-only and must include existing
# debt without turning the first adoption into a blanket failure.
full_output="$(cd "$test_root/repo" && scripts/check-lint-suppressions.sh --full)"
if [[ "$full_output" != *"full inventory mode is reporting-only"* \
  || "$full_output" != *"crates/demo/src/lib.rs"* ]]; then
  echo "full lint suppression inventory fixture failed" >&2
  printf '%s\n' "$full_output" >&2
  exit 1
fi

echo "lint suppression policy hostile fixtures passed"
