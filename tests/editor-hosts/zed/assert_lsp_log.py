#!/usr/bin/env python3
"""Assert payloads captured from the real Zed/Recite LSP session."""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path
from typing import Any


def records(path: Path) -> list[dict[str, Any]]:
    # The proxy may append one final JSON record while the host is still
    # running. Retry only an incomplete trailing record; a malformed earlier
    # record remains a hard evidence failure.
    for _ in range(20):
        lines = path.read_text(encoding="utf-8").splitlines()
        parsed: list[dict[str, Any]] = []
        try:
            for index, line in enumerate(lines):
                parsed.append(json.loads(line))
        except json.JSONDecodeError:
            if index == len(lines) - 1:
                time.sleep(0.1)
                continue
            raise
        return parsed
    raise AssertionError("LSP transport log remained incomplete after bounded retry")


def has_label(value: Any, label: str) -> bool:
    if isinstance(value, dict):
        return value.get("label") == label or any(has_label(item, label) for item in value.values())
    if isinstance(value, list):
        return any(has_label(item, label) for item in value)
    return False


def is_unsupported_empty_code_action_result(result: Any) -> bool:
    """Return whether the host produced exactly the documented empty result."""
    return type(result) is list and not result


def main() -> int:
    if len(sys.argv) != 3:
        raise SystemExit("usage: assert_lsp_log.py LOG PROJECT_DIR")
    entries = records(Path(sys.argv[1]))
    project = Path(sys.argv[2]).as_uri()

    requests = {
        entry["message"].get("id"): entry["message"]
        for entry in entries
        if entry["direction"] == "client->server" and "id" in entry["message"]
    }
    responses = {
        entry["message"].get("id"): entry["message"]
        for entry in entries
        if entry["direction"] == "server->client" and "id" in entry["message"]
    }

    def response_results(method: str) -> list[Any]:
        return [
            responses[request_id].get("result")
            for request_id, request in requests.items()
            if request.get("method") == method and request_id in responses
        ]

    initialize = next(
        (
            message
            for message in responses.values()
            if message.get("result", {}).get("serverInfo", {}).get("name") == "recite-lsp"
        ),
        None,
    )
    if initialize is None:
        raise AssertionError("no Recite initialize response was captured")
    capabilities = initialize["result"]["capabilities"]
    assert capabilities["positionEncoding"] == "utf-16"
    assert capabilities["textDocumentSync"] == {"change": 1, "openClose": True, "save": {}}
    for capability in ("completionProvider", "hoverProvider", "definitionProvider", "referencesProvider"):
        assert capabilities.get(capability), f"missing initialize capability: {capability}"
    assert capabilities["renameProvider"]["prepareProvider"] is True
    assert capabilities["codeActionProvider"]["codeActionKinds"]

    diagnostics = [
        message["params"]
        for entry in entries
        if entry["direction"] == "server->client"
        for message in [entry["message"]]
        if message.get("method") == "textDocument/publishDiagnostics"
        and message.get("params", {}).get("uri") == f"{project}/fixture.recite"
        and {item.get("code") for item in message["params"].get("diagnostics", [])}
        == {"RECITE_PARSE011", "RECITE_PARSE013"}
    ]
    assert diagnostics, "canonical malformed fixture diagnostics were not captured"
    latest = diagnostics[-1]
    assert latest.get("version") is not None
    by_code = {item["code"]: item for item in latest["diagnostics"]}
    assert by_code["RECITE_PARSE011"]["severity"] == 1
    assert by_code["RECITE_PARSE011"]["range"] == {
        "start": {"line": 2, "character": 11},
        "end": {"line": 2, "character": 13},
    }
    assert by_code["RECITE_PARSE013"]["severity"] == 1
    assert by_code["RECITE_PARSE013"]["range"] == {
        "start": {"line": 3, "character": 11},
        "end": {"line": 3, "character": 13},
    }

    assert any(has_label(result, "work") for result in response_results("textDocument/completion")), \
        "no Zed-triggered completion response contained the canonical work symbol"
    assert any(
        isinstance(result, dict)
        and result.get("uri", "").endswith("/core.recite")
        and result.get("range", {}).get("start") == {"line": 13, "character": 3}
        for result in response_results("textDocument/definition")
    ), "no canonical definition response was captured from Zed"
    assert any(
        isinstance(result, dict)
        and result.get("range", {}).get("start", {}).get("line") == 6
        for result in response_results("textDocument/hover")
    ), "no canonical hover response was captured from Zed"
    assert any(
        isinstance(result, list)
        and len(result) == 2
        and [item.get("range", {}).get("start", {}).get("line") for item in result] == [13, 6]
        for result in response_results("textDocument/references")
    ), "no canonical references response was captured from Zed"
    assert any(
        isinstance(result, dict) and result.get("placeholder") == "work"
        for result in response_results("textDocument/prepareRename")
    ), "no canonical prepare-rename response was captured from Zed"
    code_action_uri = f"{project}/code-action.recite"
    code_action_transactions = [
        (request, responses[request_id].get("result"))
        for request_id, request in requests.items()
        if request.get("method") == "textDocument/codeAction"
        and request.get("params", {}).get("textDocument", {}).get("uri") == code_action_uri
        and request_id in responses
    ]
    assert code_action_transactions, "Zed did not receive a code-action response for the canonical fixture"
    assert any(
        request.get("params", {}).get("range") == {
            "start": {"line": 1, "character": 0},
            "end": {"line": 1, "character": 1},
        }
        and any(
            diagnostic.get("code") == "RECITE_ID001"
            for diagnostic in request.get("params", {}).get("context", {}).get("diagnostics", [])
            if isinstance(diagnostic, dict)
        )
        for request, _ in code_action_transactions
    ), "Zed did not send the canonical missing-ID code-action range and diagnostic"
    assert all(
        is_unsupported_empty_code_action_result(result)
        for _, result in code_action_transactions
    ), "Zed code-action result changed; update the documented host boundary before claiming success"
    print("lsp_transport=actual_zed_requests_and_recite_responses_asserted")
    print("lsp_diagnostics=RECITE_PARSE011/013 severity=1 UTF-16 ranges asserted")
    print("lsp_features=completion/hover/definition/references/prepareRename asserted")
    print("lsp_code_action=unsupported_empty_result(request_crossed_zed; no_edit_applied)")
    print("lsp_rename_edit=unsupported_in_this_key_sequence(rename requires host text-entry confirmation)")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, KeyError, TypeError) as error:
        print(f"LSP evidence assertion failed: {error}", file=sys.stderr)
        raise SystemExit(1)
