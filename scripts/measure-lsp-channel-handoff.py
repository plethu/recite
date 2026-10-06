#!/usr/bin/env python3
"""Alternate Crossbeam blocking/selected receives with exact child CPU sampling."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import queue
import statistics
import threading

import psutil


def distribution(values):
    ordered = sorted(values)
    return {
        "median": statistics.median(ordered),
        "p95": ordered[(95 * len(ordered) + 99) // 100 - 1],
        "max": ordered[-1],
    }


def probe(binary, mode, capacity, work_us, samples, alternate_yield):
    env = os.environ.copy()
    env.pop("PTHREAD_YIELD_TO_ZERO", None)
    if alternate_yield:
        env["PTHREAD_YIELD_TO_ZERO"] = "0"
    command = [str(binary), mode, str(capacity), str(work_us), "2000", str(samples), "20"]
    child = psutil.Popen(command, env=env, stdin=-1, stdout=-1, stderr=-1, text=True)
    lines = queue.Queue()
    reader = threading.Thread(target=lambda: lines.put(child.stdout.readline()), daemon=True)
    reader.start()
    try:
        line = lines.get(timeout=30)
        if not line:
            raise RuntimeError(f"probe exited before report: {child.stderr.read()}")
        cpu = child.cpu_times()
        report = json.loads(line)
        _, stderr = child.communicate(input="sampled\n", timeout=5)
        reader.join(timeout=1)
        if child.returncode:
            raise RuntimeError(f"probe exited {child.returncode}: {stderr}")
    finally:
        if child.poll() is None:
            child.kill()
            child.communicate(timeout=5)
        reader.join(timeout=1)
    if len(report["samples"]) != samples:
        raise RuntimeError("probe produced incomplete sample set")
    report["yield_to_zero"] = "0" if alternate_yield else "default"
    report["cpu_ms"] = (cpu.user + cpu.system) * 1000
    report["cpu_ms_per_exchange_including_warmup"] = report["cpu_ms"] / (samples + 20)
    report["summary_ms"] = {
        metric: distribution([sample[metric] for sample in report["samples"]])
        for metric in ("dispatch_ms", "reply_ms", "roundtrip_ms", "input_send_ms", "output_send_ms")
    }
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--samples", type=int, default=150)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    report = {
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "platform": platform.platform(),
        "machine": platform.machine(),
        "cpu_count": psutil.cpu_count(),
        "runs": [],
    }
    for repetition in range(1, 4):
        variants = [("blocking", False), ("select", False)]
        if platform.system() == "Darwin":
            variants.extend([("blocking", True), ("select", True)])
        if repetition % 2 == 0:
            variants.reverse()
        for capacity in (0, 1):
            for work_us in (0, 200):
                for mode, alternate_yield in variants:
                    result = probe(binary, mode, capacity, work_us, args.samples, alternate_yield)
                    result["repetition"] = repetition
                    report["runs"].append(result)
                    args.output.parent.mkdir(parents=True, exist_ok=True)
                    args.output.write_text(json.dumps(report, indent=2) + "\n")
                    print(json.dumps({key: value for key, value in result.items() if key != "samples"}), flush=True)


if __name__ == "__main__":
    main()
