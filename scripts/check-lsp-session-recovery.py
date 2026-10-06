#!/usr/bin/env python3
"""Enforce calibrated recovery tails across three complete session repetitions."""

import argparse
import json
from pathlib import Path

from lsp_session_health import idle_cpu_budget, recovery_tail


def evaluate(directory, budget_ms, completion_budget_ms=None, *, cycles=40, edits=50, idle_cpu_budget_ms=None):
    results, completion_results, idle_results, identity = {}, {}, {}, None
    for mode in ("fixed", "churn"):
        repetitions = []
        completions = []
        idle_intervals = []
        for round_number in range(1, 4):
            report = json.loads((directory / f"{mode}-{round_number}.json").read_text())
            if (report["status"] != "pass" or report["health"]["status"] != "pass"
                    or report["cycles"] != cycles or report["edits_per_cycle"] != edits
                    or report["churn"] != (mode == "churn")
                    or len(report["checkpoints"]) != cycles
                    or [row["cycle"] for row in report["checkpoints"]] != list(range(cycles))):
                raise ValueError("recovery calibration requires complete workloads with the requested cycle and edit counts")
            observed = (report["provenance"]["binary_sha256"], report["provenance"]["harness_revision"],
                        report["provenance"]["files"], report["seed"], report.get("driver"))
            if identity is not None and observed != identity:
                raise ValueError("recovery repetitions have inconsistent binary, harness, fixture, seed or driver")
            identity = observed
            repetitions.append([row["recovery_ms"] for row in report["checkpoints"][5:]])
            if completion_budget_ms is not None:
                completions.append([row["timing"]["recovery_completion_ms"] for row in report["checkpoints"][5:]])
            if idle_cpu_budget_ms is not None:
                idle_intervals.append(report["settled_idle"])
        results[mode] = recovery_tail(repetitions, budget_ms)
        if completion_budget_ms is not None:
            completion_results[mode] = recovery_tail(completions, completion_budget_ms)
        if idle_cpu_budget_ms is not None:
            idle_results[mode] = idle_cpu_budget(idle_intervals, idle_cpu_budget_ms)
    output = {"status": "pass" if all(value["status"] == "pass" for value in
                                     (*results.values(), *completion_results.values(), *idle_results.values())) else "regression",
              "warmup_cycles": 5, "percentile_method": "nearest rank", "workloads": results}
    if completion_budget_ms is not None:
        output["completion_workloads"] = completion_results
    if idle_cpu_budget_ms is not None:
        output["idle_workloads"] = idle_results
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reports", type=Path, required=True)
    parser.add_argument("--budget-ms", type=float, required=True)
    parser.add_argument("--completion-budget-ms", type=float,
                        help="Optional repeated p95 bound on the post-cancellation completion stage")
    parser.add_argument("--idle-cpu-budget-ms", type=float,
                        help="Optional repeated CPU bound during a settled three-second interval")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = evaluate(args.reports, args.budget_ms, args.completion_budget_ms, idle_cpu_budget_ms=args.idle_cpu_budget_ms)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))
    raise SystemExit(0 if result["status"] == "pass" else 1)


if __name__ == "__main__":
    main()
