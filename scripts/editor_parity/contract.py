import re
from pathlib import Path

from .evidence import validate_capabilities
from .model import Context
from .paths import require_no_symlink_components, require_repo_file


CANONICAL_FIXTURES = {
    "fixtures/recite/valid/language_pressure.recite",
    "fixtures/recite/valid/locale_fallback_fr.po",
    "fixtures/recite/valid/core_language_spike.recite",
    "fixtures/recite/invalid/parser_marker_leading_prose.recite",
    "fixtures/schema/valid/generated_manifest.json",
    "fixtures/schema/valid/full_manifest.json",
}


def unique_records(ctx: Context, key: str, records):
    if not isinstance(records, list):
        ctx.require(False, f"{key} must be an array")
        records = []
    ctx.require(bool(records), f"{key} must be a non-empty array")
    valid = [record for record in records if isinstance(record, dict)]
    ctx.require(len(valid) == len(records), f"{key} entries must be objects")
    ids = [record.get("id") for record in valid]
    string_ids = [record_id for record_id in ids if isinstance(record_id, str)]
    ctx.require(len(string_ids) == len(ids) and all(record_id.strip() for record_id in string_ids), f"{key} IDs must be non-empty strings")
    ctx.require(len(string_ids) == len(set(string_ids)), f"{key} IDs must be unique")
    return {record["id"]: record for record in valid if isinstance(record.get("id"), str) and record["id"].strip()}


def validate(ctx: Context, data: dict, document_path: Path) -> tuple[dict, dict, dict, dict, dict]:
    ctx.require(type(data.get("contract_version")) is int and data["contract_version"] == 1, "contract_version must be 1")
    status_values = data.get("status_values")
    ctx.require(isinstance(status_values, list) and set(status_values) == ctx.statuses and len(status_values) == len(ctx.statuses), "status_values must contain exactly the four contract statuses")
    scenario_map = unique_records(ctx, "scenarios", data.get("scenarios"))
    capability_map = unique_records(ctx, "capabilities", data.get("capabilities"))
    artifact_map = unique_records(ctx, "artifacts", data.get("artifacts"))
    distribution_map = unique_records(ctx, "distributions", data.get("distributions"))
    client_map = unique_records(ctx, "clients", data.get("clients"))
    ctx.require(set(client_map) == ctx.clients, "clients must contain exactly vscode, vscodium, neovim, and zed")
    validate_scenarios(ctx, scenario_map)
    validate_artifacts(ctx, artifact_map, client_map)
    validate_clients(ctx, client_map, artifact_map)
    validate_distributions(ctx, distribution_map, artifact_map)
    validate_capabilities(ctx, data, scenario_map, artifact_map, distribution_map, client_map)
    validate_neovim_topology(ctx, distribution_map, capability_map)
    validate_document(ctx, document_path, capability_map, scenario_map)
    return scenario_map, capability_map, artifact_map, distribution_map, client_map


def validate_scenarios(ctx: Context, scenarios: dict) -> None:
    for scenario_id, scenario in scenarios.items():
        paths = scenario.get("canonical_fixtures")
        ctx.require(isinstance(paths, list) and bool(paths), f"scenario {scenario_id} must name canonical_fixtures")
        for path in paths or []:
            ctx.require(isinstance(path, str), f"scenario {scenario_id} fixture path must be a string: {path!r}")
            if isinstance(path, str):
                ctx.require(path in CANONICAL_FIXTURES, f"scenario {scenario_id} references non-canonical fixture {path!r}")
                require_repo_file(ctx, path, f"scenario {scenario_id} fixture")
        ctx.require(scenario.get("status") in ctx.statuses, f"scenario {scenario_id} has invalid status")
        ctx.require(isinstance(scenario.get("derived_inputs"), list) and scenario["derived_inputs"], f"scenario {scenario_id} must describe derived inputs")
        ctx.require(isinstance(scenario.get("expected_evidence"), list) and scenario["expected_evidence"], f"scenario {scenario_id} must describe expected evidence")


