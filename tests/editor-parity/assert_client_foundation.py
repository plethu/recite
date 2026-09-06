#!/usr/bin/env python3
"""Assert the checked client evidence recorded by the editor parity contract."""

import json
import sys
from pathlib import Path


def evidence_artifacts(evidence: dict) -> set[str]:
    if "artifacts" in evidence:
        return set(evidence["artifacts"])
    if "artifact" in evidence:
        return {evidence["artifact"]}
    return set()


def main() -> int:
    if len(sys.argv) != 3:
        raise SystemExit("usage: assert_client_foundation.py CONTRACT DOCUMENT")
    contract_path, document_path = map(Path, sys.argv[1:])
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    clients = {client["id"]: client for client in contract["clients"]}
    artifacts = {artifact["id"]: artifact for artifact in contract["artifacts"]}

    for client_id in ("vscode", "vscodium"):
        client = clients[client_id]
        if client["status"] != "partial":
            raise SystemExit(f"{client_id} foundation must remain partial")
        if client["platform_status"] != {"linux": "partial", "macos": "planned", "windows": "planned"}:
            raise SystemExit(f"{client_id} foundation must claim Linux-only partial evidence")

    artifact = artifacts["vscode-vsix"]
    if artifact["status"] != "partial" or artifact["path"] is not None:
        raise SystemExit("VS Code artifact must remain a partial generated artifact, not checked-in archive")
    if "package-checked" not in artifact["notes"] or "ignored build output" not in artifact["notes"]:
        raise SystemExit("VS Code artifact notes must distinguish package checks from checked-in output")

    capabilities = {capability["id"]: capability for capability in contract["capabilities"]}
    expected_client_evidence = {
        "command.compile.validate.extract",
        "command.run.trace",
        "command.structured.results",
        "command.watch.lifecycle",
        "editor.filetype.registration",
        "editor.vscode.syntax-projection",
        "lsp.code-actions",
        "lsp.completion.navigation",
        "lsp.definition",
        "lsp.initialize.capabilities",
        "lsp.overlay.recovery",
        "lsp.publish.diagnostics",
        "lsp.rename",
        "lsp.utf16.positions",
        "workspace.configuration",
        "workspace.project.discovery",
    }
    actual_client_evidence = {
        capability_id
        for capability_id, capability in capabilities.items()
        if capability["client_status"]["vscode"] == "partial"
        and "scripts/check-vscode.sh" in capability["expected_evidence"].get("commands", [])
    }
    if actual_client_evidence != expected_client_evidence:
        raise SystemExit("VS Code partial client evidence rows drifted from the checked package/live surface")
    for capability_id in expected_client_evidence:
        capability = capabilities[capability_id]
        expected_follow_up = "#53" if capability_id.startswith("command.") else "#51"
        if capability["follow_up"] != expected_follow_up:
            raise SystemExit(f"{capability_id} must retain the open VS Code follow-up")
        if not capability_id.startswith("command.") and "vscode-vsix" not in evidence_artifacts(capability["expected_evidence"]):
            raise SystemExit(f"{capability_id} must attribute package/live evidence to vscode-vsix")

    expected_zed_evidence = {
        "lsp.completion",
        "lsp.completion.navigation",
        "lsp.definition",
        "lsp.hover",
        "lsp.initialize.capabilities",
        "lsp.publish.diagnostics",
        "lsp.references",
        "command.compile.validate.extract",
        "command.watch.lifecycle",
        "editor.filetype.registration",
        "editor.keyboard.workflow",
        "editor.zed.syntax-projection",
    }
    actual_zed_evidence = {
        capability_id
        for capability_id, capability in capabilities.items()
        if capability["client_status"].get("zed") == "partial"
        and "scripts/check-zed.sh" in capability["expected_evidence"].get("commands", [])
    }
    if actual_zed_evidence != expected_zed_evidence:
        raise SystemExit("Zed partial client evidence rows drifted from the checked package/static surface")
    for capability_id in expected_zed_evidence:
        if "zed-extension" not in evidence_artifacts(capabilities[capability_id]["expected_evidence"]):
            raise SystemExit(f"{capability_id} must attribute package/static evidence to zed-extension")
    zed_syntax = capabilities["editor.zed.syntax-projection"]
    if zed_syntax["follow_up"] != "#192":
        raise SystemExit("editor.zed.syntax-projection must retain the open Zed follow-up")
    if zed_syntax["client_status"].get("vscode") != "planned":
        raise SystemExit("editor.zed.syntax-projection must not project Zed evidence to VS Code")

    expected_zed_command_status = {
        "command.compile.validate.extract": "partial",
        "command.run.trace": "unsupported",
        "command.structured.results": "planned",
        "command.watch.lifecycle": "partial",
    }
    for capability_id, expected_status in expected_zed_command_status.items():
        actual_status = capabilities[capability_id]["client_status"].get("zed")
        if actual_status != expected_status:
            raise SystemExit(
                f"{capability_id} must retain Zed status {expected_status!r}, "
                f"not {actual_status!r}"
            )

    static_task_assertions = {
        "command.compile.validate.extract": (
            "zed only exposes explicit static terminal tasks and does not parse their output"
        ),
        "command.watch.lifecycle": (
            "zed checks only explicit static watch task argv; no parsed task controller is claimed"
        ),
    }
    for capability_id, assertion in static_task_assertions.items():
        assertions = capabilities[capability_id]["expected_evidence"].get("assertions", [])
        if assertion not in (value.lower() for value in assertions):
            raise SystemExit(
                f"{capability_id} must describe Zed as static task evidence without a parsed adapter"
            )

    if "installed vs code/vscodium activation smoke" not in document_path.read_text(encoding="utf-8").lower():
        raise SystemExit("editor parity docs must retain the missing host activation boundary")
    print("editor parity VS Code partial-foundation fixture passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
