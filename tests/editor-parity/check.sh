#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT

fixture_repo="$test_root/target/repo"
mkdir -p "$fixture_repo/docs" "$fixture_repo/fixtures/editor-parity" \
  "$fixture_repo/fixtures/recite/valid" "$fixture_repo/fixtures/recite/invalid" \
  "$fixture_repo/fixtures/schema/valid" "$fixture_repo/scripts" \
  "$fixture_repo/editors/recite-tree-sitter" \
  "$fixture_repo/editors/recite-neovim" \
  "$fixture_repo/crates/recite-lsp/tests" \
  "$fixture_repo/crates/recite-cli/tests" \
  "$fixture_repo/crates/recite-compiler/tests/authoring_build"
cp "$repo_root/scripts/check-editor-parity.sh" "$fixture_repo/scripts/"
cp "$repo_root/scripts/check-tree-sitter.sh" "$fixture_repo/scripts/"
cp "$repo_root/scripts/check-neovim.sh" "$fixture_repo/scripts/"
cp "$repo_root/scripts/check-vscode.sh" "$fixture_repo/scripts/"
cp -R "$repo_root/scripts/editor_parity" "$fixture_repo/scripts/"
cp "$repo_root/editors/recite-tree-sitter/grammar.js" "$fixture_repo/editors/recite-tree-sitter/"
cp -R "$repo_root/editors/recite-neovim/." "$fixture_repo/editors/recite-neovim/"
cp "$repo_root/docs/editor-parity-contract.md" "$fixture_repo/docs/"
cp "$repo_root/fixtures/editor-parity/contract.json" "$fixture_repo/fixtures/editor-parity/"
cp "$repo_root/fixtures/recite/valid/language_pressure.recite" "$fixture_repo/fixtures/recite/valid/"
cp "$repo_root/fixtures/recite/valid/locale_fallback_fr.po" "$fixture_repo/fixtures/recite/valid/"
cp "$repo_root/fixtures/recite/valid/core_language_spike.recite" "$fixture_repo/fixtures/recite/valid/"
cp "$repo_root/fixtures/recite/invalid/parser_marker_leading_prose.recite" "$fixture_repo/fixtures/recite/invalid/"
cp "$repo_root/fixtures/schema/valid/generated_manifest.json" "$fixture_repo/fixtures/schema/valid/"
cp "$repo_root/fixtures/schema/valid/full_manifest.json" "$fixture_repo/fixtures/schema/valid/"
cp -R "$repo_root/tests/editor-parity/cargo-fixture/." "$fixture_repo/"
chmod +x "$fixture_repo/scripts/check-editor-parity.sh" "$fixture_repo/scripts/check-tree-sitter.sh" "$fixture_repo/scripts/check-neovim.sh" "$fixture_repo/scripts/check-vscode.sh"

git -C "$fixture_repo" init -q -b main
git -C "$fixture_repo" config user.name Fixture
git -C "$fixture_repo" config user.email fixture@example.invalid
git -C "$fixture_repo" config commit.gpgsign false
git -C "$fixture_repo" add .
git -C "$fixture_repo" commit -q -m initial
(
  cd "$fixture_repo"
  cargo generate-lockfile --quiet
)
assert_no_hashed_targets() {
  if [[ -d "$fixture_repo/target/editor-parity" ]]; then
    echo "editor parity target isolation fixture failed: checker created a hash-cache directory" >&2
    find "$fixture_repo/target/editor-parity" -mindepth 1 -maxdepth 2 -print >&2
    exit 1
  fi
  if [[ ! -f "$fixture_repo/target/editor-parity.lock" ]]; then
    echo "editor parity target isolation fixture failed: fixed lock was not created" >&2
    exit 1
  fi
  if find "$fixture_repo/target" -mindepth 1 -type d -regextype posix-extended -regex '.*/[0-9a-f]{64}' -print -quit | grep -q .; then
    echo "editor parity target isolation fixture failed: hashed Cargo target directory remains" >&2
    exit 1
  fi
  echo "editor parity shared-target fixture passed"
}

run_checker() {
  (cd "$fixture_repo" && scripts/check-editor-parity.sh)
}

mutate_fixture() {
  local mutation="$1"
  python3 "$repo_root/tests/editor-parity/mutate_fixture.py" \
    "$fixture_repo/fixtures/editor-parity/contract.json" "$mutation"
}

