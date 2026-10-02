#!/usr/bin/env python3
"""Measure rename scaling over stdio using full-text overlays, without disk edits."""

import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import statistics
import tempfile

spec = importlib.util.spec_from_file_location(
    "latency_probe", Path(__file__).with_name("measure-lsp-latency.py")
)
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


def run(binary, root, samples):
    source = sorted((root / "src").glob("*.recite"))[0]
    text = source.read_text()
    uri = source.as_uri()
    results = []
    with tempfile.TemporaryDirectory(prefix="recite-fanout-") as config:
        client = probe.Client(binary, Path(config))
        try:
            client.request("initialize", {
                "processId": None, "rootUri": root.as_uri(), "capabilities": {},
            })
            client.send("initialized", {})
            for version, limit in enumerate([0, 100, 500, 10000], 1):
                baseline = re.sub(r"(-> )block_00001\b", r"\g<1>block_00002", text)
                overlay = (re.sub(r"(-> )block_\d+\b", r"\g<1>block_00001",
                                  baseline, count=limit) if limit else text)
                expected = []
                for match in re.finditer(r"(?:-> |:: )(block_00001)\b", overlay):
                    prefix = overlay[:match.start(1)]
                    expected.append({"line": prefix.count("\n"),
                                     "character": len(prefix.rsplit("\n", 1)[-1].encode("utf-16-le")) // 2})
                if version == 1:
                    client.send("textDocument/didOpen", {"textDocument": {
                        "uri": uri, "languageId": "recite", "version": version,
                        "text": overlay,
                    }})
                else:
                    client.send("textDocument/didChange", {
                        "textDocument": {"uri": uri, "version": version},
                        "contentChanges": [{"text": overlay}],
                    })
                client.diagnostics(uri, version)
                request = {"textDocument": {"uri": uri},
                           "position": probe.position(overlay, r"^:: (block_00001)\b"),
                           "newName": "measured_block"}
                observations = []
                for index in range(samples + 2):
                    ms, response = client.request("textDocument/rename", request)
                    result = response.get("result")
                    assert result, response
                    changes = result["documentChanges"]
                    assert len(changes) == 1, changes
                    assert changes[0]["textDocument"] == {"uri": uri, "version": version}
                    edits = changes[0]["edits"]
                    assert [e["range"]["start"] for e in edits] == expected
                    for edit in edits:
                        start, end = edit["range"]["start"], edit["range"]["end"]
                        assert end == {"line": start["line"], "character": start["character"] + 11}
                        assert edit["newText"] == "measured_block"
                    if index >= 2:
                        observations.append(ms)
                results.append({"edits": len(expected), "samples_ms": observations,
                                "overlay_sha256": hashlib.sha256(overlay.encode()).hexdigest()})
        finally:
            client.close()
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("target/release/recite-lsp"))
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--samples", type=int, default=7)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root, binary = args.root.resolve(), args.binary.resolve()
    sources = sorted(root.rglob("*.recite"))
    hashes = {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}
    runs = [run(binary, root, args.samples) for _ in range(args.runs)]
    assert hashes == {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}
    report = {"recorded_at": datetime.now(timezone.utc).isoformat(), "files": hashes,
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "runs": runs}
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    for index, case in enumerate(runs[0]):
        values = [value for run in runs for value in run[index]["samples_ms"]]
        print(f'{case["edits"]} edits: median {statistics.median(values):.3f} ms')


if __name__ == "__main__":
    main()
