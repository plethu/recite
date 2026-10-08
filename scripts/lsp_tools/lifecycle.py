"""Fresh-process startup and peak-memory samples on an already generated project."""

import hashlib
import json
import tempfile
import time
from pathlib import Path


def measure(probe, binary, root, samples):
    source = sorted((root / "src").glob("*.recite"))[0]
    text, uri = source.read_text(), source.as_uri()
    position = probe.position(text, r"-> (block_\d+)")
    values = {
        name: [] for name in ("lifecycle/index_ready", "lifecycle/open", "memory/peak_rss_kib")
    }
    fingerprints = []
    for index in range(samples + 2):
        with tempfile.TemporaryDirectory(prefix="recite-lsp-startup-") as config:
            startup_started = time.perf_counter_ns()
            client = probe.Client(binary, Path(config))
            failed = False
            try:
                client.request("initialize", {"rootUri": root.as_uri(), "capabilities": {}})
                client.send("initialized", {})
                _, response = client.request(
                    "textDocument/definition", {"textDocument": {"uri": uri}, "position": position}
                )
                indexed = (time.perf_counter_ns() - startup_started) / 1e6
                assert response.get("result"), response
                _, started = client.send(
                    "textDocument/didOpen",
                    {
                        "textDocument": {
                            "uri": uri,
                            "languageId": "recite",
                            "version": 1,
                            "text": text,
                        }
                    },
                )
                client.diagnostics(uri, 1)
                opened = (time.perf_counter_ns() - started) / 1e6
                status = Path(f"/proc/{client.process.pid}/status").read_text()
                peak = int(
                    next(line for line in status.splitlines() if line.startswith("VmHWM:")).split()[
                        1
                    ]
                )
                diagnostics = next(
                    message["params"]["diagnostics"]
                    for message in reversed(client.notifications)
                    if message.get("method") == "textDocument/publishDiagnostics"
                    and message["params"].get("uri") == uri
                    and message["params"].get("version") == 1
                )
                assert not diagnostics, diagnostics
                fingerprint = hashlib.sha256(
                    json.dumps(response["result"], sort_keys=True)
                    .replace(str(root), "$PROJECT")
                    .encode()
                ).hexdigest()
                if index >= 2:
                    values["lifecycle/index_ready"].append(indexed)
                    values["lifecycle/open"].append(opened)
                    values["memory/peak_rss_kib"].append(peak)
                    fingerprints.append({"sha256": fingerprint, "diagnostics": diagnostics})
            except BaseException:
                failed = True
                raise
            finally:
                if failed:
                    client.abort()
                else:
                    client.close()
    return {"workloads": values, "diagnostics": {name: fingerprints for name in values}}