def validate_artifacts(ctx: Context, artifacts: dict, clients: dict) -> None:
    for artifact_id, artifact in artifacts.items():
        ctx.require(artifact.get("status") in ctx.statuses, f"artifact {artifact_id} has invalid status")
        listed = artifact.get("clients")
        ctx.require(isinstance(listed, list) and bool(listed), f"artifact {artifact_id} must name clients")
        listed_ids = [client for client in listed if isinstance(client, str)] if isinstance(listed, list) else []
        ctx.require(len(listed_ids) == len(set(listed_ids)), f"artifact {artifact_id} client IDs must be unique")
        for client in listed or []:
            ctx.require(client in ctx.clients, f"artifact {artifact_id} names unknown client {client}")
            if client in clients:
                ctx.require(artifact_id in client_artifacts(clients[client]), f"artifact {artifact_id} and client {client} disagree on their underlying artifact")
        reciprocal = {client_id for client_id, client in clients.items() if artifact_id in client_artifacts(client)}
        ctx.require(set(listed_ids) == reciprocal, f"artifact {artifact_id} client list must exactly reciprocate client artifact references")
        path = artifact.get("path")
        if artifact.get("status") == "implemented":
            ctx.require(isinstance(path, str) and bool(path), f"implemented artifact {artifact_id} must name a path")
        if path is not None:
            valid_path = isinstance(path, str) and bool(path)
            artifact_path = Path(path) if valid_path else None
            ctx.require(valid_path and not artifact_path.is_absolute(), f"artifact {artifact_id} path must be a non-empty relative path")
            if artifact_path is not None:
                ctx.require(artifact_path.as_posix() == path and not {".", ".."}.intersection(artifact_path.parts), f"artifact {artifact_id} path must be normalized")
                candidate = ctx.repo_root / artifact_path
                require_no_symlink_components(ctx, candidate, f"artifact {artifact_id} path")
                resolved = candidate.resolve()
                ctx.require(ctx.repo_root in resolved.parents, f"artifact {artifact_id} path escapes the repository: {path}")
                ctx.require(resolved.is_file(), f"artifact {artifact_id} claims missing path {path}")


def client_artifacts(client: dict) -> list:
    return client["artifacts"] if "artifacts" in client else [client.get("artifact")]


def validate_clients(ctx: Context, clients: dict, artifacts: dict) -> None:
    for client_id, client in clients.items():
        ctx.require(client.get("status") in ctx.statuses, f"client {client_id} has invalid status")
        platform_status = client.get("platform_status") or {}
        ctx.require(isinstance(platform_status, dict) and set(platform_status) == ctx.platforms, f"client {client_id} must name exactly Linux, macOS, and Windows status")
        for platform, status in platform_status.items() if isinstance(platform_status, dict) else []:
            ctx.require(status in ctx.statuses, f"client {client_id} has invalid {platform} status")
            if client.get("status") == "planned":
                ctx.require(status in {"planned", "unsupported"}, f"planned client {client_id} cannot claim partial/implemented {platform} support")
        primary = client.get("artifact")
        ctx.require(primary in artifacts, f"client {client_id} references unknown artifact {primary}")
        supporting = client_artifacts(client)
        ctx.require(isinstance(supporting, list) and primary in supporting, f"client {client_id} artifacts must include its primary artifact")
        if isinstance(supporting, list):
            ctx.require(all(isinstance(value, str) for value in supporting), f"client {client_id} artifacts must be strings")
            ctx.require(len(supporting) == len(set(supporting)), f"client {client_id} artifacts must be unique")
            for value in supporting:
                ctx.require(value in artifacts, f"client {client_id} references unknown supporting artifact {value}")
                if value in artifacts:
                    ctx.require(client_id in artifacts[value].get("clients", []), f"client {client_id} artifact reference is not reciprocated by artifact {value}")
        if client.get("status") == "partial" and primary in artifacts:
            ctx.require(artifacts[primary].get("status") in {"partial", "implemented"}, f"partial client {client_id} needs a partial or implemented artifact")
        if client.get("status") == "implemented" and primary in artifacts:
            ctx.require(artifacts[primary].get("status") == "implemented", f"implemented client {client_id} needs an implemented artifact")
            ctx.require(any(value in {"partial", "implemented"} for value in platform_status.values()), f"implemented client {client_id} needs platform evidence")
    vscode_artifact = clients.get("vscode", {}).get("artifact")
    vscodium_artifact = clients.get("vscodium", {}).get("artifact")
    ctx.require(vscode_artifact == vscodium_artifact, "VS Code and VSCodium must share one VSIX artifact topology")
    if vscode_artifact in artifacts:
        ctx.require(set(artifacts[vscode_artifact].get("clients", [])) == {"vscode", "vscodium"}, "shared VS Code/VSCodium VSIX artifact must list exactly both clients")


