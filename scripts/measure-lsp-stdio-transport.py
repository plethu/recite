#!/usr/bin/env python3
"""Compare actual lsp-server stdio with a direct framing diagnostic baseline."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import platform
import tempfile
import time

import psutil

from lsp_session_native import distribution

spec = importlib.util.spec_from_file_location("latency", Path(__file__).with_name("measure-lsp-latency.py"))
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


def measure(binary, mode, entries, samples):
    with tempfile.TemporaryDirectory(prefix="recite-stdio-probe-") as config:
        client = probe.Client(binary, Path(config), server_env={"RECITE_STDIO_PROBE_MODE": mode})
        process = psutil.Process(client.process.pid)
        wire, pauses = [], []
        try:
            for serial in range(samples + 20):
                started = time.perf_counter_ns()
                time.sleep(0.002)
                pauses.append((time.perf_counter_ns() - started) / 1e6)
                elapsed, response = client.request("probe/exchange", {"serial": serial, "entries": entries}, timeout=10)
                assert response["result"] == {"serial": serial}, response
                assert len(client.notifications) == 1, client.notifications
                notification = client.notifications.pop()
                assert notification["method"] == "probe/diagnostics"
                assert len(notification["params"]) == entries
                wire.append(elapsed)
            _, response = client.request("probe/report", {}, timeout=10)
            native = response["result"]
            assert [row["serial"] for row in native] == list(range(samples + 20))
            cpu = process.cpu_times()
            result = {"mode": mode, "entries": entries, "warmup": 20,
                      "cpu_ms": (cpu.user + cpu.system) * 1000,
                      "wire_ms": distribution(wire[20:]), "pause_ms": distribution(pauses[20:]),
                      "native_ms": {key: distribution([row[key] for row in native[20:]]) for key in
                                    ("notification_handoff_ms", "response_handoff_ms")},
                      "samples": [{**row, "wire_ms": elapsed} for row, elapsed in zip(native[20:], wire[20:])]}
            client.close()
            return result
        except BaseException:
            client.abort()
            raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--samples", type=int, default=150)
    args = parser.parse_args()
    if not 15 <= args.samples <= 1000:
        parser.error("require 15 to 1000 samples")
    binary = args.binary.resolve(strict=True)
    report = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "platform": platform.platform(), "runs": [],
              "notes": ["Direct is a diagnostic baseline, not a compatible production replacement.",
                        "Native uses the pinned lsp-server stdio reader, writer and dropper unchanged.",
                        "Wire timing includes Python framing/JSON and scheduler effects; native timing is one Rust clock."]}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    for repetition in range(1, 4):
        for entries in (0, 128, 4096):
            for mode in (("native", "direct") if repetition % 2 else ("direct", "native")):
                result = measure(binary, mode, entries, args.samples)
                result["repetition"] = repetition
                report["runs"].append(result)
                args.output.write_text(json.dumps(report, indent=2) + "\n")
                print(json.dumps({key: value for key, value in result.items() if key != "samples"}), flush=True)


if __name__ == "__main__":
    main()
