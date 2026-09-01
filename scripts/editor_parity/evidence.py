import json
import os
import re
import shlex
import subprocess
from pathlib import Path

from .content_digest import selected_target_digest
from .model import Context, has_record
from .paths import require_no_symlink_components, require_repo_file


_CARGO_TIMEOUT_SECONDS = 120
_MAX_CARGO_DIAGNOSTIC_LENGTH = 4096
_CARGO_DIAGNOSTIC_TRUNCATION = "... [truncated]"


def cargo_test_list(ctx: Context, package: str, target: str, test_filter: str | None = None) -> set[str]:
    executable = cargo_test_executable(ctx, package, target)
    if executable is None:
        return set()
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
        return set()
    except OSError as error:
        ctx.require(False, f"test harness discovery could not start for {package}/{target}: {error}")
        return set()
    if result.returncode != 0:
        output = (result.stdout + result.stderr).strip().splitlines()
        detail = output[-1] if output else "no cargo output"
        suffix = " exact selection" if test_filter is not None else " test list"
        ctx.require(False, f"test harness discovery failed for {package}/{target}{suffix}: {detail}")
        return set()
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


def discovered_test_paths(ctx: Context, package: str, target: str) -> set[str]:
    key = (package, target)
    if key not in ctx.cargo_test_list_cache:
        discovered = cargo_test_list(ctx, package, target)
        ctx.require(bool(discovered), f"evidence target has no Cargo-discovered runnable tests: {package}/{target}")
        ctx.cargo_test_list_cache[key] = discovered
    return ctx.cargo_test_list_cache[key]


def exact_test_selection(ctx: Context, package: str, target: str, test_filter: str) -> set[str]:
    key = (package, target, test_filter)
    if key not in ctx.cargo_exact_selection_cache:
        ctx.cargo_exact_selection_cache[key] = cargo_test_list(ctx, package, target, test_filter)
    return ctx.cargo_exact_selection_cache[key]


def validate_command(ctx: Context, capability_id: str, command: str) -> None:
    if command in {"scripts/check-tree-sitter.sh", "scripts/check-neovim.sh", "scripts/check-vscode.sh"}:
        script, _ = require_repo_file(ctx, command, f"capability {capability_id} evidence script")
        ctx.require(script.stat().st_mode & 0o111, f"capability {capability_id} evidence script is not executable: {command}")
        return
    try:
        parts = shlex.split(command)
    except ValueError as error:
        parts = []
        ctx.require(False, f"capability {capability_id} evidence command is malformed: {error}")
    valid_shape = len(parts) == 8 and parts[:2] == ["cargo", "test"] and parts[2:4] == ["--locked", "-p"] and parts[5] == "--test" and "--" not in parts
    ctx.require(valid_shape, f"capability {capability_id} evidence command must name a cargo integration test and filter")
    if not valid_shape:
        return
    package, target, test_filter = parts[4], parts[6], parts[7]
    test_file = ctx.repo_root / "crates" / package / "tests" / f"{target}.rs"
    require_no_symlink_components(ctx, test_file, f"capability {capability_id} evidence target")
    resolved_test_file = test_file.resolve()
    ctx.require(ctx.repo_root in resolved_test_file.parents, f"capability {capability_id} evidence target escapes the repository: {test_file}")
    ctx.require(test_file.is_file() and not test_file.is_symlink(), f"capability {capability_id} evidence target does not exist: {test_file.relative_to(ctx.repo_root) if test_file.is_relative_to(ctx.repo_root) else test_file}")
    if test_file.is_file() and not test_file.is_symlink():
        registered = discovered_test_paths(ctx, package, target)
        ctx.require(test_filter in registered, f"capability {capability_id} evidence command does not name an existing runnable test discovered by Cargo: {test_filter}")
        ctx.require(exact_test_selection(ctx, package, target, test_filter) == {test_filter}, f"capability {capability_id} evidence command does not select exactly one Cargo test: {test_filter}")


