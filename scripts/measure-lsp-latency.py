#!/usr/bin/env python3
"""Measure a persistent release LSP over stdio; no source files are modified."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import queue
import re
import statistics
import subprocess
import tempfile
import threading
import time


class Client:
    def __init__(self, binary, config_root):
        self.stderr = tempfile.TemporaryFile()
        self.process = subprocess.Popen(
            [str(binary)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=self.stderr, env={**os.environ, "XDG_CONFIG_HOME": str(config_root),
                                     "APPDATA": str(config_root), "LOCALAPPDATA": str(config_root)},
        )
        self.messages = queue.Queue()
        self.next_id = 0
        self.notifications = []
        self.reader = threading.Thread(target=self.read, daemon=True)
        self.reader.start()

    def read(self):
        try:
            while True:
                headers = {}
                while True:
                    line = self.process.stdout.readline()
                    if not line:
                        raise EOFError("server stdout closed")
                    if line == b"\r\n":
                        break
                    key, value = line.decode("ascii").split(":", 1)
                    headers[key.lower()] = value.strip()
                body = self.process.stdout.read(int(headers["content-length"]))
                self.messages.put((time.perf_counter_ns(), json.loads(body)))
        except Exception as error:
            self.messages.put(error)

    def send(self, method, params, request=False):
        message = {"jsonrpc": "2.0", "method": method, "params": params}
        if request:
            self.next_id += 1
            message["id"] = self.next_id
        body = json.dumps(message, separators=(",", ":")).encode()
        started = time.perf_counter_ns()
        self.process.stdin.write(f"Content-Length: {len(body)}\r\n\r\n".encode() + body)
        self.process.stdin.flush()
        return message.get("id"), started

    def response(self, request_id, timeout=120):
        deadline = time.monotonic() + timeout
        while True:
            item = self.messages.get(timeout=max(0.001, deadline - time.monotonic()))
            if isinstance(item, Exception):
                raise item
            received, message = item
            if message.get("id") == request_id:
                return received, message
            if "id" in message:
                raise RuntimeError(f"unexpected server request or response: {message}")
            self.notifications.append(message)

    def request(self, method, params, timeout=120):
        request_id, started = self.send(method, params, request=True)
        received, message = self.response(request_id, timeout)
        return (received - started) / 1e6, message

    def diagnostics(self, uri, version, timeout=120):
        def matches(message):
            return (message.get("method") == "textDocument/publishDiagnostics"
                    and message["params"]["uri"] == uri
                    and message["params"].get("version") == version)
        if any(matches(message) for message in self.notifications):
            return
        deadline = time.monotonic() + timeout
        while True:
            item = self.messages.get(timeout=max(0.001, deadline - time.monotonic()))
            if isinstance(item, Exception):
                raise item
            _, message = item
            if "id" in message:
                raise RuntimeError(f"unexpected response while waiting for diagnostics: {message}")
            self.notifications.append(message)
            if matches(message):
                return

    def close(self):
        try:
            if self.process.poll() is None:
                self.request("shutdown", None)
                self.send("exit", None)
                self.process.stdin.close()
                self.process.wait(timeout=10)
            if self.process.returncode != 0:
                self.stderr.seek(0)
                raise RuntimeError(self.stderr.read().decode())
        finally:
            if self.process.poll() is None:
                self.process.kill()
                self.process.wait()
            self.process.stdout.close()
            self.reader.join(timeout=1)
            self.stderr.close()

    def abort(self):
        """Clean up an owned server without masking a failed probe's exception."""
        if self.process.poll() is None:
            self.process.kill()
        self.process.wait()
        self.process.stdin.close()
        self.process.stdout.close()
        self.reader.join(timeout=1)
        self.stderr.close()


