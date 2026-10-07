#!/usr/bin/env python3
"""Measure a persistent release LSP over stdio; no source files are modified."""

import argparse
import hashlib
import json
import platform
import re
import statistics
import subprocess
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

from .client import Client, position


def run(binary, root, samples, config_root):
    source = sorted((root / "src").glob("*.recite"))[0]
    text = source.read_text()
    uri = source.as_uri()
    reference = position(text, r"-> (block_\d+)")
    completion = position(text, r"-> (block_\d+)", end=True)
    declaration = position(text, r"^:: (block_00001)\b")

    def at(pos):
        return {"textDocument": {"uri": uri}, "position": pos}

    operations = {
        "completion": ("textDocument/completion", at(completion)),
        "hover": ("textDocument/hover", at(declaration)),
        "definition": ("textDocument/definition", at(reference)),
        "references": (
            "textDocument/references",
            {**at(declaration), "context": {"includeDeclaration": True}},
        ),
        "prepare_rename": ("textDocument/prepareRename", at(declaration)),
        "rename": ("textDocument/rename", {**at(declaration), "newName": "measured_block"}),
        "code_action": (
            "textDocument/codeAction",
            {
                "textDocument": {"uri": uri},
                "range": {"start": declaration, "end": declaration},
                "context": {"diagnostics": [], "only": ["source.fixAll"]},
            },
        ),
    }
    client = Client(binary, config_root)
    results = {}
    try:
        results["initialize_ms"], response = client.request(
            "initialize",
            {
                "processId": None,
                "rootUri": root.as_uri(),
                "capabilities": {},
            },
        )
        assert "result" in response, response
        _, started = client.send("initialized", {})
        _, indexed = client.request(*operations["definition"])
        assert indexed.get("result"), indexed
        results["index_ready_ms"] = (time.perf_counter_ns() - started) / 1e6
        _, started = client.send(
            "textDocument/didOpen",
            {"textDocument": {"uri": uri, "languageId": "recite", "version": 1, "text": text}},
        )
        client.diagnostics(uri, 1)
        results["open_ms"] = (time.perf_counter_ns() - started) / 1e6
        results["requests"] = {}
        for name, (method, params) in operations.items():
            observations = []
            for index in range(samples + 2):
                elapsed, response = client.request(method, params)
                assert "result" in response, response
                value = response["result"]
                if name != "code_action":
                    assert value, (name, response)
                if index >= 2:
                    normalized = json.dumps(value, sort_keys=True).replace(str(root), "$PROJECT")
                    observations.append(
                        {
                            "ms": elapsed,
                            "result_bytes": len(normalized.encode()),
                            "result_sha256": hashlib.sha256(normalized.encode()).hexdigest(),
                        }
                    )
            results["requests"][name] = observations
        version = 2
        draft = re.sub(r"@[0-9a-f]{20}", "", text, count=1)
        assert draft != text, "fixture lacks an authored stable ID"
        client.send(
            "textDocument/didChange",
            {
                "textDocument": {"uri": uri, "version": version},
                "contentChanges": [{"text": draft}],
            },
        )
        client.diagnostics(uri, version)
        results["requests"]["code_action_fix_all"] = []
        for index in range(samples + 2):
            elapsed, response = client.request(*operations["code_action"])
            assert response.get("result"), response
            if index >= 2:
                normalized = json.dumps(response["result"], sort_keys=True).replace(
                    str(root), "$PROJECT"
                )
                results["requests"]["code_action_fix_all"].append(
                    {
                        "ms": elapsed,
                        "result_bytes": len(normalized.encode()),
                        "result_sha256": hashlib.sha256(normalized.encode()).hexdigest(),
                    }
                )

        def change():
            nonlocal version
            version += 1
            return client.send(
                "textDocument/didChange",
                {
                    "textDocument": {"uri": uri, "version": version},
                    "contentChanges": [{"text": text + f"\n# measurement edit {version}\n"}],
                },
            )[1]

        results["edit_refresh_ms"] = []
        for _ in range(samples):
            started = change()
            client.diagnostics(uri, version)
            results["edit_refresh_ms"].append((time.perf_counter_ns() - started) / 1e6)
            published = [
                n["params"]
                for n in client.notifications
                if n.get("method") == "textDocument/publishDiagnostics"
                and n["params"]["uri"] == uri
            ]
            assert published[-1]["version"] == version, published[-1]
        results["cancel_behind_edit"] = []
        for _ in range(samples):
            change()
            request_id, started = client.send(*operations["rename"], request=True)
            client.send("$/cancelRequest", {"id": request_id})
            received, response = client.response(request_id)
            results["cancel_behind_edit"].append(
                {
                    "ms": (received - started) / 1e6,
                    "error_code": response.get("error", {}).get("code"),
                    "returned_edit": bool(response.get("result")),
                }
            )
            client.diagnostics(uri, version)
        client.notifications.clear()
        started = change()
        change()
        change()
        elapsed, response = client.request(*operations["completion"])
        assert response.get("result"), response
        client.diagnostics(uri, version)
        results["three_edit_burst"] = {
            "total_ms": (time.perf_counter_ns() - started) / 1e6,
            "completion_ms": elapsed,
            "published_versions": [
                n["params"].get("version")
                for n in client.notifications
                if n.get("method") == "textDocument/publishDiagnostics"
                and n["params"]["uri"] == uri
            ],
        }
        schema = root / "schema/synthetic.schema.json"
        _, started = client.send(
            "textDocument/didOpen",
            {
                "textDocument": {
                    "uri": schema.as_uri(),
                    "languageId": "json",
                    "version": 1,
                    "text": schema.read_text() + "\n",
                }
            },
        )
        client.diagnostics(schema.as_uri(), 1)
        results["schema_open_refresh_ms"] = (time.perf_counter_ns() - started) / 1e6
        status = Path(f"/proc/{client.process.pid}/status").read_text()
        results["peak_rss_kib"] = int(re.search(r"VmHWM:\s+(\d+)", status)[1])
    finally:
        client.close()
    return results


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("project", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("target/release/recite-lsp"))
    parser.add_argument("--samples", type=int, default=7)
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.samples < 1 or args.runs < 1:
        parser.error("samples and runs must be positive")
    root, binary = args.project.resolve(), args.binary.resolve()
    paths = sorted(p for p in root.rglob("*") if p.is_file())
    report = {
        "format_version": 1,
        "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
        "working_tree_dirty": bool(
            subprocess.check_output(["git", "status", "--porcelain"], text=True).strip()
        ),
        "analysis_fence": "definition result for initial index; exact diagnostic URI and version for updates",
        "recorded_at": datetime.now(timezone.utc).isoformat(),
        "rustc": subprocess.check_output(["rustc", "-Vv"], text=True).strip(),
        "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "platform": platform.platform(),
        "cpu": next(
            line.split(":", 1)[1].strip()
            for line in Path("/proc/cpuinfo").read_text().splitlines()
            if line.startswith("model name")
        ),
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "samples": args.samples,
        "warmups_per_request": 2,
        "project": str(root),
        "files": [
            {
                "path": str(p.relative_to(root)),
                "bytes": p.stat().st_size,
                "sha256": hashlib.sha256(p.read_bytes()).hexdigest(),
            }
            for p in paths
        ],
        "runs": [],
    }
    with tempfile.TemporaryDirectory(prefix="recite-lsp-measure-") as config:
        for index in range(args.runs):
            result = run(binary, root, args.samples, Path(config))
            report["runs"].append(result)
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(report, indent=2) + "\n")
            print(
                f"run {index + 1}: index={result['index_ready_ms']:.1f} ms, "
                f"edit median={statistics.median(result['edit_refresh_ms']):.1f} ms",
                flush=True,
            )
    for expected in report["files"]:
        assert (
            hashlib.sha256((root / expected["path"]).read_bytes()).hexdigest() == expected["sha256"]
        )
