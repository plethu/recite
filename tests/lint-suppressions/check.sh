#!/usr/bin/env bash
# This fixture targets the repository's Ubuntu CI runner (GNU sed).
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT
mkdir -p "$test_root/repo/crates/demo/src" "$test_root/repo/tests" \
  "$test_root/repo/fixtures" "$test_root/repo/scripts"
cp "$repo_root/scripts/check-lint-suppressions.sh" \
  "$repo_root/scripts/check-lint-suppressions.py" \
  "$repo_root/scripts/lint_suppression_ast.py" \
  "$repo_root/scripts/lint_suppression_meta.py" "$test_root/repo/scripts/"
chmod +x "$test_root/repo/scripts/check-lint-suppressions.sh"
git -C "$test_root/repo" init -q -b main
git -C "$test_root/repo" config user.name Fixture
git -C "$test_root/repo" config user.email fixture@example.invalid
git -C "$test_root/repo" config commit.gpgsign false
check_passes() {
  local base="$1" head="$2" output
  output="$(cd "$test_root/repo" && scripts/check-lint-suppressions.sh "$base" "$head" 2>&1)"
  [[ "$output" == *"lint suppression policy passed"* ]] || {
    echo "lint suppression fixture unexpectedly failed" >&2
    printf '%s\n' "$output" >&2
    exit 1
  }
}
check_fails() {
  local base="$1" head="$2" expected="$3" output result
  set +e
  output="$(cd "$test_root/repo" && scripts/check-lint-suppressions.sh "$base" "$head" 2>&1)"
  result=$?
  set -e
  if (( result == 0 )) || [[ "$output" != *"$expected"* ]]; then
    echo "lint suppression fixture missed expected diagnostic: $expected" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
}
cat > "$test_root/repo/crates/demo/src/lib.rs" <<'EOF'
const TEXT: &str = "#[allow(clippy::unwrap_used)] recite-lint-suppression: ffi";
const RAW: &str = r##"#[expect(dead_code)] recite-lint-suppression: compatibility"##;
// #[allow(dead_code)] recite-lint-suppression: ffi
#[allow(dead_code, reason = "stable function owner survives line movement")]
async fn named_function() {}
#[allow(dead_code, unused_variables, reason = "narrowing fixture")]
fn narrowable() {}
#[allow(dead_code, reason = "unicode function owner")]
fn café() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m initial
initial_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
zero_sha=0000000000000000000000000000000000000000
check_passes "$zero_sha" "$initial_sha"
# Named functions, async modifiers, Unicode identifiers, cfg predicates, and
# a safe lint-list narrowing all remain matchable when their lines move.
sed -i '1i// unrelated movement' "$test_root/repo/crates/demo/src/lib.rs"
sed -i 's/dead_code, unused_variables, reason/dead_code, reason/' \
  "$test_root/repo/crates/demo/src/lib.rs"
cat > "$test_root/repo/crates/demo/src/cfg.rs" <<'EOF'
#[cfg_attr(any(feature = "one", feature = "two"),
    allow(dead_code, reason = "cfg predicate belongs to the named helper"),
    expect(unused_variables, reason = "cfg sibling remains explicit"))]
#[cfg(feature = "one")]
fn cfg_helper() {}

#[cfg(unix)]
#[allow(dead_code, reason = "unix configured sibling remains explicit")]
fn configured_sibling() {}

#[cfg(windows)]
#[allow(dead_code, reason = "windows configured sibling remains explicit")]
fn configured_sibling() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m move-and-narrow
moved_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_passes "$initial_sha" "$moved_sha"
sed -i 's/feature = "one"/feature = "three"/' "$test_root/repo/crates/demo/src/cfg.rs"
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m cfg-predicate-change
cfg_changed_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_passes "$moved_sha" "$cfg_changed_sha"
cfg_output="$(cd "$test_root/repo" && scripts/check-lint-suppressions.sh "$moved_sha" "$cfg_changed_sha")"
[[ "$cfg_output" == *"cfg_helper"* && "$cfg_output" == *"owner_stable=false"* ]] || {
  echo "configured suppression sibling was treated as a stable owner" >&2
  printf '%s\n' "$cfg_output" >&2
  exit 1
}
# Anonymous and identical declarations cannot consume a baseline, even when
# the source text is identical. The current record is deliberately unreasoned
# so fail-closed ownership is observable.
cat > "$test_root/repo/crates/demo/src/anonymous.rs" <<'EOF'
#[allow(dead_code, reason = "anonymous baseline")]
const _: i32 = 0;
const _: i32 = 0;
#[allow(dead_code, reason = "anonymous use baseline")]
use crate::old::Thing as _;
use crate::new::Thing as _;
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m anonymous-baseline
anonymous_base="$(git -C "$test_root/repo" rev-parse HEAD)"
sed -i 's/, reason = "anonymous baseline"//' \
  "$test_root/repo/crates/demo/src/anonymous.rs"
