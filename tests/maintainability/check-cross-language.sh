#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT
mkdir -p "$test_root/scripts/maintainability" "$test_root/editors/vscode/src" "$test_root/tests"
cp "$repo_root/scripts/check-maintainability.sh" "$test_root/scripts/"
cp "$repo_root/scripts/maintainability"/{*.sh,*.py} "$test_root/scripts/maintainability/"
printf '# No exceptions required in this fixture.\nexceptions = []\n' > "$test_root/scripts/maintainability/exceptions.toml"
git -C "$test_root" init -q -b main
git -C "$test_root" config user.name Fixture
git -C "$test_root" config user.email fixture@example.invalid
git -C "$test_root" config commit.gpgsign false
lines() { mkdir -p "$(dirname "$test_root/$1")"; awk -v n="$2" 'BEGIN {for (i=0;i<n;i++) print "// fixture"}' > "$test_root/$1"; }
save() { git -C "$test_root" add .; git -C "$test_root" commit -qm fixture; }
check() { (cd "$test_root" && scripts/check-maintainability.sh HEAD^ HEAD); }
for ext in js mjs cjs lua py sh; do
  lines "editors/vscode/src/large.$ext" 401
  lines "scripts/large.$ext" 401
  lines "tests/large.$ext" 501
done
lines editors/vscode/src/messages.generated.js 501
save
for ext in js mjs cjs lua py sh; do
  lines "editors/vscode/src/large.$ext" 402
  save
  if check >/dev/null 2>&1; then echo "unexpected pass for $ext" >&2; exit 1; fi
  lines "editors/vscode/src/large.$ext" 401
  save
done
lines editors/vscode/src/messages.generated.js 502
save
check >/dev/null
mv "$test_root/scripts/large.py" "$test_root/scripts/renamed.py"
save
check >/dev/null
mv "$test_root/editors/vscode/src/messages.generated.js" "$test_root/editors/vscode/src/renamed.js"
save
if check >/dev/null 2>&1; then echo 'generated-to-handwritten rename unexpectedly passed' >&2; exit 1; fi
ln -s ../../../scripts/large.sh "$test_root/editors/vscode/src/linked.sh"
save
if (cd "$test_root" && scripts/check-maintainability.sh --full) >/dev/null 2>&1; then echo 'symlink unexpectedly passed' >&2; exit 1; fi
echo 'cross-language maintainability fixtures passed'
