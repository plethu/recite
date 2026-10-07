#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$repo_root/scripts/maintainability/paths.sh"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT
fixture="$temporary/repo"
mkdir -p "$fixture"
git -C "$fixture" init -q -b main
git -C "$fixture" config user.name Fixture
git -C "$fixture" config user.email fixture@example.invalid
git -C "$fixture" config commit.gpgsign false
cat >"$fixture/dprint.json" <<'JSON'
{"plugins":["https://plugins.dprint.dev/typescript-0.96.1.wasm"]}
JSON
printf 'export const values=[1,2,3];\n' >"$fixture/demo.js"
git -C "$fixture" add .
git -C "$fixture" commit -qm base
base="$(git -C "$fixture" rev-parse HEAD)"
mise -E quality exec -- dprint fmt --config "$fixture/dprint.json" "$fixture/demo.js"
git -C "$fixture" add .
git -C "$fixture" commit -qm formatted
head="$(git -C "$fixture" rev-parse HEAD)"
maintainability_is_format_only "$fixture" "$base" "$head" demo.js demo.js

printf '\nexport const addedCode = true;\n' >>"$fixture/demo.js"
git -C "$fixture" add .
git -C "$fixture" commit -qm substantive
head="$(git -C "$fixture" rev-parse HEAD)"
if maintainability_is_format_only "$fixture" "$base" "$head" demo.js demo.js; then
  echo 'formatter replay accepted substantive code' >&2
  exit 1
fi
if maintainability_is_format_only "$fixture" "$base" "$head" missing.js demo.js; then
  echo 'formatter replay accepted a new file' >&2
  exit 1
fi
rm "$fixture/dprint.json"
if maintainability_is_format_only "$fixture" "$base" "$head" demo.js demo.js; then
  echo 'formatter replay accepted missing configuration' >&2
  exit 1
fi
echo 'Formatter replay preserves substantive size enforcement.'