sed -i 's/, reason = "anonymous use baseline"//' \
  "$test_root/repo/crates/demo/src/anonymous.rs"
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m anonymous-swap
anonymous_head="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$anonymous_base" "$anonymous_head" "owner=unstable"
# Braced use trees and const-generic/array-const braces are parser structure,
# not declaration delimiters. A named use and impl survive unchanged.
cat > "$test_root/repo/crates/demo/src/structured.rs" <<'EOF'
#[allow(unused_imports, reason = "braced use tree is a single named item")]
use crate::{alpha::{Beta, Gamma}, delta as epsilon};
struct Generic<const N: usize> where [(); { N + 1 }]: {
    values: [u8; { N + 1 }],
}
#[allow(dead_code, reason = "const-generic implementation owner")]
impl<const N: usize> Generic<N> where [(); { N + 1 }]: {
    fn value(&self) -> [u8; { N + 1 }] { self.values }
}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m structured-rust
structured_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_passes "$anonymous_head" "$structured_sha"
# Closures and macro token trees are intentionally ambiguous;
# they are still inventoried and require a reason rather than borrowing an
# unrelated named baseline.
cat > "$test_root/repo/crates/demo/src/ambiguous.rs" <<'EOF'
fn host() {
    #[allow(dead_code, reason = "closure expression is a local exception")]
    let closure = || { 1 };
    let _ = closure;
}
macro_rules! generated {
    () => {
        #[allow(dead_code, reason = "macro body is an opaque token boundary")]
        fn generated_body() {}
    };
}
generated! {
    #[allow(dead_code, reason = "macro input is an opaque token boundary")]
    fn generated_input() {}
}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m ambiguous-scopes
ambiguous_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
set +e
ambiguous_output="$(cd "$test_root/repo" && scripts/check-lint-suppressions.sh "$structured_sha" "$ambiguous_sha" 2>&1)"
ambiguous_result=$?
set -e
(( ambiguous_result != 0 )) || {
  echo "macro token-tree suppressions were accepted" >&2
  printf '%s\n' "$ambiguous_output" >&2
  exit 1
}
[[ "$ambiguous_output" == *"ambiguous.rs:6: new opaque_macro(allow) scope=item owner=unstable"* \
  && "$ambiguous_output" == *"ambiguous.rs:12: new opaque_macro(allow) scope=item owner=unstable"* ]] || {
  echo "macro token-tree suppressions were not recorded exactly" >&2
  printf '%s\n' "$ambiguous_output" >&2
  exit 1
}
# Direct attributes, cfg_attr attributes, and comments between #[ and allow are
# parsed as ordinary attributes. Macro definitions and invocations remain raw
# opaque token trees and are represented by explicit unstable records.
cat > "$test_root/repo/crates/demo/src/opaque.rs" <<'EOF'
#[allow(dead_code, reason = "direct allow remains structured")]
fn direct_allow() {}
#[cfg_attr(unix, allow(dead_code, reason = "cfg_attr allow remains structured"))]
fn cfg_allow() {}
#[ /* comment before control */ allow(dead_code, reason = "commented control remains structured")]
fn commented_allow() {}
macro_rules! generated {
    () => {
        #[allow(dead_code)]
        fn generated_body() {}
    };
}
generated! {
    #[cfg_attr(unix, allow(dead_code))]
    fn generated_input() {}
}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m opaque-token-tree-records
opaque_base="$ambiguous_sha"
opaque_head="$(git -C "$test_root/repo" rev-parse HEAD)"
set +e
opaque_output="$(cd "$test_root/repo" && scripts/check-lint-suppressions.sh "$opaque_base" "$opaque_head" 2>&1)"
opaque_result=$?
set -e
(( opaque_result != 0 )) || {
  echo "opaque token-tree suppressions were accepted" >&2
  printf '%s\n' "$opaque_output" >&2
  exit 1
}
[[ "$opaque_output" == *"opaque.rs:1: new allow(dead_code) scope=item owner=fn:direct_allow category=production owner_stable=true"* \
  && "$opaque_output" == *"opaque.rs:3: new allow(dead_code) scope=item owner=fn:cfg_allow category=production owner_stable=false"* \
  && "$opaque_output" == *"opaque.rs:5: new allow(dead_code) scope=item owner=fn:commented_allow category=production owner_stable=true"* \
  && "$opaque_output" == *"opaque.rs:7: new opaque_macro(allow) scope=item owner=unstable category=production owner_stable=false reason=null"* \
  && "$opaque_output" == *"opaque.rs:13: new opaque_macro(allow,cfg_attr) scope=item owner=unstable category=production owner_stable=false reason=null"* ]] || {
  echo "opaque fixture records were not exact" >&2
  printf '%s\n' "$opaque_output" >&2
  exit 1
}
# Duplicate consumption is one-to-one: one baseline use cannot legitimize two
# current identical records. Cross-file moves are always new at the destination.
cat > "$test_root/repo/crates/demo/src/duplicates.rs" <<'EOF'
#[allow(unused_imports)]
use crate::duplicate::Thing;
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m duplicate-baseline
duplicate_base="$(git -C "$test_root/repo" rev-parse HEAD)"
sed -i '1i#[allow(unused_imports)]' "$test_root/repo/crates/demo/src/duplicates.rs"
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m duplicate-current
duplicate_head="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$duplicate_base" "$duplicate_head" "new allow(unused_imports)"
cat > "$test_root/repo/crates/demo/src/old.rs" <<'EOF'
#[allow(dead_code)]
fn moved_across_files() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m cross-file-baseline
cross_file_base="$(git -C "$test_root/repo" rev-parse HEAD)"
git -C "$test_root/repo" mv crates/demo/src/old.rs crates/demo/src/new.rs
git -C "$test_root/repo" commit -q -m cross-file-move
cross_file_head="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$cross_file_base" "$cross_file_head" \
  "crates/demo/src/new.rs:1: new allow(dead_code)"
