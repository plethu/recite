"""Installed-host identity, capability and keyboard evidence."""

from .commands import host_runner_clients, is_host_runner, validate_command
from .model import Context


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

    Host runners are separate from source/package/protocol commands. Each
    configured host must have a runnable scenario and explicit client coverage.
    """
    host_commands = [command for command in evidence_commands if is_host_runner(command)]
    records = evidence.get("host_records")
    if records is None and not host_commands:
        return
    if records is None:
        ctx.require(
            False, f"capability {capability_id} installed-host runner requires host_records"
        )
        return
    ctx.require(
        status in {"partial", "implemented"},
        f"capability {capability_id} host_records require partial or implemented capability status",
    )
    ctx.require(
        isinstance(records, list) and bool(records),
        f"capability {capability_id} host_records must be a non-empty array",
    )
    if not isinstance(records, list):
        return
    ctx.require(
        bool(host_commands),
        f"capability {capability_id} host_records require an installed-host evidence runner command",
    )
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
        product = record.get("product")
        version = record.get("version")
        architecture = record.get("architecture")
        ctx.require(
            isinstance(client, str) and client in client_map, f"{label} must name a known client"
        )
        ctx.require(
            isinstance(platform, str) and platform in ctx.platforms,
            f"{label} must name a known platform",
        )
        ctx.require(
            isinstance(runner, str) and is_host_runner(runner),
            f"{label} runner must be a scripts/check-*-host.sh path",
        )
        if isinstance(runner, str) and is_host_runner(runner):
            record_runners.append(runner)
            validate_command(ctx, capability_id, runner)
            runner_clients = host_runner_clients(runner)
            ctx.require(
                client in runner_clients, f"{label} runner {runner} does not match client {client}"
            )
        for field, value in (
            ("product", product),
            ("version", version),
            ("architecture", architecture),
        ):
            ctx.require(
                isinstance(value, str) and value.strip(), f"{label} must name a non-empty {field}"
            )
        if (
            isinstance(client, str)
            and isinstance(platform, str)
            and isinstance(version, str)
            and isinstance(architecture, str)
        ):
            key = (client, platform, version, architecture)
            ctx.require(
                key not in seen,
                f"{label} duplicates host identity {client}/{platform}/{version}/{architecture}",
            )
            seen.add(key)
        if isinstance(client, str) and isinstance(capability_clients, dict):
            client_status = capability_clients.get(client)
            ctx.require(
                client_status in {"partial", "implemented"},
                f"{label} names client {client} without claimed partial or implemented capability status",
            )
        if isinstance(platform, str) and isinstance(capability_platforms, dict):
            platform_status = capability_platforms.get(platform)
            ctx.require(
                platform_status in {"partial", "implemented"},
                f"{label} names platform {platform} without claimed partial or implemented capability status",
            )
        if keyboard:
            validate_keyboard_host_record(ctx, label, record)
    for command in host_commands:
        ctx.require(
            command in record_runners,
            f"capability {capability_id} installed-host runner {command} has no matching host record",
        )
    for runner in record_runners:
        ctx.require(
            runner in host_commands,
            f"capability {capability_id} host record runner {runner} is not listed in evidence commands",
        )
    if keyboard:
        if isinstance(capability_clients, dict):
            for client, client_status in capability_clients.items():
                if client_status in {"partial", "implemented"}:
                    ctx.require(
                        any(record.get("client") == client for record in valid_records),
                        f"capability {capability_id} host records do not cover claimed {client} client evidence",
                    )
        if isinstance(capability_platforms, dict):
            for platform, platform_status in capability_platforms.items():
                if platform_status in {"partial", "implemented"}:
                    ctx.require(
                        any(record.get("platform") == platform for record in valid_records),
                        f"capability {capability_id} host records do not cover claimed {platform} platform evidence",
                    )


def validate_keyboard_host_record(ctx: Context, label: str, record: dict) -> None:
    keyboard = record.get("keyboard")
    ctx.require(isinstance(keyboard, dict), f"{label} must name keyboard workflow assertions")
    if not isinstance(keyboard, dict):
        return
    sequence = keyboard.get("key_sequence")
    if isinstance(sequence, list):
        ctx.require(
            bool(sequence) and all(isinstance(value, str) and value.strip() for value in sequence),
            f"{label} key_sequence must contain non-empty strings",
        )
    else:
        ctx.require(
            isinstance(sequence, str) and sequence.strip(),
            f"{label} key_sequence must be a non-empty string or array",
        )
    for field in (
        "diagnostic_navigation",
        "textual_severity",
        "textual_status",
        "textual_failure",
        "clean_exit",
        "process_leak_check",
    ):
        ctx.require(keyboard.get(field) is True, f"{label} keyboard assertion {field} must be true")
    ctx.require(
        keyboard.get("watch_stop") in {"supported", "unsupported"},
        f"{label} keyboard watch_stop must be supported or unsupported",
    )
