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


def has_canonical_missing_id_quick_fix(result: Any) -> bool:
    """Return whether a response contains Recite's canonical missing-ID fix."""
    if not isinstance(result, list):
        return False
    return any(
        isinstance(action, dict)
        and action.get("title") == "Insert missing stable ID"
        and action.get("kind") == "quickfix"
        and bool(action.get("edit", {}).get("documentChanges"))
        for action in result
    )


def main() -> int:
    if len(sys.argv) != 3:
        raise SystemExit("usage: assert_lsp_log.py LOG PROJECT_DIR")
    entries = records(Path(sys.argv[1]))
    project_path = Path(sys.argv[2])
    project = project_path.as_uri()

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

    fixture_uri = f"{project}/host-fixtures/fixture.recite"
    diagnostics = [
        message["params"]
        for entry in entries
        if entry["direction"] == "server->client"
        for message in [entry["message"]]
        if message.get("method") == "textDocument/publishDiagnostics"
        and message.get("params", {}).get("uri") == fixture_uri
        and {item.get("code") for item in message["params"].get("diagnostics", [])}
        == {"RECITE_PARSE011", "RECITE_PARSE013"}
    ]
    assert diagnostics, "canonical malformed fixture diagnostics were not captured"
    fixture_texts = [
        entry["message"].get("params", {}).get("textDocument", {}).get("text", "")
        for entry in entries
        if entry["direction"] == "client->server"
        and entry["message"].get("method") == "textDocument/didOpen"
        and entry["message"].get("params", {}).get("textDocument", {}).get("uri") == fixture_uri
    ]
    assert any(
        len(text.splitlines()) > 2 and text.splitlines()[2][11] == "😀"
        for text in fixture_texts
    ), "installed Zed did not send the non-BMP marker at the asserted source column"
    assert any(
        entry["direction"] == "client->server"
        and entry["message"].get("method") == "textDocument/didOpen"
        and entry["message"].get("params", {}).get("textDocument", {}).get("uri") == fixture_uri
        and "😀" in entry["message"].get("params", {}).get("textDocument", {}).get("text", "")
        for entry in entries
    ), "installed Zed did not send the non-BMP fixture text over didOpen"
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
    assert any(
        request.get("method") == "textDocument/completion"
        and request.get("params", {}).get("textDocument", {}).get("uri") == fixture_uri
        and request.get("params", {}).get("position") == {"line": 2, "character": 14}
        for request in requests.values()
    ), "Zed did not send a completion request at the post-marker UTF-16 position"

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
    missing_id_transactions = [
        (request, result)
        for request, result in code_action_transactions
        if any(
            diagnostic.get("code") == "RECITE_ID001"
            for diagnostic in request.get("params", {}).get("context", {}).get("diagnostics", [])
            if isinstance(diagnostic, dict)
        )
    ]
    assert missing_id_transactions, "Zed did not send a missing-ID code-action request"
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
        for request, _ in missing_id_transactions
    ), "Zed did not send the canonical missing-ID code-action range and diagnostic"
    actions = [
        action
        for _, result in missing_id_transactions
        if isinstance(result, list)
        for action in result
        if isinstance(action, dict)
        and action.get("title") == "Insert missing stable ID"
    ]
    assert actions, "Recite returned no canonical missing-ID quick-fix through Zed"
    action = actions[0]
    assert action.get("kind") == "quickfix"
    assert any(
        diagnostic.get("code") == "RECITE_ID001"
        for diagnostic in action.get("diagnostics", [])
        if isinstance(diagnostic, dict)
    ), "canonical Zed quick-fix omitted RECITE_ID001"
    changes = action.get("edit", {}).get("documentChanges", [])
    action_change = next(
        (
            change
            for change in changes
            if change.get("textDocument", {}).get("uri") == f"{project}/code-action.recite"
            and change.get("edits")
        ),
        None,
    )
    assert action_change is not None, "canonical Zed quick-fix omitted its document edit"
    assert action_change["textDocument"].get("version") == 0
    assert action_change["edits"] == [
        {
            "range": {
                "start": {"line": 1, "character": 1},
                "end": {"line": 1, "character": 1},
            },
            "newText": action_change["edits"][0]["newText"],
        }
    ]
    new_text = action_change["edits"][0]["newText"]
    assert new_text == " line@56d52d8cd8619971011f"
    applied_code_action = (project_path / "code-action.recite").read_text(encoding="utf-8")
    assert applied_code_action.splitlines()[1] == f">{new_text}"
    rename_transactions = [
        (request, responses[request_id].get("result"))
        for request_id, request in requests.items()
        if request.get("method") == "textDocument/rename"
        and request_id in responses
    ]
    rename_request, rename_result = next(
        (
            transaction
            for transaction in rename_transactions
            if transaction[0].get("params", {}).get("newName") == "work_renamed"
        ),
        (None, None),
    )
    assert rename_request is not None, "Zed did not send the replacement-name rename request"
    assert rename_request["params"]["position"] == {"line": 13, "character": 3}
    rename_changes = (rename_result or {}).get("documentChanges", [])
    assert len(rename_changes) == 1
    assert rename_changes[0]["textDocument"] == {
        "uri": f"{project}/core.recite",
        "version": 0,
    }
    assert rename_changes[0]["edits"] == [
        {
            "range": {
                "start": {"line": 6, "character": 7},
                "end": {"line": 6, "character": 11},
            },
            "newText": "work_renamed",
        },
        {
            "range": {
                "start": {"line": 13, "character": 3},
                "end": {"line": 13, "character": 7},
            },
            "newText": "work_renamed",
        },
    ]
    print("lsp_transport=actual_zed_requests_and_recite_responses_asserted")
    print("lsp_diagnostics=RECITE_PARSE011/013 severity=1 non-BMP UTF-16 ranges asserted")
    print("lsp_utf16=non_BMP_fixture_didOpen_and_post_marker_request_asserted")
    print("lsp_features=completion/hover/definition/references/prepareRename asserted")
    print("lsp_code_action=non_empty_canonical_quick_fix_response_and_edit_asserted")
    print("lsp_rename_edit=work_renamed_two_occurrence_workspace_edit_asserted")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, KeyError, TypeError) as error:
        print(f"LSP evidence assertion failed: {error}", file=sys.stderr)
        raise SystemExit(1)