# Adjacent markers are comment nodes, never substrings in a string or old
# comment. The path remains production and therefore needs the scoped prefix.
cat > "$test_root/repo/crates/demo/src/marked.rs" <<'EOF'
// recite-lint-suppression: compatibility
#[allow(dead_code, reason = "compatibility: retain the old symbol")]
fn old_symbol() {}
EOF
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m marker
marker_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_passes "$cross_file_head" "$marker_sha"
# New broad production scopes remain forbidden, while support and generated
# paths retain their explicit exceptions.
cat > "$test_root/repo/crates/demo/src/broad.rs" <<'EOF'
#![allow(dead_code, reason = "crate scope must be rejected")]
#[allow(unused_variables, reason = "module scope must be rejected")]
mod broad {}
EOF
cat > "$test_root/repo/tests/support.rs" <<'EOF'
#[allow(dead_code)]
fn support() {}
EOF
cat > "$test_root/repo/fixtures/generated.rs" <<'EOF'
#[allow(dead_code)]
fn generated() {}
EOF
printf '%s\n' fixtures/generated.rs > "$test_root/repo/scripts/generated-rust-allowlist.txt"
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m broad-and-exceptions
broad_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$marker_sha" "$broad_sha" "crate/module-wide suppressions are not permitted"
# Malformed attributes become structural parse failures, not silently ignored
# records. Full inventory remains reporting-only.
printf '%s\n' '#[allow(' > "$test_root/repo/crates/demo/src/malformed.rs"
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m malformed
malformed_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$broad_sha" "$malformed_sha" "rustfmt rejected Rust syntax"
git -C "$test_root/repo" rm -q crates/demo/src/malformed.rs
git -C "$test_root/repo" commit -q -m remove-malformed
# Rustfmt is the syntax authority, so incomplete declarations fail before
# ast-grep matching regardless of its recovery behaviour.
for malformed_source in \
  'fn f()' \
  'fn f( {}' \
  'struct S {' \
  'mod m {' \
  'const X: = 1;'; do
  printf '%s\n' '#[allow(dead_code)]' "$malformed_source" > \
    "$test_root/repo/crates/demo/src/malformed.rs"
  git -C "$test_root/repo" add .
  git -C "$test_root/repo" commit -q -m malformed-structure
  malformed_structure_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
  check_fails "$broad_sha" "$malformed_structure_sha" "rustfmt rejected Rust syntax"
  git -C "$test_root/repo" rm -q crates/demo/src/malformed.rs
  git -C "$test_root/repo" commit -q -m remove-malformed-structure
done
# Escaped whitespace is not treated as a visible rationale. The metadata
# interpreter rejects it instead of pretending to decode only some escapes.
printf '%s\n' '#[allow(dead_code, reason = "\n")]' 'fn escaped_reason() {}' > \
  "$test_root/repo/crates/demo/src/escaped.rs"
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m escaped-reason
escaped_sha="$(git -C "$test_root/repo" rev-parse HEAD)"
check_fails "$broad_sha" "$escaped_sha" "malformed allow suppression reason"
git -C "$test_root/repo" rm -q crates/demo/src/escaped.rs
git -C "$test_root/repo" commit -q -m remove-escaped-reason
# Full mode must be self-contained even when the caller supplied exact refs
# from the outer repository (as CI does). Both refs are overridden to this
# fixture's HEAD before the temporary Git checkout is inspected.
full_output="$(cd "$test_root/repo" && RECITE_BASE_REF=HEAD RECITE_HEAD_REF=HEAD scripts/check-lint-suppressions.sh --full)"
[[ "$full_output" == *"full inventory mode is reporting-only"* \
  && "$full_output" == *"owner=fn:named_function"* \
  && "$full_output" == *"owner=unstable"* ]] || {
  echo "full lint suppression inventory fixture failed" >&2
  printf '%s\n' "$full_output" >&2
  exit 1
}
echo "lint suppression structural fixtures passed"
