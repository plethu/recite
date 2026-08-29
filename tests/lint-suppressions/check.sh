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
  "$test_root/repo/fixtures" \
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

zero_sha=0000000000000000000000000000000000000000
check_passes "$zero_sha" "$initial_sha"

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

# cfg_attr can contain more than one nested suppression. They are checked as
# item-scoped attributes rather than disappearing inside the outer attribute.
cat > "$test_root/repo/crates/demo/src/cfg_attr.rs" <<'EOF'
#[cfg_attr(
    feature = "fixture",
    allow(dead_code, reason = "conditional fixture helper"),
    expect(unused_variables, reason = "conditional fixture expectation")
)]
fn conditional_fixture() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m cfg-attr-suppressions
cfg_attr_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_passes "$valid_sha" "$cfg_attr_sha"

cat > "$test_root/repo/crates/demo/src/cfg_attr_bad.rs" <<'EOF'
#[cfg_attr(test, allow(clippy::unwrap_used))]
fn missing_nested_reason() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m cfg-attr-missing-reason
cfg_attr_bad_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$cfg_attr_sha" "$cfg_attr_bad_sha" "non-empty literal reason"
git -C "$test_root/repo" rm -q crates/demo/src/cfg_attr_bad.rs
git -C "$test_root/repo" commit -q -m remove-cfg-attr-debt
clean_before_move="$(git -C "$test_root/repo" rev-parse HEAD)"

# A rename into production is new at its destination. The old test/support
# baseline must not be consumed across either path or category.
git -C "$test_root/repo" mv tests/support.rs crates/demo/src/moved.rs
git -C "$test_root/repo" commit -q -m move-test-suppression-into-production
moved_path_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$clean_before_move" "$moved_path_sha" "non-empty literal reason"
git -C "$test_root/repo" mv crates/demo/src/moved.rs tests/support.rs
git -C "$test_root/repo" commit -q -m restore-test-suppression
clean_before_duplicate="$(git -C "$test_root/repo" rev-parse HEAD)"

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
check_fails "$clean_before_duplicate" "$duplicate_sha" "non-empty literal reason"

git -C "$test_root/repo" rm -q crates/demo/src/duplicate.rs
git -C "$test_root/repo" commit -q -m remove-duplicate-lint
clean_before_exceptions="$(git -C "$test_root/repo" rev-parse HEAD)"

# Owner identity includes nested module, impl, and trait ancestry. Reusing an
# unreasoned `fn:duplicate` in a sibling owner must be a new suppression.
cat > "$test_root/repo/crates/demo/src/nested_owners.rs" <<'EOF'
mod parent {
    mod alpha {
        #[allow(dead_code)]
        fn duplicate() {}
    }
    mod beta {
        fn duplicate() {}
    }
}
impl FirstType {
    #[allow(dead_code)]
    fn duplicate() {}
}
impl SecondType {
    fn duplicate() {}
}
trait FirstTrait {
    #[allow(dead_code)]
    fn duplicate(&self);
}
trait SecondTrait {
    fn duplicate(&self);
}
impl Foo for Bar {
    #[allow(dead_code)]
    fn collision() {}
}
impl Foo_for_Bar {
    fn collision() {}
}
trait Foo: Bar {
    #[allow(dead_code)]
    fn collision(&self);
}
trait Foo_Bar {
    fn collision(&self);
}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m add-qualified-owner-baselines
qualified_owner_base_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
cat > "$test_root/repo/crates/demo/src/nested_owners.rs" <<'EOF'
mod parent {
    mod alpha {
        fn duplicate() {}
    }
    mod beta {
        #[allow(dead_code)]
        fn duplicate() {}
    }
}
impl FirstType {
    fn duplicate() {}
}
impl SecondType {
    #[allow(dead_code)]
    fn duplicate() {}
}
trait FirstTrait {
    fn duplicate(&self);
}
trait SecondTrait {
    #[allow(dead_code)]
    fn duplicate(&self);
}
impl Foo for Bar {
    fn collision() {}
}
impl Foo_for_Bar {
    #[allow(dead_code)]
    fn collision() {}
}
trait Foo: Bar {
    fn collision(&self);
}
trait Foo_Bar {
    #[allow(dead_code)]
    fn collision(&self);
}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m reject-unqualified-owner-reuse
qualified_owner_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$qualified_owner_base_sha" "$qualified_owner_sha" \
  "owner=mod:[6:parent]::mod:[4:beta]::fn:duplicate"
