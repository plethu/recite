# Sourced by check.sh after the baseline, digest, and diagnostic fixtures.
# Keep the mutation order explicit: each case restores the fixture before the
# next case, and the final source-checkout gate remains in check.sh.

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
expect_failure follow-up-shape "capability lsp.completion follow_up must be a valid issue reference when present"
expect_failure follow-up-historical "capability lsp.completion follow_up #51 is historical; use evidence_issues"
expect_failure evidence-issues-shape "capability lsp.completion must name non-empty historical evidence_issues"
expect_failure evidence-issues-open "capability lsp.completion evidence_issues must name closed historical issues, not #206"
expect_failure follow-up-owner-drift "capability lsp.completion follow_up #206 belongs to lsp.cancellation"
expect_failure cancellation-follow-up "lsp.cancellation must remain owned by serious-v1 follow-up #206"
expect_failure cancellation-status-inflation "unsupported capability lsp.cancellation cannot claim neovim status partial"
expect_failure cancellation-status-bypass "planned capability lsp.cancellation cannot claim neovim status partial"
expect_failure cancellation-evidence-status "lsp.cancellation expected_evidence.status must remain unsupported until #206"
expect_failure zed-code-action-support "lsp.code-actions must retain partial Zed host evidence"
expect_failure zed-rename-support "lsp.rename must retain partial Zed host evidence"
expect_failure zed-utf16-post-emoji "lsp.utf16.positions must retain Zed assertion 'post-emoji utf-16 completion request'"
expect_failure stable-id-zed-support "authoring.stable-id.operations must retain partial Zed host evidence"
expect_failure zed-lsp-provenance "lsp.code-actions must retain historical evidence issue #192 for Zed host evidence"
expect_failure zed-non-lsp-provenance "command.watch.lifecycle must retain historical evidence issue #192 for Zed host evidence"
expect_failure keyboard-follow-up "editor.keyboard.workflow must retain historical evidence issue #202"
expect_failure keyboard-follow-up-missing "capability editor.keyboard.workflow must name non-empty historical evidence_issues"
expect_failure keyboard-scenario-status "editor.keyboard.workflow partial/implemented status requires a partial/implemented keyboard-workflow scenario"
expect_failure keyboard-executable-evidence "capability editor.keyboard.workflow host_records require an installed-host evidence runner command"
expect_failure keyboard-evidence-boundary "editor.keyboard.workflow known_limitation must name the headless evidence boundary"
expect_failure keyboard-zed-sequence-provenance "editor.keyboard.workflow keyboard_sequence_scope must explain 'dedicated lsp ui action sequence'"
expect_failure keyboard-document-wording "editor parity documentation must retain 'broader milestone 5 accessibility proof'"
expect_failure m4-zed-task-diagnostics "Milestone 4 reconciliation must retain the Zed task-diagnostics limitation"
expect_failure m4-zed-native-cancellation "Milestone 4 reconciliation must retain the Zed native-cancellation limitation"
expect_failure m4-zed-built-in-run-trace "Milestone 4 reconciliation must retain the unsupported Zed built-in run/trace boundary"
expect_failure m4-zed-stale-didchange "Milestone 4 reconciliation must retain the lower-level stale-didchange boundary"
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
