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
mkdir -p "$fixture_repo/docs/evidence/editor-hosts" "$fixture_repo/tests/editor-hosts/neovim"
cp "$repo_root/scripts/check-editor-parity.sh" "$fixture_repo/scripts/"
cp "$repo_root/scripts/check-tree-sitter.sh" "$fixture_repo/scripts/"
cp "$repo_root/scripts/check-neovim.sh" "$fixture_repo/scripts/"
cp "$repo_root/scripts/check-vscode.sh" "$fixture_repo/scripts/"
cp "$repo_root/scripts/check-zed.sh" "$fixture_repo/scripts/"
cp "$repo_root/scripts/check-neovim-host.sh" "$fixture_repo/scripts/"
cp "$repo_root/scripts/check-vscode-host.sh" "$fixture_repo/scripts/"
cp -R "$repo_root/scripts/editor_parity" "$fixture_repo/scripts/"
cp "$repo_root/editors/recite-tree-sitter/grammar.js" "$fixture_repo/editors/recite-tree-sitter/"
cp -R "$repo_root/editors/recite-neovim/." "$fixture_repo/editors/recite-neovim/"
cp "$repo_root/docs/editor-parity-contract.md" "$fixture_repo/docs/"
cp "$repo_root/docs/evidence/editor-hosts/neovim-linux.md" "$fixture_repo/docs/evidence/editor-hosts/"
cp "$repo_root/docs/evidence/editor-hosts/vscode-linux.md" "$fixture_repo/docs/evidence/editor-hosts/"
cp "$repo_root/fixtures/editor-parity/contract.json" "$fixture_repo/fixtures/editor-parity/"
cp "$repo_root/fixtures/recite/valid/language_pressure.recite" "$fixture_repo/fixtures/recite/valid/"
cp "$repo_root/fixtures/recite/valid/locale_fallback_fr.po" "$fixture_repo/fixtures/recite/valid/"
cp "$repo_root/fixtures/recite/valid/core_language_spike.recite" "$fixture_repo/fixtures/recite/valid/"
cp "$repo_root/fixtures/recite/invalid/parser_marker_leading_prose.recite" "$fixture_repo/fixtures/recite/invalid/"
cp "$repo_root/fixtures/schema/valid/generated_manifest.json" "$fixture_repo/fixtures/schema/valid/"
cp "$repo_root/fixtures/schema/valid/full_manifest.json" "$fixture_repo/fixtures/schema/valid/"
cp -R "$repo_root/tests/editor-parity/cargo-fixture/." "$fixture_repo/"
cp "$repo_root/.gitignore" "$fixture_repo/"
cp "$repo_root/AGENTS.md" "$fixture_repo/AGENTS.md"
mkdir -p "$fixture_repo/.claude"
ln -s ../.agents/skills "$fixture_repo/.claude/skills"
ln -s AGENTS.md "$fixture_repo/CLAUDE.md"
chmod +x "$fixture_repo/scripts/check-editor-parity.sh" "$fixture_repo/scripts/check-tree-sitter.sh" "$fixture_repo/scripts/check-neovim.sh" "$fixture_repo/scripts/check-vscode.sh" "$fixture_repo/scripts/check-zed.sh" "$fixture_repo/scripts/check-neovim-host.sh" "$fixture_repo/scripts/check-vscode-host.sh"

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

