#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-editor-parity.sh [repo-root]

Validates the editor parity contract, canonical fixture references, and honest
status/artifact claims shared by the documentation and JSON matrix.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi

if [[ $# -gt 1 ]]; then
  usage >&2
  exit 2
fi

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  repo_root="$(git -C "$input_root" rev-parse --show-toplevel)"
else
  repo_root="$(git rev-parse --show-toplevel)"
fi

fixture="$repo_root/fixtures/editor-parity/contract.json"
document="$repo_root/docs/editor-parity-contract.md"
[[ -f "$fixture" ]] || { echo "missing editor parity fixture: $fixture" >&2; exit 2; }
[[ -f "$document" ]] || { echo "missing editor parity contract: $document" >&2; exit 2; }

python3 - "$repo_root" "$fixture" "$document" <<'PY'
import json
import re
import sys
from pathlib import Path

repo_root = Path(sys.argv[1])
fixture_path = Path(sys.argv[2])
document_path = Path(sys.argv[3])
errors = []

def require(condition, message):
    if not condition:
        errors.append(message)

try:
    data = json.loads(fixture_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as error:
    print(f"unable to read editor parity contract: {error}", file=sys.stderr)
    raise SystemExit(2)
if not isinstance(data, dict):
    print("editor parity contract must contain a JSON object", file=sys.stderr)
    raise SystemExit(2)

statuses = {"planned", "partial", "implemented", "unsupported"}
clients = {"vscode", "vscodium", "neovim", "zed"}
platforms = {"linux", "macos", "windows"}
require(type(data.get("contract_version")) is int and data["contract_version"] == 1, "contract_version must be 1")
status_values = data.get("status_values")
require(isinstance(status_values, list) and set(status_values) == statuses and len(status_values) == len(statuses), "status_values must contain exactly the four contract statuses")

def unique_records(key, records):
    if not isinstance(records, list):
        require(False, f"{key} must be an array")
        records = []
    require(bool(records), f"{key} must be a non-empty array")
    valid = [record for record in records if isinstance(record, dict)]
    require(len(valid) == len(records), f"{key} entries must be objects")
    ids = [record.get("id") for record in valid]
    string_ids = [record_id for record_id in ids if isinstance(record_id, str)]
    require(len(string_ids) == len(ids) and all(record_id.strip() for record_id in string_ids), f"{key} IDs must be non-empty strings")
    require(len(string_ids) == len(set(string_ids)), f"{key} IDs must be unique")
    return {record["id"]: record for record in valid if isinstance(record.get("id"), str) and record["id"].strip()}

scenario_map = unique_records("scenarios", data.get("scenarios"))
capability_map = unique_records("capabilities", data.get("capabilities"))
artifact_map = unique_records("artifacts", data.get("artifacts"))
distribution_map = unique_records("distributions", data.get("distributions"))
client_map = unique_records("clients", data.get("clients"))
require(set(client_map) == clients, "clients must contain exactly vscode, vscodium, neovim, and zed")

canonical_allowlist = {
    "fixtures/recite/valid/language_pressure.recite",
    "fixtures/recite/valid/core_language_spike.recite",
    "fixtures/recite/invalid/parser_marker_leading_prose.recite",
    "fixtures/schema/valid/generated_manifest.json",
    "fixtures/schema/valid/full_manifest.json",
}
for scenario_id, scenario in scenario_map.items():
    paths = scenario.get("canonical_fixtures")
    require(isinstance(paths, list) and paths, f"scenario {scenario_id} must name canonical_fixtures")
    for path in paths or []:
        require(isinstance(path, str), f"scenario {scenario_id} fixture path must be a string: {path!r}")
        if isinstance(path, str):
            require(path in canonical_allowlist, f"scenario {scenario_id} references non-canonical fixture {path!r}")
            require((repo_root / path).is_file(), f"scenario {scenario_id} fixture does not exist: {path}")
    require(scenario.get("status") in statuses, f"scenario {scenario_id} has invalid status")
    require(isinstance(scenario.get("derived_inputs"), list) and scenario["derived_inputs"], f"scenario {scenario_id} must describe derived inputs")
    require(isinstance(scenario.get("expected_evidence"), list) and scenario["expected_evidence"], f"scenario {scenario_id} must describe expected evidence")

referenced_scenarios = {
    capability.get("scenario")
    for capability in capability_map.values()
    if isinstance(capability.get("scenario"), str)
}
require(set(scenario_map) == referenced_scenarios, f"every scenario must be referenced by a capability: orphaned={sorted(set(scenario_map) - referenced_scenarios)}")

for artifact_id, artifact in artifact_map.items():
    require(artifact.get("status") in statuses, f"artifact {artifact_id} has invalid status")
    require(isinstance(artifact.get("clients"), list) and artifact["clients"], f"artifact {artifact_id} must name clients")
    for client in artifact.get("clients", []):
        require(client in clients, f"artifact {artifact_id} names unknown client {client}")
        if client in client_map:
            require(client_map[client].get("artifact") == artifact_id, f"artifact {artifact_id} and client {client} disagree on their underlying artifact")
    path = artifact.get("path")
    if artifact.get("status") == "implemented":
        require(isinstance(path, str) and path, f"implemented artifact {artifact_id} must name a path")
    if path is not None:
        artifact_path = Path(path) if isinstance(path, str) else None
        require(artifact_path is not None and bool(path) and not artifact_path.is_absolute(), f"artifact {artifact_id} path must be a non-empty relative path")
        if artifact_path is not None and path:
            require(artifact_path.as_posix() == path and not {".", ".."}.intersection(artifact_path.parts), f"artifact {artifact_id} path must be normalized")
            resolved_path = (repo_root / artifact_path).resolve()
            resolved_root = repo_root.resolve()
            require(resolved_root in resolved_path.parents, f"artifact {artifact_id} path escapes the repository: {path}")
            require(resolved_path.is_file(), f"artifact {artifact_id} claims missing path {path}")

for client_id, client in client_map.items():
    require(client_id in clients, f"unknown client ID {client_id}")
    require(client.get("status") in statuses, f"client {client_id} has invalid status")
    platform_status = client.get("platform_status") or {}
    require(isinstance(platform_status, dict), f"client {client_id} platform_status must be an object")
    if not isinstance(platform_status, dict):
        platform_status = {}
    require(set(platform_status) == platforms, f"client {client_id} must name exactly Linux, macOS, and Windows status")
    for platform, status in platform_status.items():
        require(status in statuses, f"client {client_id} has invalid {platform} status")
        if client.get("status") == "planned":
            require(status in {"planned", "unsupported"}, f"planned client {client_id} cannot claim partial/implemented {platform} support")
    artifact = client.get("artifact")
    require(artifact in artifact_map, f"client {client_id} references unknown artifact {artifact}")
    if client.get("status") in {"partial", "implemented"} and artifact in artifact_map:
        require(artifact_map[artifact].get("status") == "implemented", f"{client.get('status')} client {client_id} needs an implemented artifact")
    if client.get("status") == "implemented":
        require(artifact_map[artifact].get("status") == "implemented", f"implemented client {client_id} needs an implemented artifact")
        require(any(status in {"partial", "implemented"} for status in platform_status.values()), f"implemented client {client_id} needs platform evidence")

for distribution_id, distribution in distribution_map.items():
    require(distribution.get("status") in statuses, f"distribution {distribution_id} has invalid status")
    artifact = distribution.get("artifact")
    require(artifact in artifact_map, f"distribution {distribution_id} references unknown artifact")
    if distribution.get("status") in {"partial", "implemented"} and artifact in artifact_map:
        require(artifact_map[artifact].get("status") == "implemented", f"{distribution.get('status')} distribution {distribution_id} needs an implemented artifact")

for capability_id, capability in capability_map.items():
    require(re.fullmatch(r"[a-z][a-z0-9]*(?:\.[a-z0-9-]+)+", capability_id or "") is not None, f"capability ID is not stable lowercase dotted form: {capability_id!r}")
    require(capability.get("scenario") in scenario_map, f"capability {capability_id} references unknown scenario")
    require(isinstance(capability.get("authority"), list) and capability["authority"], f"capability {capability_id} must name semantic authority")
    require(capability.get("protocol") in {"lsp", "protocol-neutral", "cli", "client"}, f"capability {capability_id} has invalid protocol")
    expected = capability.get("expected") or {}
    require(isinstance(expected, dict), f"capability {capability_id} expected must be an object")
    if not isinstance(expected, dict):
        expected = {}
    require(isinstance(expected, dict) and expected.get("kind"), f"capability {capability_id} must name expected structured result")
    require(isinstance(expected.get("assertions"), list) and expected["assertions"], f"capability {capability_id} must name expected assertions")
    require(isinstance(capability.get("edge_cases"), list) and capability["edge_cases"], f"capability {capability_id} must name edge cases")
    status = capability.get("implementation_status")
    require(status in statuses, f"capability {capability_id} has invalid implementation status")
    client_status = capability.get("client_status") or {}
    require(isinstance(client_status, dict), f"capability {capability_id} client_status must be an object")
    if not isinstance(client_status, dict):
        client_status = {}
    require(set(client_status) == clients, f"capability {capability_id} must name every client exactly once")
    for client_id, client_status_value in client_status.items():
        require(client_status_value in statuses, f"capability {capability_id} has invalid {client_id} status")
        if client_id in client_map and client_status_value == "implemented":
            require(client_map[client_id].get("status") == "implemented", f"capability {capability_id} overstates implemented support for {client_id}")
    platform_status = capability.get("platform_status") or {}
    require(isinstance(platform_status, dict), f"capability {capability_id} platform_status must be an object")
    if not isinstance(platform_status, dict):
        platform_status = {}
    require(set(platform_status) == platforms, f"capability {capability_id} must name every platform exactly once")
    for platform, platform_status_value in platform_status.items():
        require(platform_status_value in statuses, f"capability {capability_id} has invalid {platform} status")
    artifact_status = capability.get("artifact_status") or {}
    require(isinstance(artifact_status, dict), f"capability {capability_id} artifact_status must be an object")
    if not isinstance(artifact_status, dict):
        artifact_status = {}
    require(set(artifact_status) == set(artifact_map), f"capability {capability_id} must name every artifact status exactly once")
    for artifact_id, artifact_status_value in artifact_status.items():
        require(artifact_status_value in statuses, f"capability {capability_id} has invalid {artifact_id} artifact status")
    distribution_status = capability.get("distribution_status") or {}
    require(isinstance(distribution_status, dict), f"capability {capability_id} distribution_status must be an object")
    if not isinstance(distribution_status, dict):
        distribution_status = {}
    require(set(distribution_status) == set(distribution_map), f"capability {capability_id} must name every distribution status exactly once")
    for distribution_id, distribution_status_value in distribution_status.items():
        require(distribution_status_value in statuses, f"capability {capability_id} has invalid {distribution_id} distribution status")
        if distribution_id in distribution_map:
            require(distribution_status_value == distribution_map[distribution_id].get("status"), f"capability {capability_id} distribution status for {distribution_id} disagrees with its distribution record")
    evidence = capability.get("expected_evidence") or {}
    require(isinstance(evidence, dict), f"capability {capability_id} expected_evidence must be an object")
    if not isinstance(evidence, dict):
        evidence = {}
    require(evidence.get("status") in statuses, f"capability {capability_id} has invalid evidence status")
    require(isinstance(evidence.get("assertions"), list) and evidence["assertions"], f"capability {capability_id} must name evidence assertions")
    if status in {"implemented", "partial"}:
        require(isinstance(evidence.get("command"), str) and evidence["command"], f"implemented/partial capability {capability_id} must name an evidence command")
        require(evidence.get("status") in {"implemented", "partial"}, f"implemented/partial capability {capability_id} needs implemented or partial evidence")
    if status == "implemented":
        require(evidence.get("status") == "implemented", f"implemented capability {capability_id} needs implemented evidence")
    if status in {"planned", "unsupported"}:
        require(evidence.get("command") is None, f"unimplemented capability {capability_id} must not claim an executable evidence command")
    artifact = evidence.get("artifact")
    if artifact is not None:
        require(artifact in artifact_map, f"capability {capability_id} references unknown evidence artifact {artifact}")
    for artifact_id, artifact_status_value in artifact_status.items():
        if artifact_id in artifact_map:
            require(artifact_status_value == artifact_map[artifact_id].get("status"), f"capability {capability_id} artifact status for {artifact_id} disagrees with its artifact record")
    for client_id, client_status_value in client_status.items():
        if client_id in client_map and client_status_value in {"partial", "implemented"}:
            require(client_map[client_id].get("status") in {"partial", "implemented"}, f"capability {capability_id} overstates {client_id} while its client remains planned")
    require(re.fullmatch(r"#[1-9][0-9]*", capability.get("follow_up", "")) is not None, f"capability {capability_id} must name a follow-up issue")

try:
    document = document_path.read_text(encoding="utf-8")
except OSError as error:
    print(f"unable to read editor parity documentation: {error}", file=sys.stderr)
    raise SystemExit(2)

doc_bullet_ids = set(re.findall(r"^[-*] `([^`]+)`:", document, flags=re.MULTILINE))
doc_capability_ids = doc_bullet_ids.intersection(capability_map)
fixture_capability_ids = set(capability_map)
require(doc_capability_ids == fixture_capability_ids, f"documentation capability IDs differ from fixture: docs={sorted(doc_capability_ids)} fixture={sorted(fixture_capability_ids)}")
require(doc_bullet_ids.issubset(fixture_capability_ids | set(scenario_map)), f"documentation has unknown matrix IDs: {sorted(doc_bullet_ids - fixture_capability_ids - set(scenario_map))}")
for scenario_id in scenario_map:
    require(f"`{scenario_id}`" in document, f"documentation does not mention scenario {scenario_id}")
for capability_id in capability_map:
    require(f"`{capability_id}`" in document, f"documentation does not mention capability {capability_id}")

if errors:
    for error in errors:
        print(f"editor parity contract: {error}", file=sys.stderr)
    raise SystemExit(1)

print(f"Editor parity contract passed: {len(capability_map)} capabilities, {len(scenario_map)} scenarios, {len(client_map)} clients, {len(artifact_map)} artifacts.")
PY
