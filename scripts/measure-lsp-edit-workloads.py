#!/usr/bin/env python3
"""Separate changed-file scaling from project size and exercise several edit kinds."""

import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import statistics
import tempfile
import time

spec = importlib.util.spec_from_file_location(
    "latency_probe", Path(__file__).with_name("measure-lsp-latency.py")
)
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


def measure(binary, root, samples):
    sources = sorted((root / "src").glob("*.recite"))
    source = sources[0]
    text, uri = source.read_text(), source.as_uri()
    files = {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}
    result = {"root": str(root), "files": files, "source_bytes": source.stat().st_size,
              "project_bytes": sum(p.stat().st_size for p in sources), "workloads": {}, "diagnostics": {}}
    with tempfile.TemporaryDirectory(prefix="recite-edit-workloads-") as config:
        client = probe.Client(binary, Path(config))
        try:
            client.request("initialize", {"processId": None, "rootUri": root.as_uri(), "capabilities": {}})
            client.send("initialized", {})
            client.send("textDocument/didOpen", {"textDocument": {
                "uri": uri, "languageId": "recite", "version": 1, "text": text,
            }})
            client.diagnostics(uri, 1)
            version = 1
            for kind in ("comment", "prose", "line_insert", "stable_id", "block_topology", "recovery"):
                values, diagnostics = [], []
                for index in range(samples + 2):
                    version += 1
                    if kind == "comment":
                        overlay = text + f"\n# edit {version}\n"
                    elif kind == "prose":
                        overlay = re.sub(r"(line 00000 000 )", rf"\g<1>revision {version} ", text, count=1)
                    elif kind == "line_insert":
                        overlay = re.sub(r"(?m)^([ \t]*)(line 00000 000 )", lambda match: f"{match[1]}{match[2]}revision {version} " + ("\n" + match[1] + "New paragraph ") * (1 + index % 2), text, count=1)
                    elif kind == "recovery":
                        overlay = text + (f"\nstray syntax {version}\n" if index % 2 else f"\n# recovered {version}\n")
                    elif kind == "stable_id":
                        overlay = text.replace("line_00000_000@", f"line_00000_000_v{version}@", 1)
                    else:
                        overlay = text + f"\n:: measured_{version}\n-> END\n"
                    assert overlay != text
                    _, started = client.send("textDocument/didChange", {
                        "textDocument": {"uri": uri, "version": version},
                        "contentChanges": [{"text": overlay}],
                    })
                    client.diagnostics(uri, version)
                    elapsed = (time.perf_counter_ns() - started) / 1e6
                    published = next(message["params"]["diagnostics"] for message in reversed(client.notifications)
                                     if message.get("method") == "textDocument/publishDiagnostics"
                                     and message["params"].get("uri") == uri
                                     and message["params"].get("version") == version)
                    if kind != "recovery":
                        assert not published, (kind, published)
                    if index >= 2:
                        values.append(elapsed)
                        diagnostics.append({"count": len(published), "sha256": hashlib.sha256(json.dumps(published, sort_keys=True).encode()).hexdigest()})
                result["workloads"][kind] = values
                result["diagnostics"][kind] = diagnostics
        finally:
            client.close()
    assert files == {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("roots", type=Path, nargs="+")
    parser.add_argument("--binary", type=Path, default=Path("target/release/recite-lsp"))
    parser.add_argument("--samples", type=int, default=21)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve()
    runs = []
    for root in args.roots:
        result = measure(binary, root.resolve(), args.samples)
        runs.append(result)
        print(root.name, result["source_bytes"],
              {kind: round(statistics.median(values), 3) for kind, values in result["workloads"].items()},
              flush=True)
    report = {"recorded_at": datetime.now(timezone.utc).isoformat(),
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), "runs": runs}
    args.output.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