expect_target_failure() {
  local target_dir="$1"
  local expected="${2:-CARGO_TARGET_DIR inside the repository must be exactly}"
  local output result
  set +e
  output="$(CARGO_TARGET_DIR="$target_dir" run_checker 2>&1)"
  result=$?
  set -e
  if (( result == 0 )) || [[ "$output" != *"$expected"* ]]; then
    echo "editor parity target boundary fixture missed: $target_dir" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  if [[ "$output" == *"Traceback"* || "$output" == *"AttributeError"* || "$output" == *"TypeError"* ]]; then
    echo "editor parity target boundary fixture raised an uncontrolled Python exception" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  echo "editor parity target boundary fixture rejected: $target_dir"
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
python3 "$repo_root/tests/editor-parity/assert_client_foundation.py" \
  "$fixture_repo/fixtures/editor-parity/contract.json" \
  "$fixture_repo/docs/editor-parity-contract.md"
run_checker
echo "editor parity baseline fixture passed"
assert_no_hashed_targets

CARGO_TARGET_DIR="$test_root/external-target" run_checker
echo "editor parity external target fixture passed"
expect_target_failure "$fixture_repo"
expect_target_failure "$fixture_repo/crates"
expect_target_failure "$fixture_repo/target/custom"

target_probe_root="$test_root/external-target-probes"
mkdir -p "$target_probe_root/real-parent"
ln -s "$fixture_repo" "$target_probe_root/to-repo-root"
ln -s "$fixture_repo/crates" "$target_probe_root/to-crates"
ln -s "$fixture_repo/target" "$target_probe_root/to-target"
ln -s "$fixture_repo" "$target_probe_root/real-parent/repo-parent"
expect_target_failure "$target_probe_root/to-repo-root" "CARGO_TARGET_DIR must not traverse a symlink component"
expect_target_failure "$target_probe_root/to-crates" "CARGO_TARGET_DIR must not traverse a symlink component"
expect_target_failure "$target_probe_root/to-target" "CARGO_TARGET_DIR must not traverse a symlink component"
expect_target_failure "$target_probe_root/real-parent/repo-parent/target" "CARGO_TARGET_DIR must not traverse a symlink component"

python3 - "$fixture_repo" <<'PY'
import os
import shutil
import subprocess
import sys
from pathlib import Path

repo = Path(sys.argv[1])
sys.path.insert(0, str(repo / "scripts"))
from editor_parity.content_digest import selected_target_digest, workspace_files
from editor_parity.model import Context

context = Context(repo, [], repo / "target")
before = selected_target_digest(context, "recite-lsp")

# Generated documentation/editor output is ignored and must not invalidate the
# evidence executable merely because a packaging or docs command touched it.
ignored_outputs = [
    repo / "docs-site/.astro/cache.json",
    repo / "docs-site/dist/index.html",
    repo / "editors/vscode/dist/extension.js",
    repo / "editors/vscode/recite.vsix",
]
for output in ignored_outputs:
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(b"generated output")
ignored_after_creation = selected_target_digest(context, "recite-lsp")
if before != ignored_after_creation:
    raise SystemExit("ignored generated output changed the parity digest")
for output in ignored_outputs:
    output.write_bytes(b"rewritten generated output")
if ignored_after_creation != selected_target_digest(context, "recite-lsp"):
    raise SystemExit("modifying ignored generated output changed the parity digest")

# A force-added path is tracked input even when its directory is ignored. This
# is the explicit escape hatch for compiler-visible generated/source files.
force_added_digest = ignored_after_creation
for relative in (
    "editors/vscode/dist/force-added.js",
    "target/force-added.rs",
    "node_modules/force-added.js",
    "__pycache__/force-added.pyc",
    "force-added.pyo",
):
    force_added = repo / relative
    force_added.parent.mkdir(parents=True, exist_ok=True)
    force_added.write_bytes(b"force-added compiler input")
    subprocess.run(["git", "-C", os.fsencode(repo), "add", "-f", "--", os.fsencode(relative)], check=True)
    next_digest = selected_target_digest(context, "recite-lsp")
    if next_digest == force_added_digest:
        raise SystemExit(f"force-added path did not change the parity digest: {relative}")
    force_added_digest = next_digest

# A nonignored untracked input is included regardless of extension or spaces.
untracked = repo / "digest-inputs/input with spaces.arbitrary"
untracked.parent.mkdir(parents=True, exist_ok=True)
untracked.write_bytes(b"first source input")
untracked_digest = selected_target_digest(context, "recite-lsp")
original_stat = untracked.stat()
untracked.write_bytes(b"second source input")
os.utime(untracked, ns=(original_stat.st_atime_ns, original_stat.st_mtime_ns))
if untracked_digest == selected_target_digest(context, "recite-lsp"):
    raise SystemExit("restored-mtime untracked input did not change the parity digest")

# Git's NUL-delimited output preserves arbitrary filesystem bytes on platforms
# whose filesystem supports them. Skip this optional stress case on Windows.
if os.name != "nt":
    non_utf8 = repo / os.fsdecode(b"digest-inputs/non-utf8-\xff.input")
    non_utf8_before = selected_target_digest(context, "recite-lsp")
    non_utf8.write_bytes(b"non-UTF-8 filename input")
    if non_utf8_before == selected_target_digest(context, "recite-lsp"):
        raise SystemExit("non-UTF-8 untracked input did not change the parity digest")

# Git index mode is part of the identity, and a worktree executable-bit change
# is also relevant because Cargo compiles the worktree rather than the index.
if os.name != "nt":
    mode_probe = repo / "digest-inputs/mode-input"
    mode_probe.write_bytes(b"mode input")
    subprocess.run(["git", "-C", os.fsencode(repo), "add", "--", os.fsencode("digest-inputs/mode-input")], check=True)
    mode_before = selected_target_digest(context, "recite-lsp")
    subprocess.run(["git", "-C", os.fsencode(repo), "update-index", "--chmod=+x", "--", os.fsencode("digest-inputs/mode-input")], check=True)
    mode_after_index = selected_target_digest(context, "recite-lsp")
    if mode_before == mode_after_index:
        raise SystemExit("Git executable mode change did not change the parity digest")
    mode_probe.chmod(0o755)
    mode_after_worktree = selected_target_digest(context, "recite-lsp")
    if mode_after_index == mode_after_worktree:
        raise SystemExit("worktree executable mode change did not change the parity digest")

# An untracked nested repository is enumerated by Git as a directory. A staged
# gitlink is a separate unsupported index mode. Both must fail closed rather
# than disappear and produce false-green evidence.
nested_repo = repo / "nested-repo"
nested_repo.mkdir(parents=True, exist_ok=True)
subprocess.run(["git", "-C", os.fsencode(nested_repo), "init", "-q"], check=True)
nested_context = Context(repo, [], repo / "target")
selected_target_digest(nested_context, "recite-lsp")
if not any("must not be a directory" in error for error in nested_context.errors):
    raise SystemExit("untracked nested repository was not rejected as a directory input")
subprocess.run(["git", "-C", os.fsencode(nested_repo), "config", "user.name", "Nested Fixture"], check=True)
subprocess.run(["git", "-C", os.fsencode(nested_repo), "config", "user.email", "nested@example.invalid"], check=True)
subprocess.run(["git", "-C", os.fsencode(nested_repo), "config", "commit.gpgsign", "false"], check=True)
(nested_repo / "source").write_bytes(b"nested source")
subprocess.run(["git", "-C", os.fsencode(nested_repo), "add", "source"], check=True)
subprocess.run(["git", "-C", os.fsencode(nested_repo), "commit", "-q", "-m", "nested fixture"], check=True)
nested_oid = subprocess.check_output(["git", "-C", os.fsencode(nested_repo), "rev-parse", "HEAD"]).strip()
subprocess.run(["git", "-C", os.fsencode(repo), "update-index", "--add", "--cacheinfo", b"160000," + nested_oid + b",gitlink-fixture"], check=True)
gitlink_context = Context(repo, [], repo / "target")
selected_target_digest(gitlink_context, "recite-lsp")
if not any("must not be a gitlink" in error for error in gitlink_context.errors):
    raise SystemExit("staged gitlink was not rejected with a controlled error")
subprocess.run(["git", "-C", os.fsencode(repo), "update-index", "--force-remove", "--", "gitlink-fixture"], check=True)
shutil.rmtree(nested_repo)

# Python bytecode is also ignored/excluded, including bytecode created by the
# checker itself.
before_bytecode = selected_target_digest(context, "recite-lsp")
bytecode = repo / "scripts/editor_parity/__pycache__"
bytecode.mkdir(parents=True, exist_ok=True)
(bytecode / "checker.cpython-314.pyc").write_bytes(b"bytecode")
(repo / "checker-output.pyo").write_bytes(b"bytecode")
after = selected_target_digest(context, "recite-lsp")
if before_bytecode != after:
    raise SystemExit("Python bytecode changed the parity digest")
paths = [path.relative_to(repo).as_posix() for path in workspace_files(context)]
if paths != sorted(paths, key=os.fsencode):
    raise SystemExit("Git digest inputs were not sorted by stable path bytes")
if any(path.endswith(("checker.cpython-314.pyc", "checker-output.pyo")) for path in paths):
    raise SystemExit("untracked Python bytecode entered parity digest inputs")
if "__pycache__/force-added.pyc" not in paths or "force-added.pyo" not in paths:
    raise SystemExit("force-added Python bytecode did not remain a digest input")

nested_metadata = repo / "nested/CLAUDE.md"
nested_metadata.parent.mkdir(parents=True, exist_ok=True)
nested_metadata.symlink_to(repo / "AGENTS.md")
nested_metadata_context = Context(repo, [], repo / "target")
selected_target_digest(nested_metadata_context, "recite-lsp")
if not any("workspace digest input must not be a symlink" in error for error in nested_metadata_context.errors):
    raise SystemExit("nested CLAUDE.md was incorrectly treated as repository metadata")
shutil.rmtree(nested_metadata.parent)
print("editor parity Git-aware digest fixture passed")
PY

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

python3 "$repo_root/tests/editor-parity/diagnostic_probes.py" "$fixture_repo"

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
    crates/recite-lsp/tests/module_tests.inc crates/recite-lsp/tests/module_shapes.rs \
    crates/recite-lsp/build.rs \
    shared-build.inc shared_workspace.rs \
    scripts/check-neovim-host.sh scripts/check-vscode-host.sh
  rm -rf "$fixture_repo/digest-inputs" "$fixture_repo/../outside-input"
}