def validate_capabilities(ctx: Context, data: dict, scenario_map: dict, artifact_map: dict, distribution_map: dict, client_map: dict) -> None:
    for capability in data["capabilities"] if isinstance(data.get("capabilities"), list) else []:
        capability_id = capability.get("id") if isinstance(capability, dict) else None
        if not isinstance(capability_id, str):
            continue
        ctx.require(re.fullmatch(r"[a-z][a-z0-9]*(?:\.[a-z0-9-]+)+", capability_id) is not None, f"capability ID is not stable lowercase dotted form: {capability_id!r}")
        ctx.require(has_record(scenario_map, capability.get("scenario")), f"capability {capability_id} references unknown scenario")
        ctx.require(isinstance(capability.get("authority"), list) and capability["authority"], f"capability {capability_id} must name semantic authority")
        ctx.require(isinstance(capability.get("protocol"), str) and capability.get("protocol") in {"lsp", "protocol-neutral", "cli", "client"}, f"capability {capability_id} has invalid protocol")
        expected = capability.get("expected") or {}
        ctx.require(isinstance(expected, dict) and expected.get("kind"), f"capability {capability_id} must name expected structured result")
        ctx.require(isinstance(expected, dict) and isinstance(expected.get("assertions"), list) and expected["assertions"], f"capability {capability_id} must name expected assertions")
        ctx.require(isinstance(capability.get("edge_cases"), list) and capability["edge_cases"], f"capability {capability_id} must name edge cases")
        limitation = capability.get("known_limitation")
        ctx.require(isinstance(limitation, str) and limitation.strip(), f"capability {capability_id} must name a known_limitation")
        status = capability.get("implementation_status")
        status_kind = status if isinstance(status, str) else None
        ctx.require(ctx.valid_status(status), f"capability {capability_id} has invalid implementation status")
        if isinstance(limitation, str) and limitation.strip() == "none":
            ctx.require(status_kind == "implemented", f"capability {capability_id} may use known_limitation=none only when fully implemented")
        _validate_capability_status(ctx, capability_id, capability, status_kind, artifact_map, distribution_map, client_map)
        _validate_capability_evidence(ctx, capability_id, capability, status_kind, artifact_map)
        follow_up = capability.get("follow_up", "")
        ctx.require(isinstance(follow_up, str) and re.fullmatch(r"#[1-9][0-9]*", follow_up) is not None, f"capability {capability_id} must name a follow-up issue")


def _validate_capability_status(ctx: Context, capability_id: str, capability: dict, status: str, artifact_map: dict, distribution_map: dict, client_map: dict) -> None:
    client_status = capability.get("client_status") or {}
    ctx.require(isinstance(client_status, dict) and set(client_status) == ctx.clients, f"capability {capability_id} must name every client exactly once")
    for client_id, value in client_status.items() if isinstance(client_status, dict) else []:
        ctx.require(ctx.valid_status(value), f"capability {capability_id} has invalid {client_id} status")
        if value == "implemented" and client_id in client_map:
            ctx.require(client_map[client_id].get("status") == "implemented", f"capability {capability_id} overstates implemented support for {client_id}")
        if isinstance(value, str) and value in {"partial", "implemented"} and client_id in client_map:
            client_status_value = client_map[client_id].get("status")
            ctx.require(isinstance(client_status_value, str) and client_status_value in {"partial", "implemented"}, f"capability {capability_id} overstates {client_id} while its client remains planned")
    platform_status = capability.get("platform_status") or {}
    ctx.require(isinstance(platform_status, dict) and set(platform_status) == ctx.platforms, f"capability {capability_id} must name every platform exactly once")
    for platform, value in platform_status.items() if isinstance(platform_status, dict) else []:
        ctx.require(ctx.valid_status(value), f"capability {capability_id} has invalid {platform} status")
        if isinstance(value, str) and status in {"planned", "unsupported"}:
            ctx.require(value in {"planned", "unsupported"}, f"{status} capability {capability_id} cannot claim {platform} platform status {value}")
        elif isinstance(value, str) and status == "partial":
            ctx.require(value in {"planned", "partial", "unsupported"}, f"partial capability {capability_id} cannot claim {platform} platform status {value}")
    for key, expected, label in [("artifact_status", set(artifact_map), "artifact"), ("distribution_status", set(distribution_map), "distribution")]:
        values = capability.get(key) or {}
        ctx.require(isinstance(values, dict) and set(values) == expected, f"capability {capability_id} must name every {label} status exactly once")
        for record_id, value in values.items() if isinstance(values, dict) else []:
            ctx.require(ctx.valid_status(value), f"capability {capability_id} has invalid {record_id} {label} status")
            records = artifact_map if label == "artifact" else distribution_map
            if record_id in records:
                ctx.require(value == records[record_id].get("status"), f"capability {capability_id} {label} status for {record_id} disagrees with its {label} record")


