#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d)"
cleanup() {
  rm -rf "$test_root"
}
trap cleanup EXIT

mkdir -p "$test_root/repo/crates/demo/src" "$test_root/repo/scripts" "$test_root/repo/tools"
cp "$repo_root/scripts/check-ast-grep.sh" "$test_root/repo/scripts/check-ast-grep.sh"
cp -R "$repo_root/tools/ast-grep" "$test_root/repo/tools/"
chmod +x "$test_root/repo/scripts/check-ast-grep.sh"
printf '%s\n' 'fn production() {}' > "$test_root/repo/crates/demo/src/lib.rs"

git -C "$test_root/repo" init -q -b main
git -C "$test_root/repo" config user.name Fixture
git -C "$test_root/repo" config user.email fixture@example.invalid
git -C "$test_root/repo" config commit.gpgsign false
git -C "$test_root/repo" add .
git -C "$test_root/repo" commit -q -m initial

zero_sha="0000000000000000000000000000000000000000"
(
  cd "$test_root/repo"
  scripts/check-ast-grep.sh "$zero_sha" HEAD
)

echo "ast-grep zero-SHA fixture passed"

# This literal intentionally contains a four-equality-branch chain.
printf '%s\n' \
  'fn sprawling_classifier(value: usize) -> usize {' \
  '    if value == 0 {' \
  '        0' \
  '    } else if value == 1 {' \
  '        1' \
  '    } else if value == 2 {' \
  '        2' \
  '    } else if value == 3 {' \
  '        3' \
  '    } else {' \
  '        4' \
  '    }' \
  '}' \
  > "$test_root/repo/crates/demo/src/lib.rs"
git -C "$test_root/repo" add crates/demo/src/lib.rs
git -C "$test_root/repo" commit -q -m structural-rule

if (
  cd "$test_root/repo"
  scripts/check-ast-grep.sh HEAD^ HEAD
); then
  echo "ast-grep structural fixture unexpectedly passed" >&2
  exit 1
fi

echo "ast-grep structural positive fixture rejected"

sed -i '2i\    // ast-grep-ignore: rust-elseif-cascade (ordered protocol precedence)' \
  "$test_root/repo/crates/demo/src/lib.rs"
git -C "$test_root/repo" add crates/demo/src/lib.rs
git -C "$test_root/repo" commit -q -m structural-exemption

(
  cd "$test_root/repo"
  scripts/check-ast-grep.sh HEAD^ HEAD
)

echo "ast-grep structural exemption fixture passed"
