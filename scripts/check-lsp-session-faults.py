#!/usr/bin/env python3
"""Prove resource/latency decisions against a real, deliberately degrading child."""

import argparse
import json
from pathlib import Path
import subprocess
import sys
import time

from lsp_session_health import assess, resources


def child():
    retained = []
    handles = []
    for index, _ in enumerate(sys.stdin):
        retained.append(bytearray(4 * 1024**2))
        handles.append(open(__file__, "rb"))
        time.sleep(index * 0.003)
        print("ready", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    with subprocess.Popen([sys.executable, __file__, "--child"], stdin=subprocess.PIPE,
                          stdout=subprocess.PIPE, text=True) as process:
        rows = []
        try:
            for _ in range(35):
                started = time.perf_counter_ns()
                process.stdin.write("step\n")
                process.stdin.flush()
                assert process.stdout.readline().strip() == "ready"
                elapsed = (time.perf_counter_ns() - started) / 1e6
                rows.append({**resources(process.pid), "completion_ms": elapsed, "definition_ms": elapsed})
            health = assess(rows)
            assert {"rss_bytes", "handles", "completion_ms", "definition_ms"} <= set(health["failures"]), health
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps({"expected_failure_detected": True, "checkpoints": rows,
                                               "health": health}, indent=2) + "\n")
        finally:
            process.stdin.close()
            process.wait(timeout=10)
    print("Live memory, handle and latency faults were detected.")


if __name__ == "__main__":
    child() if sys.argv[1:] == ["--child"] else main()
