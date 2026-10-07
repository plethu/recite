#!/usr/bin/env python3
"""Replay sustained typing and competing requests through an owned stdio server."""

import argparse
import hashlib
import json
import math
import queue
import tempfile
import time
from pathlib import Path

from . import client as probe
from .measurement import provenance


def memory(pid):
    path = Path(f"/proc/{pid}/status")
    if not path.exists():
        return None
    return {
        line.split(":")[0]: int(line.split()[1])
        for line in path.read_text().splitlines()
        if line.startswith(("VmRSS:", "VmHWM:"))
    }


def run(
    binary, root, edits, interval_ms, ranged=False, burst_every=0, pause_ms=150, recovery_ms=500
):
    source = sorted((root / "src").glob("*.recite"))[0]
    text, uri = source.read_text(), source.as_uri()
    declaration = probe.position(text, r"^:: (block_00001)\b")
    reference = probe.position(text, r"-> (block_\d+)")

    def at(pos):
        return {"textDocument": {"uri": uri}, "position": pos}

    operations = {
        "completion": ("textDocument/completion", at(reference)),
        "definition": ("textDocument/definition", at(reference)),
        "rename": ("textDocument/rename", {**at(declaration), "newName": "session_renamed"}),
        "fix_all": (
            "textDocument/codeAction",
            {
                "textDocument": {"uri": uri},
                "range": {"start": declaration, "end": declaration},
                "context": {"diagnostics": [], "only": ["source.fixAll"]},
            },
        ),
    }
    result = {
        "source_bytes": len(text.encode()),
        "edits": edits,
        "interval_ms": interval_ms,
        "ranged": ranged,
        "requests": [],
        "diagnostics": [],
        "send_lateness_ms": [],
        "memory": [],
        "max_client_outstanding_requests": 0,
        "recoveries": [],
        "burst_every": burst_every,
        "pause_ms": pause_ms,
        "recovery_budget_ms": recovery_ms,
    }
    pending, sent = {}, {}
    eof = {
        "line": text.count("\n"),
        "character": len(text.rsplit("\n", 1)[-1].encode("utf-16-le")) // 2,
    }
    end = eof
    version = 1
    with tempfile.TemporaryDirectory(prefix="recite-lsp-session-") as config:
        client = probe.Client(binary, Path(config))
        failed = False
        try:
            client.request("initialize", {"rootUri": root.as_uri(), "capabilities": {}})
            client.send("initialized", {})
            client.send(
                "textDocument/didOpen",
                {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "recite",
                        "version": version,
                        "text": text,
                    }
                },
            )
            client.diagnostics(uri, version)
            client.notifications.clear()

            def request(name, cancel=False):
                request_id, started = client.send(*operations[name], request=True)
                pending[request_id] = (name, started, cancel)
                if cancel:
                    client.send("$/cancelRequest", {"id": request_id})
                result["max_client_outstanding_requests"] = max(
                    result["max_client_outstanding_requests"], len(pending)
                )

            def receive(item):
                if isinstance(item, Exception):
                    raise item
                received, message = item
                if "id" in message:
                    name, started, cancelled = pending.pop(message["id"])
                    code = message.get("error", {}).get("code")
                    assert code in (None, -32800, -32803), message
                    if code == -32803:
                        assert message["error"].get("data", {}).get("reason") == "stale_snapshot", (
                            message
                        )
                    if code is not None:
                        assert "result" not in message, message
                    elif name != "fix_all":
                        assert message.get("result"), message
                    result["requests"].append(
                        {
                            "operation": name,
                            "ms": (received - started) / 1e6,
                            "cancel_sent": cancelled,
                            "error_code": code,
                        }
                    )
                elif message.get("method") == "textDocument/publishDiagnostics":
                    params = message["params"]
                    published = params.get("version")
                    if params["uri"] == uri and published in sent:
                        assert not params["diagnostics"], params
                        assert (
                            not result["diagnostics"]
                            or published > result["diagnostics"][-1]["version"]
                        )
                        result["diagnostics"].append(
                            {
                                "version": published,
                                "ms": (received - sent[published]) / 1e6,
                                "newer_edits_sent": sum(
                                    t <= received for v, t in sent.items() if v > published
                                ),
                            }
                        )

            def settle(timeout):
                deadline = time.monotonic() + timeout
                while (
                    pending
                    or not result["diagnostics"]
                    or result["diagnostics"][-1]["version"] != version
                ):
                    remaining = deadline - time.monotonic()
                    assert remaining > 0, "session failed to settle within budget"
                    receive(client.messages.get(timeout=remaining))

            start = time.perf_counter_ns()
            pause_total = 0
            for index in range(edits):
                if burst_every and index and index % burst_every == 0:
                    pause_started = time.perf_counter_ns()
                    settle(recovery_ms / 1000)
                    queries = {}
                    for name in ("completion", "definition", "rename", "fix_all"):
                        remaining = (
                            recovery_ms / 1000 - (time.perf_counter_ns() - sent[version]) / 1e9
                        )
                        assert remaining > 0, "burst recovery deadline expired"
                        elapsed, response = client.request(*operations[name], timeout=remaining)
                        assert "result" in response and "error" not in response, response
                        assert (
                            response["result"] if name != "fix_all" else response["result"] == []
                        ), response
                        queries[name] = elapsed
                    ready_ms = (time.perf_counter_ns() - sent[version]) / 1e6
                    assert ready_ms <= recovery_ms, (
                        f"burst recovery {ready_ms:.2f} ms exceeds {recovery_ms} ms"
                    )
                    result["recoveries"].append(
                        {"version": version, "ready_ms": ready_ms, "queries_ms": queries}
                    )
                    pause_total += max(pause_ms, (time.perf_counter_ns() - pause_started) / 1e6)
                deadline = start + (index * interval_ms + pause_total) * 1e6
                while time.perf_counter_ns() < deadline:
                    try:
                        receive(
                            client.messages.get(
                                timeout=max(0, (deadline - time.perf_counter_ns()) / 1e9)
                            )
                        )
                    except queue.Empty:
                        break
                result["send_lateness_ms"].append(max(0, (time.perf_counter_ns() - deadline) / 1e6))
                version += 1
                suffix = f"\n# session edit {version}\n"
                event = (
                    {"range": {"start": eof, "end": end}, "text": suffix}
                    if ranged
                    else {"text": text + suffix}
                )
                end = {"line": eof["line"] + 2, "character": 0}
                sent[version] = client.send(
                    "textDocument/didChange",
                    {
                        "textDocument": {"uri": uri, "version": version},
                        "contentChanges": [event],
                    },
                )[1]
                if index % 5 == 0:
                    request("completion")
                if index % 7 == 0:
                    request("definition")
                if index % 11 == 0:
                    request("rename", cancel=True)
                if index % 19 == 0:
                    request("fix_all")
                if index % 50 == 0:
                    result["memory"].append({"edit": index, "kib": memory(client.process.pid)})
                while not client.messages.empty():
                    receive(client.messages.get_nowait())
            settle(30)
            result["duration_ms"] = (time.perf_counter_ns() - start) / 1e6
            result["memory"].append({"edit": edits, "kib": memory(client.process.pid)})
            for name in ("completion", "definition", "rename"):
                _, response = client.request(*operations[name])
                assert response.get("result"), response
            assert any(row["error_code"] == -32800 for row in result["requests"]), (
                "cancellation unexercised"
            )
        except BaseException:
            failed = True
            raise
        finally:
            if failed:
                client.abort()
            else:
                client.close()
    assert source.read_text() == text
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("project", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("target/release/recite-lsp"))
    parser.add_argument("--burst-every", type=int, default=0)
    parser.add_argument("--pause-ms", type=float, default=150)
    parser.add_argument("--recovery-ms", type=float, default=500)
    parser.add_argument("--ranged", action="store_true")
    parser.add_argument("--edits", type=int, default=1000)
    parser.add_argument("--interval-ms", type=float, default=20)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    if (
        args.edits < 20
        or args.burst_every < 0
        or args.burst_every >= args.edits
        or any(
            not math.isfinite(value) or value <= 0
            for value in (args.interval_ms, args.pause_ms, args.recovery_ms)
        )
    ):
        parser.error(
            "require >=20 edits, 0 <= burst-every < edits, and finite positive timing budgets"
        )
    report = {
        "provenance": provenance(args.binary.resolve(), args.project.resolve()),
        "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
        "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "project": str(args.project.resolve()),
        "result": run(
            args.binary.resolve(),
            args.project.resolve(),
            args.edits,
            args.interval_ms,
            args.ranged,
            args.burst_every,
            args.pause_ms,
            args.recovery_ms,
        ),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
