#!/usr/bin/env python3
"""Prove resource/latency decisions against a real, deliberately degrading child."""

import argparse
import json
from pathlib import Path
import subprocess
import sys
import threading
import time

from lsp_session_health import assess, resources


def child():
    retained = []
    handles = []
    for index, _ in enumerate(sys.stdin):
        allocation = bytearray(4 * 1024**2)
        for offset in range(0, len(allocation), 4096):
            allocation[offset] = 1
        retained.append(allocation)
        handles.append(open(__file__, "rb"))
        threading.Thread(target=threading.Event().wait, daemon=True).start()
        # Quadratic delay dominates platform-specific allocation overhead, so
        # the injected slowdown actually crosses the unchanged 2x threshold.
        time.sleep(index * index * 0.001)
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
            expected = {"rss_bytes", "threads", "handles", "completion_ms", "definition_ms"}
            detected = expected <= set(health["failures"])
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps({"expected_failure_detected": detected, "checkpoints": rows,
                                               "health": health}, indent=2) + "\n")
            assert detected, health
        finally:
            process.stdin.close()
            process.wait(timeout=10)
    print("Live memory, thread, handle and latency faults were detected.")


if __name__ == "__main__":
    child() if sys.argv[1:] == ["--child"] else main()
