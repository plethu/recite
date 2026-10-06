#!/usr/bin/env python3
"""Compare prepared stopped responses under the paced condition that showed tails."""
import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

from lsp_session_native import distribution, recovery_wire_samples

spec = importlib.util.spec_from_file_location("comparison", Path(__file__).with_name("summarize-lsp-server-comparison.py"))
comparison = importlib.util.module_from_spec(spec)
spec.loader.exec_module(comparison)


def wire_workload(report, trace_path):
    driver = report["driver"]
    if (report["provenance"]["harness_dirty"] or driver["native_trace"]
            or driver["server_environment"] or driver["edit_interval_ms"] != 100
            or not driver["editing_cpu_accounting"] or driver["settled_idle_seconds"] != 3
            or driver["settled_idle_location"] != "startup"):
        raise ValueError("stopped response probe requires its clean, untraced 100 ms driver")
    samples = recovery_wire_samples(report, trace_path)
    outcomes = [row["recovery_outcomes"]["rename_error_code"] for row in report["checkpoints"][5:]]
    if any(code not in (None, -32800) for code in outcomes):
        raise ValueError("rename must finish successfully or with RequestCancelled")
    grouped = {}
    for label, code in (("cancelled", -32800), ("success", None)):
        values = [value for value, outcome in zip(samples["rename"], outcomes, strict=True) if outcome == code]
        grouped[label] = {"count": len(values), "over_10_ms": sum(value > 10 for value in values)}
        if values:
            grouped[label]["wire_ms"] = distribution(values)
    return {"rename_wire_ms": distribution(samples["rename"]),
            "rename_over_10_ms": sum(value > 10 for value in samples["rename"]),
            "rename_outcomes": grouped, "completion_wire_ms": distribution(samples["completion"])}


def run_profile(binary, directory, repetition):
    for mode in ("fixed", "churn"):
        command = [sys.executable, str(Path(__file__).with_name("measure-lsp-endurance.py")),
                   "--binary", str(binary.resolve(strict=True)), "--edit-interval-ms", "100",
                   "--edits", "10", "--editing-cpu-accounting", "--startup-idle-seconds", "3",
                   "--output", str(directory / f"{mode}-{repetition}.json")]
        if mode == "churn":
            command.append("--churn")
        subprocess.run(command, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--control", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    for repetition in range(1, 4):
        order = ("control", "candidate") if repetition % 2 else ("candidate", "control")
        for side in order:
            run_profile(args.control if side == "control" else args.candidate,
                        args.output / "control" if side == "control" else args.output, repetition)
    result = comparison.summarize(args.output, cycles=40, edits=10)
    result["wire_workloads"] = {}
    for side in ("control", "candidate"):
        root = args.output / "control" if side == "control" else args.output
        result["wire_workloads"][side] = []
        for mode in ("fixed", "churn"):
            for repetition in range(1, 4):
                path = root / f"{mode}-{repetition}.json"
                report = json.loads(path.read_text())
                result["wire_workloads"][side].append({"workload": f"{mode}-{repetition}",
                    **wire_workload(report, path.with_suffix(".jsonl"))})
    result["notes"].append("100 ms pacing; 40 cycles/10 edits; both arms enable editing CPU and hold startup idle at three seconds.")
    (args.output / "summary.json").write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
