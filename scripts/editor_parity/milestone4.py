"""Validators for the bounded Milestone 4 editor parity contract."""

from .evidence import is_host_runner
from .model import Context


KEYBOARD_CAPABILITY_ID = "editor.keyboard.workflow"
KEYBOARD_EVIDENCE_ISSUE = "#202"
CANCELLATION_CAPABILITY_ID = "lsp.cancellation"
CANCELLATION_FOLLOW_UP = "#206"
ZED_HOST_EVIDENCE_ISSUE = "#192"
ZED_HOST_CAPABILITY_ASSERTIONS = {
    "lsp.utf16.positions": "post-emoji utf-16 completion request",
    "lsp.code-actions": "non-empty missing-id quick fix",
    "lsp.rename": "exact two-edit workspace rename",
    "authoring.stable-id.operations": "exact canonical missing-id repair",
}


def validate_reconciliation_document(ctx: Context, document: str, capabilities: dict) -> None:
    """Check the bounded M4 reconciliation prose against capability status.

    Keep these checks inside the reconciliation section so unrelated design
    prose can discuss the same host limitations without becoming a fragile
    global wording contract.
    """
    heading = "## Milestone 4 reconciliation"
    end_heading = "## Reopening conditions"
    start = document.find(heading)
    end = document.find(end_heading, start + len(heading)) if start >= 0 else -1
    ctx.require(start >= 0 and end > start, "editor parity documentation must contain a bounded Milestone 4 reconciliation section")
    if start < 0 or end <= start:
        return
    section = " ".join(document[start:end].lower().split())

    def require_phrase(phrase: str, reason: str) -> None:
        ctx.require(phrase in section, f"Milestone 4 reconciliation must retain {reason}")

    command_capability = capabilities.get("command.compile.validate.extract", {})
    command_client_status = command_capability.get("client_status") if isinstance(command_capability, dict) else None
    if isinstance(command_client_status, dict) and command_client_status.get("zed") == "partial":
        require_phrase("zed does not parse task records", "the Zed task-diagnostics limitation")
        require_phrase("native task cancellation controller", "the Zed native-cancellation limitation")
    run_trace = capabilities.get("command.run.trace", {})
    run_trace_client_status = run_trace.get("client_status") if isinstance(run_trace, dict) else None
    if isinstance(run_trace_client_status, dict) and run_trace_client_status.get("zed") == "unsupported":
        require_phrase("built-in run/trace remain unclaimed", "the unsupported Zed built-in run/trace boundary")
    stale_version = capabilities.get("lsp.stale.version", {})
    stale_evidence = stale_version.get("expected_evidence", {}) if isinstance(stale_version, dict) else {}
    stale_commands = stale_evidence.get("commands", []) if isinstance(stale_evidence, dict) else []
    if not any(is_host_runner(command) for command in stale_commands):
        require_phrase("stale-version rejection remains a lower-level test boundary", "the lower-level stale-didchange boundary")
    cancellation = capabilities.get(CANCELLATION_CAPABILITY_ID, {})
    if isinstance(cancellation, dict) and cancellation.get("implementation_status") == "unsupported":
        require_phrase("lsp request cancellation remains unsupported", "the unsupported LSP cancellation status")
        require_phrase("#206", "the LSP cancellation owner")
        require_phrase("serious-v1 scheduler/performance capability", "the serious-v1 scheduler/performance scope")