check_fails "$qualified_owner_base_sha" "$qualified_owner_sha" \
  "owner=impl:[10:SecondType]::fn:duplicate"
check_fails "$qualified_owner_base_sha" "$qualified_owner_sha" \
  "owner=trait:[11:SecondTrait]::fn:duplicate"
check_fails "$qualified_owner_base_sha" "$qualified_owner_sha" \
  "owner=impl:[11:Foo_for_Bar]::fn:collision"
check_fails "$qualified_owner_base_sha" "$qualified_owner_sha" \
  "owner=trait:[7:Foo_Bar]::fn:collision"
git -C "$test_root/repo" rm -q crates/demo/src/nested_owners.rs
git -C "$test_root/repo" commit -q -m remove-qualified-owner-fixture
clean_before_exceptions="$(git -C "$test_root/repo" rev-parse HEAD)"

# Declaration identity must remain complete through impl headers, use trees,
# function modifiers, generics, and unknown/bare item fallbacks.
cat > "$test_root/repo/crates/demo/src/ambiguous_owners.rs" <<'EOF'
impl Foo for Bar {
    #[allow(dead_code)]
    fn moved_impl() {}
}
impl Foo for Baz {
    fn moved_impl() {}
}
impl<T> Foo<T> for Bar<T> {
    #[allow(dead_code)]
    fn moved_generic() {}
}
impl<T> Foo<T> for Baz<T> {
    fn moved_generic() {}
}
#[allow(dead_code)]
use crate::old::Thing;
use crate::new::Thing;
#[allow(dead_code)]
pub extern "C" fn old_name() {}
pub extern "C" fn new_name() {}
#[allow(dead_code)]
{}
{}
#[allow(dead_code)]
old_macro!(fn nested);
new_macro!(fn nested);
fn outer() -> impl Foo {
    #[allow(dead_code)]
    fn moved() {}
}
impl Foo {
    fn moved() {}
}
#[allow(dead_code)]
fn r#old() {}
fn r#new() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m add-ambiguous-owner-baselines
ambiguous_owner_base_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
cat > "$test_root/repo/crates/demo/src/ambiguous_owners.rs" <<'EOF'
impl Foo for Bar {
    fn moved_impl() {}
}
impl Foo for Baz {
    #[allow(dead_code)]
    fn moved_impl() {}
}
impl<T> Foo<T> for Bar<T> {
    fn moved_generic() {}
}
impl<T> Foo<T> for Baz<T> {
    #[allow(dead_code)]
    fn moved_generic() {}
}
use crate::old::Thing;
#[allow(dead_code)]
use crate::new::Thing;
pub extern "C" fn old_name() {}
#[allow(dead_code)]
pub extern "C" fn new_name() {}
{}
#[allow(dead_code)]
{}
old_macro!(fn nested);
#[allow(dead_code)]
new_macro!(fn nested);
fn outer() -> impl Foo {
    fn moved() {}
}
impl Foo {
    #[allow(dead_code)]
    fn moved() {}
}
fn r#old() {}
#[allow(dead_code)]
fn r#new() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m reject-ambiguous-owner-reuse
ambiguous_owner_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$ambiguous_owner_base_sha" "$ambiguous_owner_sha" \
  "owner=impl:[3:Foo,3:for,3:Baz]::fn:moved_impl"
check_fails "$ambiguous_owner_base_sha" "$ambiguous_owner_sha" \
  "owner=impl:[1:<,1:T,1:>,3:Foo"
check_fails "$ambiguous_owner_base_sha" "$ambiguous_owner_sha" \
  "owner=use:[5:crate"
check_fails "$ambiguous_owner_base_sha" "$ambiguous_owner_sha" \
  "owner=fn:new_name"
check_fails "$ambiguous_owner_base_sha" "$ambiguous_owner_sha" \
  "owner=unstable"
check_fails "$ambiguous_owner_base_sha" "$ambiguous_owner_sha" \
  "owner=impl:[3:Foo]::fn:moved"