def _validate_capability_evidence(ctx: Context, capability_id: str, capability: dict, status: str, artifact_map: dict) -> None:
    evidence = capability.get("expected_evidence") or {}
    ctx.require(isinstance(evidence, dict), f"capability {capability_id} expected_evidence must be an object")
    if not isinstance(evidence, dict):
        return
    evidence_status = evidence.get("status")
    ctx.require(ctx.valid_status(evidence_status), f"capability {capability_id} has invalid evidence status")
    ctx.require(isinstance(evidence.get("assertions"), list) and evidence["assertions"], f"capability {capability_id} must name evidence assertions")
    command, commands = evidence.get("command"), evidence.get("commands")
    ctx.require(not (command is not None and commands is not None), f"capability {capability_id} must use either command or commands, not both")
    evidence_commands = commands if isinstance(commands, list) else ([command] if command is not None else [])
    if commands is not None:
        ctx.require(isinstance(commands, list) and bool(commands), f"capability {capability_id} commands must be a non-empty array")
    if status in {"implemented", "partial"}:
        ctx.require(evidence_commands and all(isinstance(value, str) and value for value in evidence_commands), f"implemented/partial capability {capability_id} must name evidence command(s)")
        ctx.require(isinstance(evidence_status, str) and evidence_status in {"implemented", "partial"}, f"implemented/partial capability {capability_id} needs implemented or partial evidence")
    if status in {"planned", "unsupported"}:
        ctx.require(isinstance(evidence_status, str) and evidence_status in {"planned", "unsupported"}, f"{status} capability {capability_id} cannot claim evidence status {evidence_status}")
        ctx.require(command is None and commands is None, f"unimplemented capability {capability_id} must not claim an executable evidence command")
    if status == "partial":
        ctx.require(isinstance(evidence_status, str) and evidence_status in {"planned", "partial"}, f"partial capability {capability_id} cannot claim implemented evidence")
    if status == "implemented":
        ctx.require(evidence_status == "implemented", f"implemented capability {capability_id} needs implemented evidence")
    for value in evidence_commands:
        if isinstance(value, str) and value:
            validate_command(ctx, capability_id, value)
    artifact, artifacts = evidence.get("artifact"), evidence.get("artifacts")
    ctx.require(not (artifact is not None and artifacts is not None), f"capability {capability_id} must use either artifact or artifacts, not both")
    if artifacts is not None:
        ctx.require(isinstance(artifacts, list) and bool(artifacts), f"capability {capability_id} artifacts must be a non-empty array")
        if isinstance(artifacts, list):
            ctx.require(all(isinstance(value, str) and value for value in artifacts), f"capability {capability_id} evidence artifacts must be non-empty strings")
            strings_only = all(isinstance(value, str) and value for value in artifacts)
            ctx.require(strings_only and len(artifacts) == len(set(artifacts)), f"capability {capability_id} evidence artifacts must be unique")
            for value in artifacts:
                ctx.require(has_record(artifact_map, value), f"capability {capability_id} references unknown evidence artifact {value}")
    elif artifact is not None:
        ctx.require(has_record(artifact_map, artifact), f"capability {capability_id} references unknown evidence artifact {artifact}")
