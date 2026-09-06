import re
import shlex
from pathlib import Path

from .cargo_evidence import discovered_test_paths, exact_test_selection
from .model import Context, has_record
from .paths import require_no_symlink_components, require_repo_file


HOST_RUNNER_PATTERN = re.compile(r"scripts/check-[a-z0-9][a-z0-9-]*-host\.sh")
HOST_RECORD_DIRECTORY = Path("docs/evidence/editor-hosts")


def is_host_runner(command: object) -> bool:
    return isinstance(command, str) and HOST_RUNNER_PATTERN.fullmatch(command) is not None


def host_runner_client(command: str) -> str | None:
    if not is_host_runner(command):
        return None
    return command[len("scripts/check-") : -len("-host.sh")]


def validate_command(ctx: Context, capability_id: str, command: str) -> None:
    if command in {"scripts/check-tree-sitter.sh", "scripts/check-neovim.sh", "scripts/check-vscode.sh", "scripts/check-zed.sh"}:
        script, _ = require_repo_file(ctx, command, f"capability {capability_id} evidence script")
        if script.is_file():
            ctx.require(script.stat().st_mode & 0o111, f"capability {capability_id} evidence script is not executable: {command}")
        return
    if is_host_runner(command):
        script, _ = require_repo_file(ctx, command, f"capability {capability_id} installed-host evidence runner")
        if script.is_file():
            ctx.require(script.stat().st_mode & 0o111, f"capability {capability_id} installed-host evidence runner is not executable: {command}")
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
        if registered is None:
            return
        ctx.require(test_filter in registered, f"capability {capability_id} evidence command does not name an existing runnable test discovered by Cargo: {test_filter}")
        if test_filter not in registered:
            return
        selected = exact_test_selection(ctx, package, target, test_filter)
        if selected is None:
            return
        ctx.require(selected == {test_filter}, f"capability {capability_id} evidence command does not select exactly one Cargo test: {test_filter}")


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
        _validate_capability_evidence(ctx, capability_id, capability, status_kind, artifact_map, client_map)
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


def _validate_capability_evidence(ctx: Context, capability_id: str, capability: dict, status: str, artifact_map: dict, client_map: dict) -> None:
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
    validate_host_records(
        ctx,
        capability_id,
        capability,
        status,
        evidence,
        evidence_commands,
        client_map,
        keyboard=capability_id == "editor.keyboard.workflow",
    )
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