assert_portable_lock_source() {
  python3 - "$repo_root/scripts/editor_parity/portable_lock.py" <<'PY'
import ast
import sys
from pathlib import Path

tree = ast.parse(Path(sys.argv[1]).read_text(encoding="utf-8"))

platform_import = next(
    node for node in tree.body
    if isinstance(node, ast.If)
    and isinstance(node.test, ast.Compare)
    and isinstance(node.test.left, ast.Attribute)
    and isinstance(node.test.left.value, ast.Name)
    and node.test.left.value.id == "os"
    and node.test.left.attr == "name"
    and len(node.test.ops) == 1
    and isinstance(node.test.ops[0], ast.Eq)
    and len(node.test.comparators) == 1
    and isinstance(node.test.comparators[0], ast.Constant)
    and node.test.comparators[0].value == "nt"
)

def imported_modules(nodes):
    return {
        alias.name
        for node in nodes
        if isinstance(node, ast.Import)
        for alias in node.names
    }

if "msvcrt" not in imported_modules(platform_import.body):
    raise SystemExit("Windows parity lock branch must import msvcrt")
if "fcntl" not in imported_modules(platform_import.orelse):
    raise SystemExit("POSIX parity lock branch must import fcntl")
if any(
    module == "fcntl"
    for node in tree.body
    if isinstance(node, ast.Import)
    for module in imported_modules([node])
):
    raise SystemExit("fcntl must not be imported unconditionally")

print("editor parity portable-lock source fixture passed")
PY
}

assert_portable_lock_source
python3 - "$fixture_repo/fixtures/editor-parity/contract.json" "$fixture_repo/docs/editor-parity-contract.md" <<'PY'
import json
import os
import sys
from pathlib import Path

contract_path, document_path = map(Path, sys.argv[1:])
contract = json.loads(contract_path.read_text(encoding="utf-8"))
clients = {client["id"]: client for client in contract["clients"]}
artifacts = {artifact["id"]: artifact for artifact in contract["artifacts"]}

for client_id in ("vscode", "vscodium"):
    client = clients[client_id]
    if client["status"] != "partial":
        raise SystemExit(f"{client_id} foundation must remain partial")
    if client["platform_status"] != {"linux": "partial", "macos": "planned", "windows": "planned"}:
        raise SystemExit(f"{client_id} foundation must claim Linux-only partial evidence")

artifact = artifacts["vscode-vsix"]
if artifact["status"] != "partial" or artifact["path"] is not None:
    raise SystemExit("VS Code artifact must remain a partial generated artifact, not checked-in archive")
if "package-checked" not in artifact["notes"] or "ignored build output" not in artifact["notes"]:
    raise SystemExit("VS Code artifact notes must distinguish package checks from checked-in output")
capabilities = {capability["id"]: capability for capability in contract["capabilities"]}
expected_client_evidence = {
    "editor.filetype.registration",
    "lsp.code-actions",
    "lsp.completion.navigation",
    "lsp.definition",
    "lsp.initialize.capabilities",
    "lsp.overlay.recovery",
    "lsp.publish.diagnostics",
    "lsp.utf16.positions",
    "workspace.configuration",
    "workspace.project.discovery",
}
actual_client_evidence = {
    capability_id
    for capability_id, capability in capabilities.items()
    if capability["client_status"]["vscode"] == "partial"
    and "scripts/check-vscode.sh" in capability["expected_evidence"].get("commands", [])
}
if actual_client_evidence != expected_client_evidence:
    raise SystemExit("VS Code partial client evidence rows drifted from the checked package/live surface")
for capability_id in expected_client_evidence:
    capability = capabilities[capability_id]
    if capability["follow_up"] != "#51":
        raise SystemExit(f"{capability_id} must retain the open VS Code follow-up")
    evidence_artifacts = set(capability["expected_evidence"].get("artifacts", []))
    if "vscode-vsix" not in evidence_artifacts:
        raise SystemExit(f"{capability_id} must attribute package/live evidence to vscode-vsix")
if "installed vs code/vscodium activation smoke" not in document_path.read_text(encoding="utf-8").lower():
    raise SystemExit("editor parity docs must retain the missing host activation boundary")

print("editor parity VS Code partial-foundation fixture passed")
PY
run_checker
echo "editor parity baseline fixture passed"
assert_no_hashed_targets

mutate_fixture module-shapes
set +e
module_shapes_output="$(run_checker 2>&1)"
module_shapes_result=$?
set -e
if (( module_shapes_result != 0 )); then
  echo "editor parity valid Rust module-shapes fixture failed" >&2
  printf '%s\n' "$module_shapes_output" >&2
  exit 1
