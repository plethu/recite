#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-test-organization.sh [repo-root]

Checks Recite Rust test layout conventions:
  - production source may declare `#[cfg(test)] mod tests;` only;
  - private source-side tests must live in module-local `tests.rs` sidecars;
  - source-side `*_test.rs` and `*_tests.rs` files are not allowed;
  - crate behavior tests should live under `crates/<crate>/tests/`.

The check reads tracked files and unignored local additions from git so ignored
build output does not affect PR gates.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi

repo_root="${1:-}"
if [[ -z "$repo_root" ]]; then
  repo_root="$(git rev-parse --show-toplevel)"
fi

if [[ ! -d "$repo_root/.git" ]]; then
  echo "repo root is not a git checkout: $repo_root" >&2
  exit 2
fi

failures=0

fail() {
  echo "$*" >&2
  failures=$((failures + 1))
}

while IFS= read -r file; do
  [[ -f "$repo_root/$file" ]] || continue

  basename="${file##*/}"
  if [[ "$basename" == *_test.rs || ( "$basename" == *_tests.rs && "$basename" != "tests.rs" ) ]]; then
    fail "${file}: source-side test files must be named tests.rs"
  fi

  if [[ "$basename" != "tests.rs" ]]; then
    while IFS= read -r match; do
      fail "${match}: #[test] belongs in tests.rs or crates/<crate>/tests/"
    done < <(grep -n '#\[test\]' "$repo_root/$file" || true)
  fi

  while IFS= read -r match; do
    fail "${match}: inline mod tests blocks are not allowed; use #[cfg(test)] mod tests;"
  done < <(grep -n '^[[:space:]]*mod tests[[:space:]]*{' "$repo_root/$file" || true)

  while IFS= read -r match; do
    fail "$match"
  done < <(
    awk '
      /^[[:space:]]*#\[cfg\(test\)\][[:space:]]*$/ {
        line = NR
        if ((getline next_line) <= 0 || next_line !~ /^[[:space:]]*mod tests;[[:space:]]*$/) {
          printf "%s:%d: #[cfg(test)] is only allowed before `mod tests;`\n", FILENAME, line
        }
      }
    ' "$repo_root/$file"
  )
done < <(
  git -C "$repo_root" ls-files --cached --others --exclude-standard -- \
    'crates/*/src/*.rs' \
    'crates/*/src/**/*.rs'
)

if (( failures > 0 )); then
  echo
  echo "Found ${failures} Recite test organization violation(s)." >&2
  exit 1
fi

echo "Recite test organization check passed."