def validate_keyboard_capability(ctx: Context, capabilities: dict, scenarios: dict) -> None:
    capability = capabilities.get(KEYBOARD_CAPABILITY_ID)
    ctx.require(isinstance(capability, dict), f"contract must contain {KEYBOARD_CAPABILITY_ID}")
    if not isinstance(capability, dict):
        return
    scenario_id = capability.get("scenario")
    ctx.require(scenario_id == "keyboard-workflow", f"{KEYBOARD_CAPABILITY_ID} must use the keyboard-workflow scenario")
    scenario = scenarios.get(scenario_id)
    capability_status = capability.get("implementation_status")
    if capability_status == "planned":
        if isinstance(scenario, dict):
            ctx.require(scenario.get("status") == "planned", f"keyboard-workflow scenario must remain planned until installed-host evidence exists")
    elif capability_status in {"partial", "implemented"}:
        if isinstance(scenario, dict):
            ctx.require(scenario.get("status") in {"partial", "implemented"}, f"{KEYBOARD_CAPABILITY_ID} partial/implemented status requires a partial/implemented keyboard-workflow scenario")
        evidence = capability.get("expected_evidence")
        evidence_commands = []
        if isinstance(evidence, dict):
            commands = evidence.get("commands")
            command = evidence.get("command")
            evidence_commands = commands if isinstance(commands, list) else ([command] if isinstance(command, str) else [])
            host_records = evidence.get("host_records")
            ctx.require(isinstance(host_records, list) and bool(host_records), f"{KEYBOARD_CAPABILITY_ID} partial/implemented status requires host_records")
        ctx.require(any(is_host_runner(command) for command in evidence_commands), f"{KEYBOARD_CAPABILITY_ID} partial/implemented status requires an installed-host evidence runner")
    evidence_issues = capability.get("evidence_issues")
    ctx.require(
        isinstance(evidence_issues, list) and KEYBOARD_EVIDENCE_ISSUE in evidence_issues,
        f"{KEYBOARD_CAPABILITY_ID} must retain historical evidence issue {KEYBOARD_EVIDENCE_ISSUE}",
    )
    ctx.require(
        capability.get("follow_up") is None,
        f"{KEYBOARD_CAPABILITY_ID} closed evidence issue {KEYBOARD_EVIDENCE_ISSUE} must not remain a current follow_up",
    )
    if capability_status == "planned":
        ctx.require(capability.get("implementation_status") == "planned", f"{KEYBOARD_CAPABILITY_ID} must remain planned until installed-host evidence exists")
    client_status = capability.get("client_status")
    if capability_status == "planned" and isinstance(client_status, dict):
        for client_id, client_status_value in client_status.items():
            ctx.require(isinstance(client_status_value, str) and client_status_value in {"planned", "unsupported"}, f"{KEYBOARD_CAPABILITY_ID} cannot claim {client_id} host evidence before {KEYBOARD_EVIDENCE_ISSUE}")
    platform_status = capability.get("platform_status")
    if capability_status == "planned" and isinstance(platform_status, dict):
        for platform, platform_status_value in platform_status.items():
            ctx.require(isinstance(platform_status_value, str) and platform_status_value in {"planned", "unsupported"}, f"{KEYBOARD_CAPABILITY_ID} cannot claim {platform} host evidence before {KEYBOARD_EVIDENCE_ISSUE}")
    evidence = capability.get("expected_evidence")
    if capability_status == "planned" and isinstance(evidence, dict):
        ctx.require(evidence.get("status") == "planned", f"{KEYBOARD_CAPABILITY_ID} must not claim executable evidence before {KEYBOARD_EVIDENCE_ISSUE}")
        ctx.require("command" not in evidence and "commands" not in evidence, f"{KEYBOARD_CAPABILITY_ID} must not reuse package/source/headless commands as keyboard evidence")
    limitation = str(capability.get("known_limitation", "")).lower()
    for boundary in ("installed-host", "package", "source", "headless"):
        ctx.require(boundary in limitation, f"{KEYBOARD_CAPABILITY_ID} known_limitation must name the {boundary} evidence boundary")
    sequence_scope = str(capability.get("keyboard_sequence_scope", "")).lower()
    for phrase in ("dedicated lsp ui action sequence", "scripts/check-zed-host.sh", "lsp.code-actions", "lsp.rename"):
        ctx.require(
            phrase in sequence_scope,
            f"{KEYBOARD_CAPABILITY_ID} keyboard_sequence_scope must explain {phrase!r}",
        )


