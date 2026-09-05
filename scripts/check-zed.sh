#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-zed.sh [repo-root]

Checks the source/package evidence for the Recite Zed extension. This gate
does not claim installed Zed host activation or rendering smoke.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi
if (( $# > 1 )); then
  usage >&2
  exit 2
fi

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  repo_root="$(git -C "$input_root" rev-parse --show-toplevel 2>/dev/null)" || {
    echo "repo root is not a git checkout: $input_root" >&2
    exit 2
  }
else
  repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
    echo "unable to resolve Git repository root" >&2
    exit 2
  }
fi

# Tree-sitter's parser cache must remain scoped to this checkout. This also
# makes the gate deterministic in environments where the user cache is read-only.
if [[ -z "${XDG_CACHE_HOME:-}" || ! -d "$XDG_CACHE_HOME" || ! -w "$XDG_CACHE_HOME" ]]; then
  export XDG_CACHE_HOME="$repo_root/target/tree-sitter-cache"
fi

extension_dir="$repo_root/editors/zed"
query="$extension_dir/languages/recite/highlights.scm"
fixture="$repo_root/fixtures/editor-parity/zed/incomplete.recite"
grammar_revision="209ea23195f674a18be0b8f87e037273fb3296bd"

required_files=(
  Cargo.toml
  Cargo.lock
  LICENSE-APACHE
  LICENSE-MIT
  README.md
  extension.toml
  src/lib.rs
  languages/recite/config.toml
  languages/recite/highlights.scm
  languages/recite/tasks.json
)
for relative_path in "${required_files[@]}"; do
  candidate="$extension_dir/$relative_path"
  if [[ ! -f "$candidate" || -L "$candidate" ]]; then
    echo "missing or symlinked Zed extension file: ${candidate#$repo_root/}" >&2
    exit 2
  fi
done

while IFS= read -r -d '' symlink; do
  echo "Zed extension package must not contain symlinks: ${symlink#$repo_root/}" >&2
  exit 1
done < <(find "$extension_dir" -type l -print0)

echo "== Zed manifest/config/task contract =="
python3 - "$extension_dir" "$repo_root" "$grammar_revision" <<'PY'
import json
import pathlib
import sys
import tomllib

extension = pathlib.Path(sys.argv[1])
repo = pathlib.Path(sys.argv[2])
revision = sys.argv[3]

def fail(message):
    raise SystemExit(message)

with (extension / "extension.toml").open("rb") as handle:
    manifest = tomllib.load(handle)
if manifest.get("schema_version") != 1:
    fail("Zed extension manifest must use schema_version = 1")
if manifest.get("id") != "recite" or manifest.get("name") != "Recite":
    fail("Zed extension manifest must identify the Recite extension")
grammar = manifest.get("grammars", {}).get("recite")
if grammar != {
    "repository": "https://github.com/plethu/recite",
    "rev": revision,
    "path": "editors/recite-tree-sitter",
}:
    fail(f"Zed grammar source/pin drifted: {grammar!r}")
language_server = manifest.get("language_servers", {}).get("recite-lsp")
if language_server != {"name": "Recite Language Server", "languages": ["Recite"]}:
    fail(f"Zed language-server registration drifted: {language_server!r}")

with (extension / "Cargo.toml").open("rb") as handle:
    cargo = tomllib.load(handle)
if "workspace" not in cargo:
    fail("Zed Cargo.toml must define an isolated workspace")
package = cargo.get("package", {})
if package.get("license") != "MIT OR Apache-2.0":
    fail("Zed Cargo.toml must retain the dual license")
if cargo.get("lib", {}).get("crate-type") != ["cdylib"]:
    fail("Zed Cargo.toml must build a cdylib")
if cargo.get("dependencies", {}).get("zed_extension_api") != "0.7.0":
    fail("Zed extension must pin zed_extension_api = 0.7.0")

with (extension / "languages/recite/config.toml").open("rb") as handle:
    language = tomllib.load(handle)
if language != {
    "name": "Recite",
    "grammar": "recite",
    "path_suffixes": ["recite"],
    "line_comments": ["#"],
    "tab_size": 2,
    "hard_tabs": False,
}:
    fail(f"Zed language config drifted: {language!r}")

with (extension / "languages/recite/tasks.json").open(encoding="utf-8") as handle:
    tasks = json.load(handle)
if not isinstance(tasks, list) or not tasks:
    fail("Zed language tasks must be a non-empty array")
labels = {task.get("label") for task in tasks if isinstance(task, dict)}
expected_labels = {
    "Recite: validate current file",
    "Recite: extract current file",
    "Recite: compile current file",
    "Recite: watch worktree",
}
if labels != expected_labels or len(tasks) != len(expected_labels):
    fail(f"Zed task set drifted: {sorted(labels)!r}")
for task in tasks:
    if task.get("command") != "recite":
        fail(f"Zed task must invoke the structured recite CLI directly: {task!r}")
    args = task.get("args")
    if not isinstance(args, list) or "--output-format" not in args:
        fail(f"Zed task lacks an argv structured-output opt-in: {task!r}")
    output_index = args.index("--output-format")
    if output_index + 1 >= len(args) or args[output_index + 1] != "structured":
        fail(f"Zed task must request --output-format structured: {task!r}")
    if task.get("cwd") != "$ZED_WORKTREE_ROOT":
        fail(f"Zed task must run from the current worktree: {task!r}")
    if task.get("save") != "current":
        fail(f"Zed task must save only the current buffer before launch: {task!r}")
    if any(key in task for key in ("problemMatcher", "problem_matcher", "parser")):
        fail(f"Zed task must not parse human or NDJSON output: {task!r}")
    if any(isinstance(value, str) and any(operator in value for operator in ("&&", "||", ";", "|")) for value in [task.get("command"), *args]):
        fail(f"Zed task contains shell composition instead of argv: {task!r}")
    label = task["label"]
    if label.endswith("compile current file"):
        if "--output" not in args:
            fail("compile task must name an explicit output path")
        output = args[args.index("--output") + 1]
        if output != "$ZED_DIRNAME/$ZED_STEM.recitec":
            fail(f"compile task output path is not the documented sibling: {output!r}")
        if "$ZED_FILE" not in args:
            fail(f"compile task lacks $ZED_FILE: {task!r}")
    elif label.endswith("current file"):
        if "$ZED_FILE" not in args:
            fail(f"current-file task lacks $ZED_FILE: {task!r}")
    elif label.endswith("watch worktree"):
        if "$ZED_WORKTREE_ROOT" not in args:
            fail("watch task must derive its project root from $ZED_WORKTREE_ROOT")

if any("run" in task.get("args", []) or "trace" in task.get("args", []) for task in tasks):
    fail("Zed package must not ship run/trace tasks without derivable asset/block/fixture inputs")

inventory = {
    path.relative_to(extension).as_posix()
    for path in extension.rglob("*")
    if path.is_file() and "target" not in path.relative_to(extension).parts
}
expected_inventory = {
    "Cargo.lock",
    "Cargo.toml",
    "LICENSE-APACHE",
    "LICENSE-MIT",
    "README.md",
    "extension.toml",
    "src/lib.rs",
    "src/launcher.rs",
    "src/tests.rs",
    "languages/recite/config.toml",
    "languages/recite/highlights.scm",
    "languages/recite/tasks.json",
}
if inventory != expected_inventory:
    fail(f"Zed package inventory drifted: extra={sorted(inventory - expected_inventory)!r}, missing={sorted(expected_inventory - inventory)!r}")

parity = json.loads((repo / "fixtures/editor-parity/contract.json").read_text(encoding="utf-8"))
zed = next(client for client in parity["clients"] if client["id"] == "zed")
artifact = next(item for item in parity["artifacts"] if item["id"] == "zed-extension")
zed_syntax = next(item for item in parity["capabilities"] if item["id"] == "editor.zed.syntax-projection")
if zed["status"] != "partial" or zed["platform_status"]["linux"] != "partial":
    fail("parity contract must record Zed as partial on Linux")
if artifact["status"] != "partial" or zed_syntax["implementation_status"] != "partial":
    fail("parity contract must record checked Zed source/package evidence as partial")
expected_partial = {"editor.filetype.registration", "editor.zed.syntax-projection"}
actual_partial = set()
for capability in parity.get("capabilities", []):
    if capability.get("client_status", {}).get("zed") != "partial":
        continue
    capability_id = capability.get("id")
    actual_partial.add(capability_id)
    evidence = capability.get("expected_evidence", {})
    evidence_commands = evidence.get("commands")
    if evidence_commands is None:
        evidence_commands = [evidence.get("command")]
    if "scripts/check-zed.sh" not in evidence_commands:
        fail(f"partial Zed capability lacks scripts/check-zed.sh evidence: {capability_id}")
    evidence_artifacts = evidence.get("artifacts")
    if evidence_artifacts is None:
        evidence_artifacts = [evidence.get("artifact")]
    if "zed-extension" not in evidence_artifacts:
        fail(f"partial Zed capability lacks zed-extension evidence: {capability_id}")
if actual_partial != expected_partial:
    fail(f"Zed partial capability set drifted: expected={sorted(expected_partial)!r}, actual={sorted(actual_partial)!r}")
print("manifest, language config, tasks, package inventory, and parity status passed")
PY

echo "== dual-license content =="
cmp -s "$repo_root/LICENSE-APACHE" "$extension_dir/LICENSE-APACHE" || {
  echo "Zed Apache license does not match the repository license" >&2
  exit 1
}
cmp -s "$repo_root/LICENSE-MIT" "$extension_dir/LICENSE-MIT" || {
  echo "Zed MIT license does not match the repository license" >&2
  exit 1
}

echo "== pinned grammar reachability and query drift =="
git -C "$repo_root" cat-file -e "$grammar_revision^{commit}"
git -C "$repo_root" show "$grammar_revision:editors/recite-tree-sitter/grammar.js" >/dev/null
git -C "$repo_root" show "$grammar_revision:editors/recite-tree-sitter/queries/highlights.scm" | cmp -s - "$query" || {
  echo "Zed highlights query drifted from the pinned Recite query" >&2
  exit 1
}
if ! command -v tree-sitter >/dev/null 2>&1; then
  echo "missing required tool: tree-sitter" >&2
  exit 2
fi
query_output="$(mktemp "${TMPDIR:-/tmp}/recite-zed-query.XXXXXX")"
pinned_grammar_dir="$(mktemp -d "${TMPDIR:-/tmp}/recite-zed-grammar.XXXXXX")"
test_list=""
cleanup() {
  rm -f "$query_output" "${test_list:-}"
  rm -rf "$pinned_grammar_dir"
}
trap cleanup EXIT
git -C "$repo_root" archive "$grammar_revision" editors/recite-tree-sitter \
  | tar -x -C "$pinned_grammar_dir" --strip-components=2
if [[ ! -f "$pinned_grammar_dir/grammar.js" || ! -f "$pinned_grammar_dir/test/fixtures/capture-values.recite" ]]; then
  echo "pinned grammar materialization is incomplete" >&2
  exit 1
fi
tree-sitter query --grammar-path "$pinned_grammar_dir" --captures "$query" "$fixture" >"$query_output"
tree-sitter query --grammar-path "$pinned_grammar_dir" --captures "$query" "$pinned_grammar_dir/test/fixtures/capture-values.recite" >>"$query_output"
for capture in keyword punctuation.special label string.special variable.parameter; do
  if ! grep -Fq " - $capture," "$query_output"; then
    echo "Zed highlights query did not emit required capture @$capture" >&2
    exit 1
  fi
done
echo "pinned grammar and lexical captures passed"

echo "== isolated extension host/API checks =="
cargo check --locked --manifest-path "$extension_dir/Cargo.toml"
test_list="$(mktemp "${TMPDIR:-/tmp}/recite-zed-tests.XXXXXX")"
cargo test --locked --manifest-path "$extension_dir/Cargo.toml" -- --list | tee "$test_list"
for test_name in \
  configured_path_preserves_arguments_and_sorts_environment \
  path_fallback_carries_configured_arguments_and_environment \
  blank_configured_path_is_refused_without_fallback \
  missing_binary_error_names_install_and_configuration \
  equal_environment_keys_retain_input_order; do
  if ! grep -Fq "tests::${test_name}: test" "$test_list"; then
    echo "missing named Zed launcher unit test: $test_name" >&2
    exit 1
  fi
done
echo "named Zed launcher unit tests discovered: 5"
cargo test --locked --manifest-path "$extension_dir/Cargo.toml"
if rustup target list --installed 2>/dev/null | grep -Fxq wasm32-wasip2; then
  cargo check --locked --manifest-path "$extension_dir/Cargo.toml" --target wasm32-wasip2
else
  echo "RESIDUAL: wasm32-wasip2 is unavailable; host Cargo check/test above are the deterministic API evidence."
fi

echo "== real recite-lsp stdio parity =="
(
  cd "$repo_root"
  cargo test --locked -p recite-lsp --test editor_parity initialize_and_project_features_use_shared_stdio_contract
)

echo "RESIDUAL: installed Zed host activation/rendering smoke is unavailable or unexecuted; macOS/Windows smoke, gallery publication, dynamic tasks, parsed structured task diagnostics, and a task watch-cancellation controller are not claimed."
echo "Zed source/package and shared-protocol evidence passed."
