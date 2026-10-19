#!/usr/bin/env bash
set -euo pipefail

source_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$source_root/scripts/maintainability/paths.sh"
# CI must not depend on formatter executables from an activated or personal PATH.
PATH="$(dirname "$(command -v mise)"):/usr/bin:/bin"
export PATH
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT
fixture="$temporary/repo"
mkdir -p "$fixture/.config/mise"
git -C "$fixture" init -q -b main
git -C "$fixture" config user.name Fixture
git -C "$fixture" config user.email fixture@example.invalid
git -C "$fixture" config commit.gpgsign false
cat >"$fixture/.config/dprint.json" <<'JSON'
{"plugins":["https://plugins.dprint.dev/typescript-0.96.1.wasm"]}
JSON
printf '%s\n' '{"name":"formatter-replay-fixture","private":true}' >"$fixture/package.json"
cp "$source_root/oxfmt.config.mts" "$fixture/"
ln -s "$source_root/node_modules" "$fixture/node_modules"
printf 'export const values=[1,2,3];\n' >"$fixture/demo.js"
git -C "$fixture" add .
git -C "$fixture" commit -qm base
base="$(git -C "$fixture" rev-parse HEAD)"
mise -C "$source_root" exec -- bash -c 'cd "$1" && exec bun "$2" demo.js' \
  _ "$fixture" "$source_root/node_modules/.bin/oxfmt"
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
mkdir "$fixture/nested"
cat >"$fixture/nested/oxfmt.config.mts" <<'TS'
import base from "../oxfmt.config.mts";
export default { ...base, singleQuote: true };
TS
printf 'export const quote="hello";\n' >"$fixture/nested/demo.ts"
git -C "$fixture" add .
git -C "$fixture" commit -qm nested-base
base="$(git -C "$fixture" rev-parse HEAD)"
mise -C "$source_root" exec -- bash -c 'cd "$1" && exec bun "$2" nested/demo.ts' \
  _ "$fixture" "$source_root/node_modules/.bin/oxfmt"
git -C "$fixture" add .
git -C "$fixture" commit -qm nested-formatted
head="$(git -C "$fixture" rev-parse HEAD)"
maintainability_is_format_only "$fixture" "$base" "$head" nested/demo.ts nested/demo.ts
rm "$fixture/.config/dprint.json" "$fixture/oxfmt.config.mts"
if maintainability_is_format_only "$fixture" "$base" "$head" demo.js demo.js; then
  echo 'formatter replay accepted missing configuration' >&2
  exit 1
fi
mise -E quality exec -- jq '.exec.commands |= map(select(.exts == ["lua"]))' \
  "$source_root/.config/dprint.json" >"$fixture/.config/dprint.json"
cp "$source_root/stylua.toml" "$fixture/"
cp "$source_root/.config/mise/config.quality.toml" "$fixture/.config/mise/"
printf 'local values={1,2,3}\n' >"$fixture/demo.lua"
git -C "$fixture" add .
git -C "$fixture" commit -qm lua-base
base="$(git -C "$fixture" rev-parse HEAD)"
mise -E quality exec -- bash -c 'cd "$1/nested" && exec dprint fmt ../demo.lua' _ "$fixture"
git -C "$fixture" add .
git -C "$fixture" commit -qm lua-formatted
head="$(git -C "$fixture" rev-parse HEAD)"
maintainability_is_format_only "$fixture" "$base" "$head" demo.lua demo.lua
printf '\nlocal addedCode = true\n' >>"$fixture/demo.lua"
git -C "$fixture" add .
git -C "$fixture" commit -qm lua-substantive
head="$(git -C "$fixture" rev-parse HEAD)"
if maintainability_is_format_only "$fixture" "$base" "$head" demo.lua demo.lua; then
  echo 'formatter replay accepted substantive Lua' >&2
  exit 1
fi
echo 'Formatter replay preserves substantive size enforcement.'