def validate_host_records(
    ctx: Context,
    capability_id: str,
    capability: dict,
    status: str,
    evidence: dict,
    evidence_commands: list,
    client_map: dict,
    *,
    keyboard: bool = False,
) -> None:
    """Validate optional installed-host evidence without changing old rows.

    Host runners are deliberately separate from the source/package/protocol
    evidence commands.  A runner is only meaningful when it has a checked-in
    record naming the exact host it exercised; the record carries the stronger
    keyboard assertions when the keyboard workflow uses it.
    """
    host_commands = [command for command in evidence_commands if is_host_runner(command)]
    records = evidence.get("host_records")
    if records is None and not host_commands:
        return
    if records is None:
        ctx.require(False, f"capability {capability_id} installed-host runner requires host_records")
        return
    ctx.require(status in {"partial", "implemented"}, f"capability {capability_id} host_records require partial or implemented capability status")
    ctx.require(isinstance(records, list) and bool(records), f"capability {capability_id} host_records must be a non-empty array")
    if not isinstance(records, list):
        return
    ctx.require(bool(host_commands), f"capability {capability_id} host_records require an installed-host evidence runner command")
    record_runners: list[str] = []
    valid_records: list[dict] = []
    seen: set[tuple[str, str, str, str]] = set()
    capability_clients = capability.get("client_status")
    capability_platforms = capability.get("platform_status")
    for index, record in enumerate(records):
        label = f"capability {capability_id} host record {index}"
        ctx.require(isinstance(record, dict), f"{label} must be an object")
        if not isinstance(record, dict):
            continue
        valid_records.append(record)
        client = record.get("client")
        platform = record.get("platform")
        runner = record.get("runner")
        document = record.get("record")
        product = record.get("product")
        version = record.get("version")
        architecture = record.get("architecture")
        ctx.require(isinstance(client, str) and client in client_map, f"{label} must name a known client")
        ctx.require(isinstance(platform, str) and platform in ctx.platforms, f"{label} must name a known platform")
        ctx.require(isinstance(runner, str) and is_host_runner(runner), f"{label} runner must be a scripts/check-*-host.sh path")
        if isinstance(runner, str) and is_host_runner(runner):
            record_runners.append(runner)
            validate_command(ctx, capability_id, runner)
            runner_client = host_runner_client(runner)
            ctx.require(runner_client == client, f"{label} runner {runner} does not match client {client}")
        ctx.require(isinstance(document, str) and bool(document), f"{label} must name an evidence document")
        if isinstance(document, str) and document:
            document_path = Path(document)
            ctx.require(
                not document_path.is_absolute()
                and document_path.as_posix() == document
                and document_path.parent == HOST_RECORD_DIRECTORY,
                f"{label} evidence document must be a direct path under {HOST_RECORD_DIRECTORY}",
            )
            if document_path.parent == HOST_RECORD_DIRECTORY and isinstance(client, str) and isinstance(platform, str):
                stem = document_path.stem.lower()
                ctx.require(client.lower() in stem and platform.lower() in stem, f"{label} evidence document must name {client} and {platform}")
            require_repo_file(ctx, document, f"{label} evidence document")
        for field, value in (("product", product), ("version", version), ("architecture", architecture)):
            ctx.require(isinstance(value, str) and value.strip(), f"{label} must name a non-empty {field}")
        if isinstance(client, str) and isinstance(platform, str) and isinstance(version, str) and isinstance(architecture, str):
            key = (client, platform, version, architecture)
            ctx.require(key not in seen, f"{label} duplicates host identity {client}/{platform}/{version}/{architecture}")
            seen.add(key)
        if isinstance(client, str) and isinstance(capability_clients, dict):
            client_status = capability_clients.get(client)
            ctx.require(client_status in {"partial", "implemented"}, f"{label} names client {client} without claimed partial or implemented capability status")
        if isinstance(platform, str) and isinstance(capability_platforms, dict):
            platform_status = capability_platforms.get(platform)
            ctx.require(platform_status in {"partial", "implemented"}, f"{label} names platform {platform} without claimed partial or implemented capability status")
        if keyboard:
            validate_keyboard_host_record(ctx, label, record)
    for command in host_commands:
        ctx.require(command in record_runners, f"capability {capability_id} installed-host runner {command} has no matching host record")
    for runner in record_runners:
        ctx.require(runner in host_commands, f"capability {capability_id} host record runner {runner} is not listed in evidence commands")
    if isinstance(capability_clients, dict):
        for client, client_status in capability_clients.items():
            if client_status in {"partial", "implemented"}:
                ctx.require(any(record.get("client") == client for record in valid_records), f"capability {capability_id} host records do not cover claimed {client} client evidence")
    if isinstance(capability_platforms, dict):
        for platform, platform_status in capability_platforms.items():
            if platform_status in {"partial", "implemented"}:
                ctx.require(any(record.get("platform") == platform for record in valid_records), f"capability {capability_id} host records do not cover claimed {platform} platform evidence")


def validate_keyboard_host_record(ctx: Context, label: str, record: dict) -> None:
    keyboard = record.get("keyboard")
    ctx.require(isinstance(keyboard, dict), f"{label} must name keyboard workflow assertions")
    if not isinstance(keyboard, dict):
        return
    sequence = keyboard.get("key_sequence")
    if isinstance(sequence, list):
        ctx.require(bool(sequence) and all(isinstance(value, str) and value.strip() for value in sequence), f"{label} key_sequence must contain non-empty strings")
    else:
        ctx.require(isinstance(sequence, str) and sequence.strip(), f"{label} key_sequence must be a non-empty string or array")
    for field in ("diagnostic_navigation", "textual_severity", "textual_status", "textual_failure", "clean_exit", "process_leak_check"):
        ctx.require(keyboard.get(field) is True, f"{label} keyboard assertion {field} must be true")
    ctx.require(keyboard.get("watch_stop") in {"supported", "unsupported"}, f"{label} keyboard watch_stop must be supported or unsupported")
