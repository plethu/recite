import re

from .commands import validate_command
from .host_evidence import validate_host_records
from .model import Context, has_record

ISSUE_REFERENCE_PATTERN = re.compile(r"#[1-9][0-9]*")
HISTORICAL_EVIDENCE_ISSUES = frozenset({"#51", "#53", "#98", "#192", "#202"})
CURRENT_FOLLOW_UP_OWNERS = {"#206": "lsp.cancellation"}


def validate_capabilities(
    ctx: Context,
    data: dict,
    scenario_map: dict,
    artifact_map: dict,
    distribution_map: dict,
    client_map: dict,
) -> None:
    for capability in data["capabilities"] if isinstance(data.get("capabilities"), list) else []:
        capability_id = capability.get("id") if isinstance(capability, dict) else None
        if not isinstance(capability_id, str):
            continue
        ctx.require(
            re.fullmatch(r"[a-z][a-z0-9]*(?:\.[a-z0-9-]+)+", capability_id) is not None,
            f"capability ID is not stable lowercase dotted form: {capability_id!r}",
        )
        ctx.require(
            has_record(scenario_map, capability.get("scenario")),
            f"capability {capability_id} references unknown scenario",
        )
        ctx.require(
            isinstance(capability.get("authority"), list) and capability["authority"],
            f"capability {capability_id} must name semantic authority",
        )
        ctx.require(
            isinstance(capability.get("protocol"), str)
            and capability.get("protocol") in {"lsp", "protocol-neutral", "cli", "client"},
            f"capability {capability_id} has invalid protocol",
        )
        expected = capability.get("expected") or {}
        ctx.require(
            isinstance(expected, dict) and expected.get("kind"),
            f"capability {capability_id} must name expected structured result",
        )
        ctx.require(
            isinstance(expected, dict)
            and isinstance(expected.get("assertions"), list)
            and expected["assertions"],
            f"capability {capability_id} must name expected assertions",
        )
        ctx.require(
            isinstance(capability.get("edge_cases"), list) and capability["edge_cases"],
            f"capability {capability_id} must name edge cases",
        )
        limitation = capability.get("known_limitation")
        ctx.require(
            isinstance(limitation, str) and limitation.strip(),
            f"capability {capability_id} must name a known_limitation",
        )
        status = capability.get("implementation_status")
        status_kind = status if isinstance(status, str) else None
        ctx.require(
            ctx.valid_status(status),
            f"capability {capability_id} has invalid implementation status",
        )
        if isinstance(limitation, str) and limitation.strip() == "none":
            ctx.require(
                status_kind == "implemented",
                f"capability {capability_id} may use known_limitation=none only when fully implemented",
            )
        _validate_capability_status(
            ctx, capability_id, capability, status_kind, artifact_map, distribution_map, client_map
        )
        _validate_capability_evidence(
            ctx, capability_id, capability, status_kind, artifact_map, client_map
        )
        validate_issue_provenance(ctx, capability_id, capability)


def validate_issue_provenance(ctx: Context, capability_id: str, capability: dict) -> None:
    evidence_issues = capability.get("evidence_issues")
    valid_evidence_issues = (
        isinstance(evidence_issues, list)
        and bool(evidence_issues)
        and all(
            isinstance(issue, str) and ISSUE_REFERENCE_PATTERN.fullmatch(issue)
            for issue in evidence_issues
        )
    )
    ctx.require(
        valid_evidence_issues,
        f"capability {capability_id} must name non-empty historical evidence_issues",
    )
    if isinstance(evidence_issues, list):
        string_issues = [issue for issue in evidence_issues if isinstance(issue, str)]
        if len(string_issues) == len(evidence_issues):
            ctx.require(
                len(evidence_issues) == len(set(evidence_issues)),
                f"capability {capability_id} evidence_issues must be unique",
            )
            for issue in string_issues:
                ctx.require(
                    issue in HISTORICAL_EVIDENCE_ISSUES,
                    f"capability {capability_id} evidence_issues must name closed historical issues, not {issue}",
                )
    follow_up = capability.get("follow_up")
    if follow_up is None:
        return
    valid_follow_up = (
        isinstance(follow_up, str) and ISSUE_REFERENCE_PATTERN.fullmatch(follow_up) is not None
    )
    ctx.require(
        valid_follow_up,
        f"capability {capability_id} follow_up must be a valid issue reference when present",
    )
    if not valid_follow_up:
        return
    ctx.require(
        follow_up not in HISTORICAL_EVIDENCE_ISSUES,
        f"capability {capability_id} follow_up {follow_up} is historical; use evidence_issues",
    )
    owner = CURRENT_FOLLOW_UP_OWNERS.get(follow_up)
    ctx.require(
        owner == capability_id,
        f"capability {capability_id} follow_up {follow_up} belongs to {owner or 'no current capability owner'}",
    )
    if isinstance(evidence_issues, list):
        ctx.require(
            follow_up not in evidence_issues,
            f"capability {capability_id} follow_up must not duplicate evidence_issues",
        )


