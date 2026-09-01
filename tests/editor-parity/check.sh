#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT

fixture_repo="$test_root/repo"
mkdir -p "$fixture_repo/docs" "$fixture_repo/fixtures/editor-parity" \
  "$fixture_repo/fixtures/recite/valid" "$fixture_repo/fixtures/recite/invalid" \
  "$fixture_repo/fixtures/schema/valid" "$fixture_repo/scripts" \
  "$fixture_repo/crates/recite-lsp/tests" \
  "$fixture_repo/crates/recite-cli/tests" \
  "$fixture_repo/crates/recite-compiler/tests/authoring_build"
cp "$repo_root/scripts/check-editor-parity.sh" "$fixture_repo/scripts/"
cp "$repo_root/docs/editor-parity-contract.md" "$fixture_repo/docs/"
cp "$repo_root/fixtures/editor-parity/contract.json" "$fixture_repo/fixtures/editor-parity/"
cp "$repo_root/fixtures/recite/valid/language_pressure.recite" "$fixture_repo/fixtures/recite/valid/"
cp "$repo_root/fixtures/recite/valid/locale_fallback_fr.po" "$fixture_repo/fixtures/recite/valid/"
cp "$repo_root/fixtures/recite/valid/core_language_spike.recite" "$fixture_repo/fixtures/recite/valid/"
cp "$repo_root/fixtures/recite/invalid/parser_marker_leading_prose.recite" "$fixture_repo/fixtures/recite/invalid/"
cp "$repo_root/fixtures/schema/valid/generated_manifest.json" "$fixture_repo/fixtures/schema/valid/"
cp "$repo_root/fixtures/schema/valid/full_manifest.json" "$fixture_repo/fixtures/schema/valid/"
cp -R "$repo_root/tests/editor-parity/cargo-fixture/." "$fixture_repo/"
chmod +x "$fixture_repo/scripts/check-editor-parity.sh"

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
from pathlib import Path

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
elif mutation == "duplicate":
    contract["capabilities"].append(dict(contract["capabilities"][0]))
elif mutation == "malformed":
    capability = next(capability for capability in contract["capabilities"] if capability["id"] == "lsp.completion")
    capability["expected_evidence"]["command"] = "not a cargo test command"
elif mutation == "stale-evidence":
    capability = next(capability for capability in contract["capabilities"] if capability["id"] == "lsp.completion")
    capability["expected_evidence"]["command"] = "cargo test --locked -p recite-lsp --test editor_parity no_such_test"
elif mutation == "stale-module-evidence":
    capability = next(capability for capability in contract["capabilities"] if capability["id"] == "command.structured.results")
    capability["expected_evidence"]["command"] = "cargo test --locked -p recite-compiler --test authoring_build invented::projects_every_lifecycle_state_with_stable_fields"
elif mutation == "disconnected-module":
    fixture_repo = Path(path).parents[2]
    root = fixture_repo / "crates/recite-compiler/tests/authoring_build.rs"
    source = root.read_text(encoding="utf-8")
    declaration = '#[path = "authoring_build/status_projection.rs"]\nmod status_projection;\n'
    if declaration not in source:
        raise SystemExit("status projection module declaration was not present")
    root.write_text(source.replace(declaration, "", 1), encoding="utf-8")
elif mutation == "reciprocity":
    artifact = next(artifact for artifact in contract["artifacts"] if artifact["id"] == "vscode-vsix")
    artifact["clients"].remove("vscode")
elif mutation == "topology":
    client = next(client for client in contract["clients"] if client["id"] == "vscodium")
    client["artifact"] = "neovim-setup"
elif mutation == "symlink":
    fixture_repo = Path(path).parents[2]
    outside = fixture_repo.parent / "outside-editor-parity.recite"
    outside.write_text("outside\n", encoding="utf-8")
    canonical = fixture_repo / "fixtures/recite/valid/core_language_spike.recite"
    canonical.unlink()
    canonical.symlink_to(outside)
elif mutation == "symlink-component":
    import shutil

    fixture_repo = Path(path).parents[2]
    valid = fixture_repo / "fixtures/recite/valid"
    internal = fixture_repo / "fixtures/recite/internal-valid"
    shutil.copytree(valid, internal, symlinks=True)
    shutil.rmtree(valid)
    valid.symlink_to(internal, target_is_directory=True)
elif mutation == "symlink-artifact-component":
    fixture_repo = Path(path).parents[2]
    alias = fixture_repo / "fixtures/editor-parity/artifact-alias"
    alias.symlink_to(alias.parent, target_is_directory=True)
    artifact = next(artifact for artifact in contract["artifacts"] if artifact["id"] == "vscode-vsix")
    artifact["status"] = "implemented"
    artifact["path"] = "fixtures/editor-parity/artifact-alias/contract.json"
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
expect_failure duplicate "capabilities IDs must be unique"
expect_failure malformed "evidence command must name a cargo integration test and filter"
expect_failure stale-evidence "evidence command does not name an existing runnable test"
expect_failure stale-module-evidence "evidence command does not name an existing runnable test"
expect_failure disconnected-module "evidence command does not name an existing runnable test discovered by Cargo"
expect_failure reciprocity "artifact vscode-vsix client list must exactly reciprocate"
expect_failure topology "VS Code and VSCodium must share one VSIX artifact topology"
expect_failure symlink-artifact-component "artifact vscode-vsix path must not traverse symlink component"
expect_failure symlink "scenario lsp-stdio-baseline fixture must not be a symlink"
expect_failure symlink-component "scenario lsp-stdio-baseline fixture must not traverse symlink component"