def validate_zed_host_contract(ctx: Context, capabilities: dict) -> None:
    """Keep the accepted Zed host proofs attached to their contract rows.

    The source/package gate and its hostile fixture checker run independently
    of the installed-host runner.  These assertions make a later status,
    runner, or evidence-row edit fail closed instead of silently reducing the
    Milestone 4 proof back to the old unsupported claims.
    """
    for capability_id, assertion_fragment in ZED_HOST_CAPABILITY_ASSERTIONS.items():
        capability = capabilities.get(capability_id)
        ctx.require(isinstance(capability, dict), f"contract must contain {capability_id} for accepted Zed host evidence")
        if not isinstance(capability, dict):
            continue
        client_status = capability.get("client_status")
        ctx.require(
            isinstance(client_status, dict) and client_status.get("zed") == "partial",
            f"{capability_id} must retain partial Zed host evidence",
        )
        evidence = capability.get("expected_evidence")
        ctx.require(isinstance(evidence, dict), f"{capability_id} must retain Zed host evidence details")
        if not isinstance(evidence, dict):
            continue
        commands = evidence.get("commands")
        ctx.require(
            isinstance(commands, list) and "scripts/check-zed-host.sh" in commands,
            f"{capability_id} must retain scripts/check-zed-host.sh evidence",
        )
        records = evidence.get("host_records")
        ctx.require(
            isinstance(records, list)
            and any(
                isinstance(record, dict)
                and record.get("client") == "zed"
                and record.get("platform") == "linux"
                for record in records
            ),
            f"{capability_id} must retain an installed Zed Linux host record",
        )
        assertions = evidence.get("assertions")
        assertion_text = " ".join(value.lower() for value in assertions if isinstance(value, str)) if isinstance(assertions, list) else ""
        ctx.require(
            assertion_fragment in assertion_text,
            f"{capability_id} must retain Zed assertion {assertion_fragment!r}",
        )
    for capability_id, capability in capabilities.items():
        if not isinstance(capability, dict):
            continue
        evidence = capability.get("expected_evidence")
        records = evidence.get("host_records") if isinstance(evidence, dict) else None
        has_zed_host_record = isinstance(records, list) and any(
            isinstance(record, dict)
            and record.get("client") == "zed"
            for record in records
        )
        if not has_zed_host_record:
            continue
        evidence_issues = capability.get("evidence_issues")
        ctx.require(
            isinstance(evidence_issues, list) and ZED_HOST_EVIDENCE_ISSUE in evidence_issues,
            f"{capability_id} must retain historical evidence issue {ZED_HOST_EVIDENCE_ISSUE} for Zed host evidence",
        )


def validate_cancellation_contract(ctx: Context, capabilities: dict) -> None:
    capability = capabilities.get(CANCELLATION_CAPABILITY_ID)
    ctx.require(isinstance(capability, dict), f"contract must contain {CANCELLATION_CAPABILITY_ID}")
    if not isinstance(capability, dict):
        return
    ctx.require(
        capability.get("implementation_status") == "unsupported",
        f"{CANCELLATION_CAPABILITY_ID} implementation_status must remain unsupported until {CANCELLATION_FOLLOW_UP}",
    )
    evidence = capability.get("expected_evidence")
    ctx.require(
        isinstance(evidence, dict) and evidence.get("status") == "unsupported",
        f"{CANCELLATION_CAPABILITY_ID} expected_evidence.status must remain unsupported until {CANCELLATION_FOLLOW_UP}",
    )
    follow_up = capability.get("follow_up")
    ctx.require(
        follow_up == CANCELLATION_FOLLOW_UP,
        f"{CANCELLATION_CAPABILITY_ID} must remain owned by serious-v1 follow-up {CANCELLATION_FOLLOW_UP}",
    )
    evidence_issues = capability.get("evidence_issues")
    ctx.require(
        isinstance(evidence_issues, list) and "#53" in evidence_issues,
        f"{CANCELLATION_CAPABILITY_ID} must retain historical evidence issue #53",
    )
    limitation = str(capability.get("known_limitation", "")).lower()
    for phrase in ("serious-v1", "scheduler", "performance", "not m4 command/watch work"):
        ctx.require(
            phrase in limitation,
            f"{CANCELLATION_CAPABILITY_ID} limitation must retain {phrase!r}",
        )