def position(text, pattern, end=False):
    match = re.search(pattern, text, re.MULTILINE)
    if not match:
        raise ValueError(f"fixture lacks probe {pattern}")
    offset = match.end(1) if end else match.start(1)
    prefix = text[:offset]
    return {"line": prefix.count("\n"),
            "character": len(prefix.rsplit("\n", 1)[-1].encode("utf-16-le")) // 2}


def run(binary, root, samples, config_root):
    source = sorted((root / "src").glob("*.recite"))[0]
    text = source.read_text()
    uri = source.as_uri()
    reference = position(text, r"-> (block_\d+)")
    completion = position(text, r"-> (block_\d+)", end=True)
    declaration = position(text, r"^:: (block_00001)\b")
    at = lambda pos: {"textDocument": {"uri": uri}, "position": pos}
    operations = {
        "completion": ("textDocument/completion", at(completion)),
        "hover": ("textDocument/hover", at(declaration)),
        "definition": ("textDocument/definition", at(reference)),
        "references": ("textDocument/references", {**at(declaration),
                       "context": {"includeDeclaration": True}}),
        "prepare_rename": ("textDocument/prepareRename", at(declaration)),
        "rename": ("textDocument/rename", {**at(declaration), "newName": "measured_block"}),
        "code_action": ("textDocument/codeAction", {"textDocument": {"uri": uri},
                        "range": {"start": declaration, "end": declaration},
                        "context": {"diagnostics": [], "only": ["source.fixAll"]}}),
    }
    client = Client(binary, config_root)
    results = {}
    try:
        results["initialize_ms"], response = client.request("initialize", {
            "processId": None, "rootUri": root.as_uri(), "capabilities": {},
        })
        assert "result" in response, response
        _, started = client.send("initialized", {})
        _, indexed = client.request(*operations["definition"])
        assert indexed.get("result"), indexed
        results["index_ready_ms"] = (time.perf_counter_ns() - started) / 1e6
        _, started = client.send("textDocument/didOpen", {"textDocument": {
            "uri": uri, "languageId": "recite", "version": 1, "text": text}})
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
                    observations.append({"ms": elapsed, "result_bytes": len(normalized.encode()),
                                         "result_sha256": hashlib.sha256(normalized.encode()).hexdigest()})
            results["requests"][name] = observations
        version = 2
        draft = re.sub(r"@[0-9a-f]{20}", "", text, count=1)
        assert draft != text, "fixture lacks an authored stable ID"
        client.send("textDocument/didChange", {
            "textDocument": {"uri": uri, "version": version},
            "contentChanges": [{"text": draft}],
        })
        client.diagnostics(uri, version)
        results["requests"]["code_action_fix_all"] = []
        for index in range(samples + 2):
            elapsed, response = client.request(*operations["code_action"])
            assert response.get("result"), response
            if index >= 2:
                normalized = json.dumps(response["result"], sort_keys=True).replace(str(root), "$PROJECT")
                results["requests"]["code_action_fix_all"].append({"ms": elapsed,
                    "result_bytes": len(normalized.encode()),
                    "result_sha256": hashlib.sha256(normalized.encode()).hexdigest()})

        def change():
            nonlocal version
            version += 1
            return client.send("textDocument/didChange", {
                "textDocument": {"uri": uri, "version": version},
                "contentChanges": [{"text": text + f"\n# measurement edit {version}\n"}],
            })[1]

        results["edit_refresh_ms"] = []
        for _ in range(samples):
            started = change()
            client.diagnostics(uri, version)
            results["edit_refresh_ms"].append((time.perf_counter_ns() - started) / 1e6)
            published = [n["params"] for n in client.notifications
                         if n.get("method") == "textDocument/publishDiagnostics"
                         and n["params"]["uri"] == uri]
            assert published[-1]["version"] == version, published[-1]
        results["cancel_behind_edit"] = []
        for _ in range(samples):
            change()
            request_id, started = client.send(*operations["rename"], request=True)
            client.send("$/cancelRequest", {"id": request_id})
            received, response = client.response(request_id)
            results["cancel_behind_edit"].append({"ms": (received - started) / 1e6,
                "error_code": response.get("error", {}).get("code"),
                "returned_edit": bool(response.get("result"))})
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
            "published_versions": [n["params"].get("version") for n in client.notifications
                if n.get("method") == "textDocument/publishDiagnostics" and n["params"]["uri"] == uri],
        }
        schema = root / "schema/synthetic.schema.json"
        _, started = client.send("textDocument/didOpen", {"textDocument": {
            "uri": schema.as_uri(), "languageId": "json", "version": 1,
            "text": schema.read_text() + "\n"}})
        client.diagnostics(schema.as_uri(), 1)
        results["schema_open_refresh_ms"] = (time.perf_counter_ns() - started) / 1e6
        status = Path(f"/proc/{client.process.pid}/status").read_text()
        results["peak_rss_kib"] = int(re.search(r"VmHWM:\s+(\d+)", status)[1])
    finally:
        client.close()
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("project", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("target/release/recite-lsp"))
    parser.add_argument("--samples", type=int, default=7)
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.samples < 1 or args.runs < 1:
        parser.error("samples and runs must be positive")
    root, binary = args.project.resolve(), args.binary.resolve()
    paths = sorted(p for p in root.rglob("*") if p.is_file())
    report = {"format_version": 1, "revision": subprocess.check_output(
        ["git", "rev-parse", "HEAD"], text=True).strip(),
        "working_tree_dirty": bool(subprocess.check_output(["git", "status", "--porcelain"], text=True).strip()),
        "analysis_fence": "definition result for initial index; exact diagnostic URI and version for updates",
        "recorded_at": datetime.now(timezone.utc).isoformat(),
        "rustc": subprocess.check_output(["rustc", "-Vv"], text=True).strip(),
        "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "platform": platform.platform(), "cpu": next(line.split(":", 1)[1].strip()
        for line in Path("/proc/cpuinfo").read_text().splitlines() if line.startswith("model name")),
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "samples": args.samples, "warmups_per_request": 2, "project": str(root),
        "files": [{"path": str(p.relative_to(root)), "bytes": p.stat().st_size,
                   "sha256": hashlib.sha256(p.read_bytes()).hexdigest()} for p in paths],
        "runs": []}
    with tempfile.TemporaryDirectory(prefix="recite-lsp-measure-") as config:
        for index in range(args.runs):
            result = run(binary, root, args.samples, Path(config))
            report["runs"].append(result)
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(report, indent=2) + "\n")
            print(f"run {index + 1}: index={result['index_ready_ms']:.1f} ms, "
                  f"edit median={statistics.median(result['edit_refresh_ms']):.1f} ms", flush=True)
    for expected in report["files"]:
        assert hashlib.sha256((root / expected["path"]).read_bytes()).hexdigest() == expected["sha256"]


if __name__ == "__main__":
    main()