fi
git -C "$fixture_repo" checkout -q -- fixtures/editor-parity/contract.json
echo "editor parity valid Rust module-shapes fixture passed"

run_checker >"$test_root/concurrent-a.log" 2>&1 &
first_pid=$!
run_checker >"$test_root/concurrent-b.log" 2>&1 &
second_pid=$!
wait "$first_pid"
wait "$second_pid"
assert_no_hashed_targets
echo "editor parity concurrent fixture passed"

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
  if [[ "$output" == *"Traceback"* || "$output" == *"AttributeError"* || "$output" == *"TypeError"* ]]; then
    echo "editor parity hostile fixture raised an uncontrolled Python exception: $mutation" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  echo "editor parity hostile fixture rejected: $mutation"
  git -C "$fixture_repo" checkout -q -- fixtures/editor-parity/contract.json \
    docs/editor-parity-contract.md crates/recite-compiler/tests/authoring_build.rs \
    crates/recite-compiler/tests/authoring_catalog_summary.rs \
    crates/recite-lsp/tests/module_tests.inc crates/recite-lsp/build.rs \
    shared-build.inc shared_workspace.rs
}

expect_failure traversal "path escapes the repository"
expect_failure client "implemented client vscode needs an implemented artifact"
expect_failure distribution "implemented distribution vs-marketplace needs an implemented artifact"
expect_failure capability-platform "partial capability lsp.completion cannot claim linux platform status implemented"
expect_failure capability-evidence "partial capability lsp.completion cannot claim implemented evidence"
expect_failure duplicate "capabilities IDs must be unique"
expect_failure malformed "evidence command must name a cargo integration test and filter"
expect_failure stale-evidence "evidence command does not name an existing runnable test"
expect_failure stale-module-evidence "evidence command does not name an existing runnable test"
expect_failure preserved-mtime-disconnected-module "evidence target has no Cargo-discovered runnable tests"
expect_failure block-commented-stale-test "evidence target has no Cargo-discovered runnable tests"
expect_failure block-commented-include-test "evidence command does not name an existing runnable test discovered by Cargo"
expect_failure build-input "cargo test-target compilation failed"
expect_failure shared-build-input "cargo test-target compilation failed"
expect_failure shared-workspace-input "cargo test-target compilation failed"
expect_failure evidence-traversal "evidence target escapes the repository"
expect_failure orphan-utf16 "orphaned=['orphan-utf16-crlf-non-bmp']"
expect_failure disconnected-module "evidence command does not name an existing runnable test discovered by Cargo"
expect_failure neovim-stale-filetype "Neovim filetype evidence cannot retain stale no-activation wording"
assert_no_hashed_targets
expect_failure reciprocity "artifact vscode-vsix client list must exactly reciprocate"
expect_failure topology "VS Code and VSCodium must share one VSIX artifact topology"
expect_failure wrong-primary "Neovim distribution primary artifact must match its capability artifact"
expect_failure missing-grammar-support "Neovim distribution supporting artifacts must include tree-sitter-grammar"
expect_failure unknown-supporting-artifact "distribution neovim-distribution references unknown supporting artifact unknown-editor-artifact"
expect_failure neovim-client-topology "Neovim client primary artifact must be neovim-runtimepath"
expect_failure zed-tree-sitter-claim "Zed must not claim the Neovim Tree-sitter grammar without compatibility evidence"
expect_failure client-platform-shape "client neovim must name exactly Linux, macOS, and Windows status"
expect_failure implemented-client-platform-shape "implemented client neovim needs platform evidence"
expect_failure neovim-evidence-shape "capability editor.neovim.syntax-projection expected_evidence must be an object"
expect_failure implementation-status-shape "capability lsp.completion has invalid implementation status"
expect_failure status-values-shape "status_values must contain exactly the four contract statuses"
expect_failure client-artifacts-shape "client neovim artifacts must include its primary artifact"
expect_failure distribution-artifacts-shape "distribution neovim-distribution artifacts must include its primary artifact"
expect_failure evidence-artifacts-shape "capability lsp.completion artifacts must be a non-empty array"
expect_failure follow-up-shape "capability lsp.completion must name a follow-up issue"
expect_failure symlink-artifact-component "artifact vscode-vsix path must not traverse symlink component"
expect_failure symlink "scenario lsp-stdio-baseline fixture must not be a symlink"
expect_failure symlink-component "scenario lsp-stdio-baseline fixture must not traverse symlink component"
expect_failure symlink-contract-control "editor parity fixture must not be a symlink"
expect_failure symlink-document-control "editor parity documentation must not be a symlink"