mutate_fixture compiler-diagnostic
set +e
compiler_diagnostic_a="$(run_checker 2>&1)"
compiler_diagnostic_a_result=$?
compiler_diagnostic_b="$(run_checker 2>&1)"
compiler_diagnostic_b_result=$?
set -e
if (( compiler_diagnostic_a_result == 0 || compiler_diagnostic_b_result == 0 )) \
  || [[ "$compiler_diagnostic_a" != "$compiler_diagnostic_b" ]] \
  || [[ "$compiler_diagnostic_a" != *"editor parity compiler diagnostic fixture"* ]] \
  || [[ "$compiler_diagnostic_a" != *"... [truncated]"* ]] \
  || (( ${#compiler_diagnostic_a} > 40000 )); then
  echo "editor parity compiler diagnostic fixture was not bounded and deterministic" >&2
  printf '%s\n' "$compiler_diagnostic_a" >&2
  exit 1
fi
echo "editor parity compiler diagnostic fixture surfaced and bounded the Cargo detail"
git -C "$fixture_repo" checkout -q -- fixtures/editor-parity/contract.json crates/recite-lsp/tests/module_shapes.rs

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
expect_failure contained-file-link "workspace digest input must not be a symlink"
expect_failure escaping-file-link "workspace digest input must not be a symlink"
expect_failure contained-directory-link "workspace digest input must not be a symlink"
expect_failure symlink-cycle "workspace digest input must not be a symlink"
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
expect_failure keyboard-follow-up "editor.keyboard.workflow must remain owned by open follow-up #202"
expect_failure keyboard-follow-up-missing "capability editor.keyboard.workflow must name a follow-up issue"
expect_failure keyboard-scenario-status "keyboard-workflow scenario must remain planned until installed-host evidence exists"
expect_failure keyboard-executable-evidence "unimplemented capability editor.keyboard.workflow must not claim an executable evidence command"
expect_failure keyboard-evidence-boundary "editor.keyboard.workflow known_limitation must name the headless evidence boundary"
expect_failure keyboard-document-wording "keyboard workflow documentation must retain 'broader milestone 5 accessibility proof'"
mutate_fixture keyboard-valid-host-evidence
set +e
keyboard_host_output="$(run_checker 2>&1)"
keyboard_host_result=$?
set -e
if (( keyboard_host_result != 0 )); then
  echo "editor parity valid installed-host keyboard evidence fixture failed" >&2
  printf '%s\n' "$keyboard_host_output" >&2
  exit 1
fi
echo "editor parity valid installed-host keyboard evidence fixture passed"
git -C "$fixture_repo" checkout -q -- fixtures/editor-parity/contract.json
expect_failure keyboard-host-record-missing "must name a non-empty architecture"
expect_failure keyboard-host-runner-missing "host_records require an installed-host evidence runner command"
expect_failure keyboard-host-runner-file-missing "installed-host evidence runner does not exist"
expect_failure keyboard-host-runner-non-executable "installed-host evidence runner is not executable"
expect_failure keyboard-host-runner-mismatch "runner scripts/check-vscode-host.sh does not match client neovim"
expect_failure keyboard-host-platform-overclaim "host records do not cover claimed macos platform evidence"
expect_failure keyboard-host-scenario-mismatch "partial/implemented status requires a partial/implemented keyboard-workflow scenario"
expect_failure keyboard-host-doc-missing "evidence document does not exist"
expect_failure keyboard-host-key-sequence "key_sequence must be a non-empty string or array"
expect_failure keyboard-host-no-leak "keyboard assertion process_leak_check must be true"
mutate_fixture non-keyboard-dual-client-host-evidence
set +e
non_keyboard_dual_output="$(run_checker 2>&1)"
non_keyboard_dual_result=$?
set -e
if (( non_keyboard_dual_result != 0 )); then
  echo "editor parity valid dual-client installed-host evidence fixture failed" >&2
  printf '%s\n' "$non_keyboard_dual_output" >&2
  exit 1
fi
echo "editor parity valid dual-client installed-host evidence fixture passed"
git -C "$fixture_repo" checkout -q -- fixtures/editor-parity/contract.json
mutate_fixture non-keyboard-incremental-host-evidence
set +e
non_keyboard_incremental_output="$(run_checker 2>&1)"
non_keyboard_incremental_result=$?
set -e
if (( non_keyboard_incremental_result != 0 )); then
  echo "editor parity valid incremental installed-host evidence fixture failed" >&2
  printf '%s\n' "$non_keyboard_incremental_output" >&2
  exit 1
fi
echo "editor parity valid incremental installed-host evidence fixture passed"
git -C "$fixture_repo" checkout -q -- fixtures/editor-parity/contract.json
expect_failure host-dual-client-mismatch "runner scripts/check-vscode-host.sh does not match client zed"
expect_failure keyboard-host-missing-client-platform "host records do not cover claimed vscode client evidence"
expect_failure symlink-artifact-component "artifact vscode-vsix path must not traverse symlink component"
expect_failure symlink "scenario lsp-stdio-baseline fixture must not be a symlink"
expect_failure symlink-component "scenario lsp-stdio-baseline fixture must not traverse symlink component"
expect_failure symlink-contract-control "editor parity fixture must not be a symlink"
expect_failure symlink-document-control "editor parity documentation must not be a symlink"

if [[ -L "$repo_root/.claude/skills" && -L "$repo_root/CLAUDE.md" ]]; then
  CARGO_TARGET_DIR="$test_root/source-checkout-target" "$repo_root/scripts/check-editor-parity.sh" "$repo_root"
  echo "editor parity source-checkout metadata symlink fixture passed"
fi
