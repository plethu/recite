#!/usr/bin/env python3
"""Isolate two optional CPU reads using one server binary and one committed driver."""
import argparse
import json
from pathlib import Path
import subprocess
import sys

from lsp_session_native import distribution


def summarize(directory):
    identity, fingerprint = None, None
    pairs = []
    for repetition in range(1, 4):
        pair = {}
        for enabled in (False, True):
            label = "enabled" if enabled else "disabled"
            path = directory / f"{label}-{repetition}.json"
            report = json.loads(path.read_text())
            if (report["status"] != "pass" or report["health"]["status"] != "pass"
                    or report["cycles"] != 40 or report["edits_per_cycle"] != 50
                    or report["churn"] or report["result_fingerprint_version"] != 2
                    or [row["cycle"] for row in report["checkpoints"]] != list(range(40))):
                raise ValueError("accounting comparison requires complete fixed workloads")
            if report["provenance"]["harness_dirty"]:
                raise ValueError("accounting comparison requires a committed clean driver")
            driver = dict(report["driver"])
            if driver.pop("editing_cpu_accounting") != enabled or driver["native_trace"]:
                raise ValueError("accounting comparison requires only its intended sampling flag")
            if driver["edit_interval_ms"] != 5:
                raise ValueError("accounting comparison requires the calibrated 5 ms pacing")
            if driver["settled_idle_location"] != "startup" or driver["settled_idle_seconds"] != 3:
                raise ValueError("accounting comparison holds startup idle constant")
            provenance = {key: report["provenance"][key] for key in
                          ("binary_sha256", "files", "harness_revision", "environment")}
            observed = {"provenance": provenance, "driver": driver, "seed": report["seed"]}
            if identity is not None and identity != observed:
                raise ValueError("accounting comparison requires one binary, fixture, harness and environment")
            identity = observed
            if not all(row.get("fresh_oracle_matched") for row in report["checkpoints"]
                       if row["cycle"] % 10 == 0 or row["cycle"] == 39):
                raise ValueError("accounting comparison requires fresh-server checks")
            for row in report["checkpoints"]:
                if fingerprint is not None and fingerprint != row["result_sha256"]:
                    raise ValueError("accounting comparison results differ")
                fingerprint = row["result_sha256"]
                if ("editing_server_cpu_ms" in row["timing"]) != enabled:
                    raise ValueError("disabled editing CPU must be absent rather than invented")
            sent, received = {}, {}
            for line in path.with_suffix(".jsonl").read_text().splitlines():
                event = json.loads(line)
                if event["pid"] != report["server_pid"] or event.get("id") is None:
                    continue
                if event["event"] == "send":
                    sent[event["id"]] = event["started_ns"]
                elif event["event"] == "response":
                    received[event["id"]] = event["received_ns"]
            measured = report["checkpoints"][5:]
            metrics = {"recovery_ms": distribution([row["recovery_ms"] for row in measured])}
            for name in ("recovery_completion_ms", "cancelled_rename_ms", "repair_to_rename_send_ms"):
                metrics[name] = distribution([row["timing"][name] for row in measured])
            for name in ("rename", "completion"):
                ids = [row["recovery_requests"][name] for row in measured]
                metrics[name + "_wire_ms"] = distribution([(received[key] - sent[key]) / 1e6 for key in ids])
            metrics["edit_intervals_ms"] = distribution([value for row in measured for value in row["timing"]["edit_intervals_ms"]])
            pair[label] = metrics
        pairs.append(pair)
    return {"identity": identity, "result_sha256": fingerprint, "pairs": pairs,
            "notes": ["One exact server binary; three alternating fixed-workload pairs; native tracing disabled.",
                      "Only two editing CPU reads differ; startup idle is held at three seconds.",
                      "Report pair-level wire and stage tails; do not treat cycles as independent experiments."]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    for repetition in range(1, 4):
        for enabled in ((False, True) if repetition % 2 else (True, False)):
            label = "enabled" if enabled else "disabled"
            command = [sys.executable, str(Path(__file__).with_name("measure-lsp-endurance.py")),
                       "--binary", str(args.binary.resolve(strict=True)), "--startup-idle-seconds", "3",
                       "--output", str(args.output / f"{label}-{repetition}.json")]
            if enabled:
                command.append("--editing-cpu-accounting")
            subprocess.run(command, check=True)
    (args.output / "summary.json").write_text(json.dumps(summarize(args.output), indent=2) + "\n")


if __name__ == "__main__":
    main()
