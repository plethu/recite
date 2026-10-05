#!/usr/bin/env python3
"""Enforce calibrated recovery tails across three complete session repetitions."""

import argparse
import json
from pathlib import Path

from lsp_session_health import recovery_tail


def evaluate(directory, budget_ms):
    results, identity = {}, None
    for mode in ("fixed", "churn"):
        repetitions = []
        for round_number in range(1, 4):
            report = json.loads((directory / f"{mode}-{round_number}.json").read_text())
            if (report["status"] != "pass" or report["health"]["status"] != "pass"
                    or report["cycles"] != 40 or report["edits_per_cycle"] != 50
                    or report["churn"] != (mode == "churn")
                    or len(report["checkpoints"]) != 40
                    or [row["cycle"] for row in report["checkpoints"]] != list(range(40))):
                raise ValueError("recovery calibration requires complete 40-cycle, 50-edit workloads")
            observed = (report["provenance"]["binary_sha256"], report["provenance"]["files"], report["seed"])
            if identity is not None and observed != identity:
                raise ValueError("recovery repetitions have inconsistent binary, fixture or seed")
            identity = observed
            repetitions.append([row["recovery_ms"] for row in report["checkpoints"][5:]])
        results[mode] = recovery_tail(repetitions, budget_ms)
    return {"status": "pass" if all(value["status"] == "pass" for value in results.values()) else "regression",
            "warmup_cycles": 5, "percentile_method": "nearest rank", "workloads": results}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reports", type=Path, required=True)
    parser.add_argument("--budget-ms", type=float, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = evaluate(args.reports, args.budget_ms)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))
    raise SystemExit(0 if result["status"] == "pass" else 1)


if __name__ == "__main__":
    main()