check_fails "$ambiguous_owner_base_sha" "$ambiguous_owner_sha" \
  "owner=fn:r#new"
git -C "$test_root/repo" rm -q crates/demo/src/ambiguous_owners.rs
git -C "$test_root/repo" commit -q -m remove-ambiguous-owner-fixture
clean_before_exceptions="$(git -C "$test_root/repo" rev-parse HEAD)"

# Matching candidates are partitioned by path and category, and lint-list
# changes retain the lexical owner. These hostile pairs must remain `new`
# rather than laundering through a broad/module or test baseline elsewhere.
cat > "$test_root/repo/crates/demo/src/cross_file_baseline.rs" <<'EOF'
#[allow(dead_code)]
mod broad_baseline_owner {}
EOF
cat > "$test_root/repo/tests/cross_category_baseline.rs" <<'EOF'
#[allow(dead_code)]
mod test_baseline_owner {}
EOF
cat > "$test_root/repo/crates/demo/src/target_baseline.rs" <<'EOF'
#[allow(dead_code)]
fn baseline_owner() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m add-matching-baselines
matching_baselines_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
cat > "$test_root/repo/crates/demo/src/cross_file_new.rs" <<'EOF'
#[allow(dead_code)]
fn cross_file_new() {}
EOF
cat > "$test_root/repo/crates/demo/src/cross_category_new.rs" <<'EOF'
#[allow(dead_code)]
fn cross_category_new() {}
EOF
sed -i 's/baseline_owner/new_owner/' "$test_root/repo/crates/demo/src/target_baseline.rs"
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m reject-baseline-laundering
laundering_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$matching_baselines_sha" "$laundering_sha" \
  "crates/demo/src/cross_file_new.rs:1: new allow(dead_code)"
check_fails "$matching_baselines_sha" "$laundering_sha" \
  "crates/demo/src/cross_category_new.rs:1: new allow(dead_code)"
check_fails "$matching_baselines_sha" "$laundering_sha" \
  "crates/demo/src/target_baseline.rs:1: new allow(dead_code)"
git -C "$test_root/repo" rm -q crates/demo/src/cross_file_baseline.rs \
  crates/demo/src/cross_category_new.rs tests/cross_category_baseline.rs \
  crates/demo/src/cross_file_new.rs crates/demo/src/target_baseline.rs
git -C "$test_root/repo" commit -q -m remove-baseline-laundering-fixtures
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

