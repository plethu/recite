#!/usr/bin/env python3
"""Exercise editor-parity Cargo failure states without starting Cargo."""

import json
import subprocess
import sys
from pathlib import Path
from unittest.mock import patch


COMMAND = "cargo test --locked -p recite-lsp --test module_shapes inline::nested::nested_test"
TARGET_KEY = ("recite-lsp", "module_shapes")


def load_editor_parity(repo: Path):
    sys.path.insert(0, str(repo / "scripts"))
    from editor_parity import cargo_evidence, evidence
    from editor_parity.model import Context

    return cargo_evidence, evidence, Context


def compiler_failure(*args, **kwargs):
    rendered = "error: editor parity compiler diagnostic fixture " + ("x" * 8000)
    output = json.dumps(
        {
            "reason": "compiler-message",
            "message": {"level": "error", "rendered": rendered},
        }
    )
    return subprocess.CompletedProcess(args[0], 1, stdout=output, stderr="")


def timeout(*args, **kwargs):
    raise subprocess.TimeoutExpired(args[0], kwargs.get("timeout"))


def assert_single_error(context, expected: str) -> None:
    if context.errors != [expected]:
        raise SystemExit(f"unexpected parity diagnostic: {context.errors!r}")


def probe_compilation_failure(repo: Path, cargo_evidence, evidence, Context) -> None:
    context = Context(repo, [], repo / "target")
    with patch.object(cargo_evidence, "selected_target_digest", return_value="digest"), patch.object(
        cargo_evidence.subprocess, "run", side_effect=compiler_failure
    ) as run:
        evidence.validate_command(context, "editor.parity.diagnostics", COMMAND)
    if run.call_count != 1:
        raise SystemExit(f"compilation failure spawned {run.call_count} subprocesses")
    if len(context.errors) != 1:
        raise SystemExit(f"compilation failure cascaded diagnostics: {context.errors!r}")
    diagnostic = context.errors[0]
    if "cargo test-target compilation failed for recite-lsp/module_shapes" not in diagnostic:
        raise SystemExit(f"compiler failure was not surfaced: {diagnostic!r}")
    if "editor parity compiler diagnostic fixture" not in diagnostic or "... [truncated]" not in diagnostic:
        raise SystemExit(f"compiler failure detail was not bounded: {diagnostic!r}")


def probe_harness_timeout(repo: Path, cargo_evidence, evidence, Context) -> None:
    context = Context(repo, [], repo / "target")
    context.cargo_test_executable_cache[TARGET_KEY] = repo / "missing-test"
    with patch.object(cargo_evidence.subprocess, "run", side_effect=timeout) as run:
        evidence.validate_command(context, "editor.parity.diagnostics", COMMAND)
    if run.call_count != 1:
        raise SystemExit(f"harness timeout spawned {run.call_count} subprocesses")
    assert_single_error(context, "test harness discovery timed out after 120s for recite-lsp/module_shapes")


def probe_empty_discovery(repo: Path, cargo_evidence, evidence, Context) -> None:
    context = Context(repo, [], repo / "target")
    context.cargo_test_executable_cache[TARGET_KEY] = repo / "missing-test"
    result = subprocess.CompletedProcess(["missing-test", "--list"], 0, stdout="", stderr="")
    with patch.object(cargo_evidence.subprocess, "run", return_value=result) as run:
        evidence.validate_command(context, "editor.parity.diagnostics", COMMAND)
    if run.call_count != 1:
        raise SystemExit(f"empty discovery spawned {run.call_count} subprocesses")
    expected = [
        "evidence target has no Cargo-discovered runnable tests: recite-lsp/module_shapes",
        "capability editor.parity.diagnostics evidence command does not name an existing runnable test discovered by Cargo: inline::nested::nested_test",
    ]
    if context.errors != expected:
        raise SystemExit(f"successful empty discovery was not preserved: {context.errors!r}")


def probe_exact_selection_states(repo: Path, cargo_evidence, evidence, Context) -> None:
    context = Context(repo, [], repo / "target")
    context.cargo_test_executable_cache[TARGET_KEY] = repo / "missing-test"
    empty = subprocess.CompletedProcess(["missing-test", "--list"], 0, stdout="", stderr="")
    with patch.object(cargo_evidence.subprocess, "run", return_value=empty) as run:
        selection = cargo_evidence.exact_test_selection(context, *TARGET_KEY, "inline::nested::nested_test")
    if selection != set() or run.call_count != 1:
        raise SystemExit(f"successful empty exact selection was not preserved: {selection!r}")

    context = Context(repo, [], repo / "target")
    context.cargo_test_executable_cache[TARGET_KEY] = repo / "missing-test"
    with patch.object(cargo_evidence.subprocess, "run", side_effect=timeout) as run:
        selection = cargo_evidence.exact_test_selection(context, *TARGET_KEY, "inline::nested::nested_test")
        cached_selection = cargo_evidence.exact_test_selection(context, *TARGET_KEY, "inline::nested::nested_test")
    if selection is not None or cached_selection is not None or run.call_count != 1:
        raise SystemExit(f"failed exact selection was not cached explicitly: {selection!r}, {cached_selection!r}")
    assert_single_error(context, "test harness discovery timed out after 120s for recite-lsp/module_shapes")


def main() -> int:
    if len(sys.argv) != 2:
        raise SystemExit("usage: diagnostic_probes.py FIXTURE_REPO")
    repo = Path(sys.argv[1]).resolve()
    cargo_evidence, evidence, Context = load_editor_parity(repo)
    probe_compilation_failure(repo, cargo_evidence, evidence, Context)
    probe_harness_timeout(repo, cargo_evidence, evidence, Context)
    probe_empty_discovery(repo, cargo_evidence, evidence, Context)
    probe_exact_selection_states(repo, cargo_evidence, evidence, Context)
    print("editor parity mocked diagnostic probes passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
