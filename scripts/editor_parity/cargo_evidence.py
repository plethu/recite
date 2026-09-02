"""Cargo-backed test discovery and compiler diagnostics for editor parity."""

import json
import os
import subprocess
from pathlib import Path

from .content_digest import selected_target_digest
from .model import Context


_CARGO_TIMEOUT_SECONDS = 120
_MAX_CARGO_DIAGNOSTIC_LENGTH = 4096
_CARGO_DIAGNOSTIC_TRUNCATION = "... [truncated]"


def cargo_test_list(ctx: Context, package: str, target: str, test_filter: str | None = None) -> set[str] | None:
    executable = cargo_test_executable(ctx, package, target)
    if executable is None:
        return None
    command = [str(executable), "--list"]
    if test_filter is not None:
        command[1:1] = [test_filter, "--exact"]
    try:
        result = subprocess.run(
            command,
            cwd=ctx.repo_root,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env={**os.environ, "CARGO_TERM_COLOR": "never"},
            timeout=_CARGO_TIMEOUT_SECONDS,
            check=False,
        )
    except subprocess.TimeoutExpired:
        ctx.require(
            False,
            f"test harness discovery timed out after {_CARGO_TIMEOUT_SECONDS}s for {package}/{target}",
        )
        return None
    except OSError as error:
        ctx.require(False, f"test harness discovery could not start for {package}/{target}: {error}")
        return None
    if result.returncode != 0:
        output = (result.stdout + result.stderr).strip().splitlines()
        detail = output[-1] if output else "no cargo output"
        suffix = " exact selection" if test_filter is not None else " test list"
        ctx.require(False, f"test harness discovery failed for {package}/{target}{suffix}: {detail}")
        return None
    return {
        line[: -len(": test")].strip()
        for line in result.stdout.splitlines()
        if line.rstrip().endswith(": test") and line[: -len(": test")].strip()
    }


def cargo_test_executable(ctx: Context, package: str, target: str) -> Path | None:
    key = (package, target)
    if key in ctx.cargo_test_executable_cache:
        return ctx.cargo_test_executable_cache[key]
    digest = selected_target_digest(ctx, package)
    command = [
        os.environ.get("CARGO", "cargo"),
        "rustc",
        "--locked",
        "-p",
        package,
        "--test",
        target,
        "--message-format=json",
        "--",
        "--cfg",
        f'recite_editor_parity_source_digest="{digest}"',
    ]
    try:
        result = subprocess.run(
            command,
            cwd=ctx.repo_root,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env={**os.environ, "CARGO_TERM_COLOR": "never", "CARGO_TARGET_DIR": str(ctx.cargo_target_dir)},
            timeout=_CARGO_TIMEOUT_SECONDS,
            check=False,
        )
    except subprocess.TimeoutExpired:
        ctx.require(
            False,
            f"cargo test-target compilation timed out after {_CARGO_TIMEOUT_SECONDS}s for {package}/{target}",
        )
        ctx.cargo_test_executable_cache[key] = None
        return None
    except OSError as error:
        ctx.require(False, f"cargo test-target compilation could not start for {package}/{target}: {error}")
        ctx.cargo_test_executable_cache[key] = None
        return None
    executable = None
    for line in result.stdout.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        target_info = message.get("target", {})
        if (
            message.get("reason") == "compiler-artifact"
            and target_info.get("name") == target
            and "test" in target_info.get("kind", [])
            and isinstance(message.get("executable"), str)
        ):
            executable = message["executable"]
    if result.returncode != 0 or executable is None:
        detail = cargo_failure_detail(result.stdout, result.stderr)
        ctx.require(False, f"cargo test-target compilation failed for {package}/{target}: {detail}")
        ctx.cargo_test_executable_cache[key] = None
        return None
    executable_path = Path(executable)
    ctx.cargo_test_executable_cache[key] = executable_path
    return executable_path


def cargo_failure_detail(stdout: str, stderr: str) -> str:
    """Return one bounded, actionable detail from a failed Cargo invocation."""

    rendered = _first_rendered_cargo_diagnostic(stdout)
    if rendered is not None:
        return rendered
    output = (stdout + stderr).strip().splitlines()
    return _bound_cargo_diagnostic(output[-1] if output else "no cargo output")


def _first_rendered_cargo_diagnostic(output: str) -> str | None:
    fallback = None
    for line in output.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(message, dict):
            continue
        if message.get("reason") != "compiler-message":
            continue
        diagnostic = message.get("message")
        if not isinstance(diagnostic, dict):
            continue
        rendered = diagnostic.get("rendered")
        if not isinstance(rendered, str) or not rendered.strip():
            continue
        rendered = _bound_cargo_diagnostic(rendered)
        if diagnostic.get("level") == "error":
            return rendered
        if fallback is None:
            fallback = rendered
    return fallback


def _bound_cargo_diagnostic(value: str) -> str:
    normalized = value.replace("\r\n", "\n").replace("\r", "\n").strip()
    if len(normalized) <= _MAX_CARGO_DIAGNOSTIC_LENGTH:
        return normalized
    limit = _MAX_CARGO_DIAGNOSTIC_LENGTH - len(_CARGO_DIAGNOSTIC_TRUNCATION)
    return normalized[:limit].rstrip() + _CARGO_DIAGNOSTIC_TRUNCATION


def discovered_test_paths(ctx: Context, package: str, target: str) -> set[str] | None:
    key = (package, target)
    if key not in ctx.cargo_test_list_cache:
        discovered = cargo_test_list(ctx, package, target)
        if discovered is not None and not discovered:
            ctx.require(False, f"evidence target has no Cargo-discovered runnable tests: {package}/{target}")
        ctx.cargo_test_list_cache[key] = discovered
    return ctx.cargo_test_list_cache[key]


def exact_test_selection(ctx: Context, package: str, target: str, test_filter: str) -> set[str] | None:
    key = (package, target, test_filter)
    if key not in ctx.cargo_exact_selection_cache:
        ctx.cargo_exact_selection_cache[key] = cargo_test_list(ctx, package, target, test_filter)
    return ctx.cargo_exact_selection_cache[key]