#[expect(clippy::unwrap_used, reason = "broad production expect must be rejected")]
pub mod broad_expect {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m broad-production-allow
broad_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$clean_sha" "$broad_sha" "crate/module-wide suppressions are not permitted"

git -C "$test_root/repo" rm -q crates/demo/src/broad.rs
git -C "$test_root/repo" commit -q -m remove-broad-debt
clean_after_broad="$(git -C "$test_root/repo" rev-parse HEAD)"

# A changed reason is still validated against the current category. It cannot
# turn into a whitespace production rationale or an unscoped FFI rationale.
cat > "$test_root/repo/crates/demo/src/reason_changed.rs" <<'EOF'
#[allow(dead_code, reason = "keep this production helper narrow")]
fn reason_changed_helper() {}
EOF
cat > "$test_root/repo/crates/demo/src/ffi_reason_changed.rs" <<'EOF'
#[allow(dead_code, reason = "ffi: preserve the bridge boundary")]
pub fn reason_changed_bridge() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m add-reason-baselines
reason_base_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
sed -i 's/reason = "keep this production helper narrow"/reason = "   "/' \
  "$test_root/repo/crates/demo/src/reason_changed.rs"
sed -i 's/reason = "ffi: preserve the bridge boundary"/reason = "bridge boundary"/' \
  "$test_root/repo/crates/demo/src/ffi_reason_changed.rs"
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m reject-invalid-reason-change
invalid_reason_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$reason_base_sha" "$invalid_reason_sha" \
  "new production suppressions must carry a non-empty literal reason"
check_fails "$reason_base_sha" "$invalid_reason_sha" \
  "FFI-boundary suppressions must carry"
git -C "$test_root/repo" rm -q crates/demo/src/reason_changed.rs \
  crates/demo/src/ffi_reason_changed.rs
git -C "$test_root/repo" commit -q -m remove-invalid-reason-fixtures
clean_after_reason="$(git -C "$test_root/repo" rev-parse HEAD)"

cat > "$test_root/repo/crates/demo/src/generated.rs" <<'EOF'
// A generated-looking name is not an exemption.
#[allow(dead_code)]
fn fake_generated_name() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m fake-generated-name
fake_generated_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$clean_after_reason" "$fake_generated_sha" "non-empty literal reason"

git -C "$test_root/repo" rm -q crates/demo/src/generated.rs
cat > "$test_root/repo/fixtures/generated.rs" <<'EOF'
#[allow(dead_code)]
fn allowlisted_generated_fixture() {}
EOF
printf '%s\n' 'fixtures/generated.rs' > "$test_root/repo/scripts/generated-rust-allowlist.txt"
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m explicit-generated-allowlist
allowlisted_generated_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_passes "$fake_generated_sha" "$allowlisted_generated_sha"

# A malformed repository-owned allowlist fails closed during setup instead of
# producing a traceback or silently treating the entry as policy.
printf '%s\n' '../escape.rs' > "$test_root/repo/scripts/generated-rust-allowlist.txt"
git -C "$test_root/repo" add scripts/generated-rust-allowlist.txt
git -C "$test_root/repo" commit -q -m malformed-generated-allowlist
malformed_allowlist_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$allowlisted_generated_sha" "$malformed_allowlist_sha" \
  "invalid generated Rust allowlist"

git -C "$test_root/repo" rm -q fixtures/generated.rs scripts/generated-rust-allowlist.txt
git -C "$test_root/repo" commit -q -m remove-generated-fixture
clean_sha="$(git -C "$test_root/repo" rev-parse HEAD)"

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

for _ in $(seq 1 200); do
  printf '#[allow(\n' >> "$test_root/repo/crates/demo/src/malformed.rs"
done
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m malformed-suppression-stress
malformed_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$expanded_sha" "$malformed_sha" "malformed Rust attribute"
git -C "$test_root/repo" rm -q crates/demo/src/malformed.rs
git -C "$test_root/repo" commit -q -m remove-malformed-fixture

# Missing Git objects are setup failures, not silently treated as absent files.
check_fails "$expanded_sha" deadbeef "lint suppression policy setup failed"

# The project gate accepts a repository-root argument even when launched from
# outside that repository; stub the unrelated cargo/adapter work here so this
# black-box check exercises only the path handoff.
cp "$repo_root/scripts/check-project-gates.sh" "$test_root/repo/scripts/"
for gate in check-test-organization.sh generate-ffi-header.sh check-ffi-header.sh check-unity-adapter.sh; do
  printf '%s\n' '#!/usr/bin/env bash' 'exit 0' > "$test_root/repo/scripts/$gate"
  chmod +x "$test_root/repo/scripts/$gate"
done
mkdir -p "$test_root/bin"
printf '%s\n' '#!/usr/bin/env bash' 'exit 0' > "$test_root/bin/cargo"
chmod +x "$test_root/bin/cargo"
if ! (
  cd "$test_root"
  PATH="$test_root/bin:$PATH" RECITE_BASE_REF=HEAD RECITE_HEAD_REF=HEAD \
    bash "$test_root/repo/scripts/check-project-gates.sh" "$test_root/repo" >/dev/null
); then
  echo "project gate outside-root fixture failed" >&2
  exit 1
fi

# A full inventory is intentionally reporting-only and must include existing
# debt without turning the first adoption into a blanket failure.
full_output="$(cd "$test_root/repo" && scripts/check-lint-suppressions.sh --full)"
if [[ "$full_output" != *"full inventory mode is reporting-only"* \
  || "$full_output" != *"crates/demo/src/lib.rs"* \
  || "$full_output" != *"scope=item"* \
  || "$full_output" != *"owner=fn:baseline_helper"* \
  || "$full_output" != *"reason=null"* \
  || "$full_output" != *"owner=fn:production_fixture"* \
  || "$full_output" != *"rationale=present"* \
  || "$full_output" != *"baseline_status=current"* ]]; then
  echo "full lint suppression inventory fixture failed" >&2
  printf '%s\n' "$full_output" >&2
  exit 1
fi

echo "lint suppression policy hostile fixtures passed"