def _validate_capability_status(
    ctx: Context,
    capability_id: str,
    capability: dict,
    status: str,
    artifact_map: dict,
    distribution_map: dict,
    client_map: dict,
) -> None:
    client_status = capability.get("client_status") or {}
    ctx.require(
        isinstance(client_status, dict) and set(client_status) == ctx.clients,
        f"capability {capability_id} must name every client exactly once",
    )
    for client_id, value in client_status.items() if isinstance(client_status, dict) else []:
        ctx.require(
            ctx.valid_status(value), f"capability {capability_id} has invalid {client_id} status"
        )
        if value == "implemented" and client_id in client_map:
            ctx.require(
                client_map[client_id].get("status") == "implemented",
                f"capability {capability_id} overstates implemented support for {client_id}",
            )
        if (
            isinstance(value, str)
            and value in {"partial", "implemented"}
            and client_id in client_map
        ):
            client_status_value = client_map[client_id].get("status")
            ctx.require(
                isinstance(client_status_value, str)
                and client_status_value in {"partial", "implemented"},
                f"capability {capability_id} overstates {client_id} while its client remains planned",
            )
    if status in {"planned", "unsupported"} and isinstance(client_status, dict):
        for client_id, value in client_status.items():
            ctx.require(
                isinstance(value, str) and value in {"planned", "unsupported"},
                f"{status} capability {capability_id} cannot claim {client_id} status {value}",
            )
    platform_status = capability.get("platform_status") or {}
    ctx.require(
        isinstance(platform_status, dict) and set(platform_status) == ctx.platforms,
        f"capability {capability_id} must name every platform exactly once",
    )
    for platform, value in platform_status.items() if isinstance(platform_status, dict) else []:
        ctx.require(
            ctx.valid_status(value), f"capability {capability_id} has invalid {platform} status"
        )
        if isinstance(value, str) and status in {"planned", "unsupported"}:
            ctx.require(
                value in {"planned", "unsupported"},
                f"{status} capability {capability_id} cannot claim {platform} platform status {value}",
            )
        elif isinstance(value, str) and status == "partial":
            ctx.require(
                value in {"planned", "partial", "unsupported"},
                f"partial capability {capability_id} cannot claim {platform} platform status {value}",
            )
    for key, expected, label in [
        ("artifact_status", set(artifact_map), "artifact"),
        ("distribution_status", set(distribution_map), "distribution"),
    ]:
        values = capability.get(key) or {}
        ctx.require(
            isinstance(values, dict) and set(values) == expected,
            f"capability {capability_id} must name every {label} status exactly once",
        )
        for record_id, value in values.items() if isinstance(values, dict) else []:
            ctx.require(
                ctx.valid_status(value),
                f"capability {capability_id} has invalid {record_id} {label} status",
            )
            records = artifact_map if label == "artifact" else distribution_map
            if record_id in records:
                ctx.require(
                    value == records[record_id].get("status"),
                    f"capability {capability_id} {label} status for {record_id} disagrees with its {label} record",
                )


def _validate_capability_evidence(
    ctx: Context,
    capability_id: str,
    capability: dict,
    status: str,
    artifact_map: dict,
    client_map: dict,
) -> None:
    evidence = capability.get("expected_evidence") or {}
    ctx.require(
        isinstance(evidence, dict),
        f"capability {capability_id} expected_evidence must be an object",
    )
    if not isinstance(evidence, dict):
        return
    evidence_status = evidence.get("status")
    ctx.require(
        ctx.valid_status(evidence_status), f"capability {capability_id} has invalid evidence status"
    )
    ctx.require(
        isinstance(evidence.get("assertions"), list) and evidence["assertions"],
        f"capability {capability_id} must name evidence assertions",
    )
    command, commands = evidence.get("command"), evidence.get("commands")
    ctx.require(
        not (command is not None and commands is not None),
        f"capability {capability_id} must use either command or commands, not both",
    )
    evidence_commands = (
        commands if isinstance(commands, list) else ([command] if command is not None else [])
    )
    if commands is not None:
        ctx.require(
            isinstance(commands, list) and bool(commands),
            f"capability {capability_id} commands must be a non-empty array",
        )
    if status in {"implemented", "partial"}:
        ctx.require(
            evidence_commands
            and all(isinstance(value, str) and value for value in evidence_commands),
            f"implemented/partial capability {capability_id} must name evidence command(s)",
        )
        ctx.require(
            isinstance(evidence_status, str) and evidence_status in {"implemented", "partial"},
            f"implemented/partial capability {capability_id} needs implemented or partial evidence",
        )
    if status in {"planned", "unsupported"}:
        ctx.require(
            isinstance(evidence_status, str) and evidence_status in {"planned", "unsupported"},
            f"{status} capability {capability_id} cannot claim evidence status {evidence_status}",
        )
        ctx.require(
            command is None and commands is None,
            f"unimplemented capability {capability_id} must not claim an executable evidence command",
        )
    if status == "partial":
        ctx.require(
            isinstance(evidence_status, str) and evidence_status in {"planned", "partial"},
            f"partial capability {capability_id} cannot claim implemented evidence",
        )
    if status == "implemented":
        ctx.require(
            evidence_status == "implemented",
            f"implemented capability {capability_id} needs implemented evidence",
        )
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
    ctx.require(
        not (artifact is not None and artifacts is not None),
        f"capability {capability_id} must use either artifact or artifacts, not both",
    )
    if artifacts is not None:
        ctx.require(
            isinstance(artifacts, list) and bool(artifacts),
            f"capability {capability_id} artifacts must be a non-empty array",
        )
        if isinstance(artifacts, list):
            ctx.require(
                all(isinstance(value, str) and value for value in artifacts),
                f"capability {capability_id} evidence artifacts must be non-empty strings",
            )
            strings_only = all(isinstance(value, str) and value for value in artifacts)
            ctx.require(
                strings_only and len(artifacts) == len(set(artifacts)),
                f"capability {capability_id} evidence artifacts must be unique",
            )
            for value in artifacts:
                ctx.require(
                    has_record(artifact_map, value),
                    f"capability {capability_id} references unknown evidence artifact {value}",
                )
    elif artifact is not None:
        ctx.require(
            has_record(artifact_map, artifact),
            f"capability {capability_id} references unknown evidence artifact {artifact}",
        )
