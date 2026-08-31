#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT

fixture_repo="$test_root/repo"
mkdir -p "$fixture_repo/docs" "$fixture_repo/fixtures/editor-parity" \
  "$fixture_repo/fixtures/recite/valid" "$fixture_repo/fixtures/recite/invalid" \
  "$fixture_repo/fixtures/schema/valid" "$fixture_repo/scripts"
cp "$repo_root/scripts/check-editor-parity.sh" "$fixture_repo/scripts/"
cp "$repo_root/docs/editor-parity-contract.md" "$fixture_repo/docs/"
cp "$repo_root/fixtures/editor-parity/contract.json" "$fixture_repo/fixtures/editor-parity/"
cp "$repo_root/fixtures/recite/valid/language_pressure.recite" "$fixture_repo/fixtures/recite/valid/"
cp "$repo_root/fixtures/recite/valid/core_language_spike.recite" "$fixture_repo/fixtures/recite/valid/"
cp "$repo_root/fixtures/recite/invalid/parser_marker_leading_prose.recite" "$fixture_repo/fixtures/recite/invalid/"
cp "$repo_root/fixtures/schema/valid/generated_manifest.json" "$fixture_repo/fixtures/schema/valid/"
cp "$repo_root/fixtures/schema/valid/full_manifest.json" "$fixture_repo/fixtures/schema/valid/"
chmod +x "$fixture_repo/scripts/check-editor-parity.sh"

git -C "$fixture_repo" init -q -b main
git -C "$fixture_repo" config user.name Fixture
git -C "$fixture_repo" config user.email fixture@example.invalid
git -C "$fixture_repo" config commit.gpgsign false
git -C "$fixture_repo" add .
git -C "$fixture_repo" commit -q -m initial

run_checker() {
  (cd "$fixture_repo" && scripts/check-editor-parity.sh)
}

run_checker
echo "editor parity baseline fixture passed"

mutate_fixture() {
  local mutation="$1"
  python3 - "$fixture_repo/fixtures/editor-parity/contract.json" "$mutation" <<'PY'
import json
import sys

path, mutation = sys.argv[1:]
with open(path, encoding="utf-8") as handle:
    contract = json.load(handle)

if mutation == "traversal":
    contract["artifacts"][0]["path"] = "../../outside/claimed.vsix"
elif mutation == "client":
    client = next(client for client in contract["clients"] if client["id"] == "vscode")
    client["status"] = "implemented"
    client["platform_status"] = {"linux": "implemented", "macos": "partial", "windows": "partial"}
elif mutation == "distribution":
    distribution = next(distribution for distribution in contract["distributions"] if distribution["id"] == "vs-marketplace")
    distribution["status"] = "implemented"
elif mutation == "capability-platform":
    capability = next(capability for capability in contract["capabilities"] if capability["id"] == "lsp.completion")
    capability["platform_status"]["linux"] = "implemented"
elif mutation == "capability-evidence":
    capability = next(capability for capability in contract["capabilities"] if capability["id"] == "lsp.completion")
    capability["expected_evidence"]["status"] = "implemented"
else:
    raise SystemExit(f"unknown mutation: {mutation}")

with open(path, "w", encoding="utf-8") as handle:
    json.dump(contract, handle, indent=2)
    handle.write("\n")
PY
}

expect_failure() {
  local mutation="$1"
  local expected="$2"
  local output result
  mutate_fixture "$mutation"
  set +e
  output="$(run_checker 2>&1)"
  result=$?
  set -e
  if (( result == 0 )) || [[ "$output" != *"$expected"* ]]; then
    echo "editor parity hostile fixture missed: $mutation" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  echo "editor parity hostile fixture rejected: $mutation"
  git -C "$fixture_repo" checkout -q -- fixtures/editor-parity/contract.json
}

expect_failure traversal "path escapes the repository"
expect_failure client "implemented client vscode needs an implemented artifact"
expect_failure distribution "implemented distribution vs-marketplace needs an implemented artifact"
expect_failure capability-platform "partial capability lsp.completion cannot claim linux platform status implemented"
expect_failure capability-evidence "partial capability lsp.completion cannot claim implemented evidence"