def validate_distributions(ctx: Context, distributions: dict, artifacts: dict) -> None:
    for distribution_id, distribution in distributions.items():
        ctx.require(distribution.get("status") in ctx.statuses, f"distribution {distribution_id} has invalid status")
        primary = distribution.get("artifact")
        ctx.require(primary in artifacts, f"distribution {distribution_id} references unknown artifact")
        supporting = distribution["artifacts"] if "artifacts" in distribution else [primary]
        ctx.require(isinstance(supporting, list) and primary in supporting, f"distribution {distribution_id} artifacts must include its primary artifact")
        if isinstance(supporting, list):
            ctx.require(all(isinstance(value, str) for value in supporting), f"distribution {distribution_id} artifacts must be strings")
            ctx.require(len(supporting) == len(set(supporting)), f"distribution {distribution_id} artifacts must be unique")
            for value in supporting:
                ctx.require(value in artifacts, f"distribution {distribution_id} references unknown supporting artifact {value}")
        if distribution.get("status") in {"partial", "implemented"} and primary in artifacts:
            ctx.require(artifacts[primary].get("status") == "implemented", f"{distribution.get('status')} distribution {distribution_id} needs an implemented artifact")
def validate_neovim_topology(ctx: Context, distributions: dict, capabilities: dict) -> None:
    distribution = distributions.get("neovim-distribution")
    capability = capabilities.get("editor.neovim.syntax-projection")
    if not isinstance(distribution, dict) or not isinstance(capability, dict):
        return
    evidence = capability.get("expected_evidence") or {}
    capability_artifact = evidence.get("artifact")
    ctx.require(distribution.get("artifact") == capability_artifact, "Neovim distribution primary artifact must match its capability artifact")
    supporting = distribution.get("artifacts") or []
    ctx.require("tree-sitter-grammar" in supporting, "Neovim distribution supporting artifacts must include tree-sitter-grammar")


def validate_document(ctx: Context, document_path: Path, capabilities: dict, scenarios: dict) -> None:
    try:
        document = document_path.read_text(encoding="utf-8")
    except OSError as error:
        print(f"unable to read editor parity documentation: {error}", flush=True)
        raise SystemExit(2)
    doc_bullet_ids = set(re.findall(r"^[-*] `([^`]+)`:", document, flags=re.MULTILINE))
    ctx.require(doc_bullet_ids.intersection(capabilities) == set(capabilities), f"documentation capability IDs differ from fixture: docs={sorted(doc_bullet_ids.intersection(capabilities))} fixture={sorted(capabilities)}")
    ctx.require(doc_bullet_ids.issubset(set(capabilities) | set(scenarios)), f"documentation has unknown matrix IDs: {sorted(doc_bullet_ids - set(capabilities) - set(scenarios))}")
    for scenario_id in scenarios:
        ctx.require(f"`{scenario_id}`" in document, f"documentation does not mention scenario {scenario_id}")
    for capability_id in capabilities:
        ctx.require(f"`{capability_id}`" in document, f"documentation does not mention capability {capability_id}")
    filetype = capabilities.get("editor.filetype.registration", {})
    commands = (filetype.get("expected_evidence") or {}).get("commands") or []
    if "scripts/check-neovim.sh" in commands:
        limitation = str(filetype.get("known_limitation", "")).lower()
        ctx.require("no client package or activation" not in limitation, "Neovim filetype evidence cannot retain stale no-activation wording")
        ctx.require(filetype.get("platform_status", {}).get("linux") in {"partial", "implemented"}, "Neovim filetype evidence needs Linux support status")
