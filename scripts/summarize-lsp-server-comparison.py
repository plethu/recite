#!/usr/bin/env python3
"""Validate and summarize alternating server binaries under the same session driver."""
import argparse
import importlib.util
import json
from pathlib import Path

from lsp_session_native import distribution

spec = importlib.util.spec_from_file_location(
    "recovery", Path(__file__).with_name("check-lsp-session-recovery.py"))
recovery = importlib.util.module_from_spec(spec)
spec.loader.exec_module(recovery)


def summarize(directory):
    identity = None
    sides = {}
    for side, root in (("control", directory / "control"), ("candidate", directory)):
        # Reuse the fail-closed completeness and repetition-identity contract.
        validated = recovery.evaluate(root, 500)
        if validated["status"] != "pass":
            raise ValueError("server comparison requires passing recovery workloads")
        rows, tails = [], []
        for mode in ("fixed", "churn"):
            for repetition in range(1, 4):
                report = json.loads((root / f"{mode}-{repetition}.json").read_text())
                observed = {key: report["provenance"][key] for key in
                            ("files", "harness_revision", "environment")}
                observed["seed"] = report["seed"]
                observed["driver"] = report["driver"]
                if report.get("result_fingerprint_version") != 2:
                    raise ValueError("comparison requires portable result fingerprints")
                if identity is not None and observed != identity:
                    raise ValueError("comparison requires the same fixture, harness, environment and driver")
                identity = observed
                if not all(row.get("fresh_oracle_matched") for row in report["checkpoints"]
                           if row["cycle"] % 10 == 0 or row["cycle"] == 39):
                    raise ValueError("server comparison requires fresh-server checks")
                measured = report["checkpoints"][5:]
                rows.extend(measured)
                tails.append({"workload": f"{mode}-{repetition}",
                              "recovery_ms": distribution([row["recovery_ms"] for row in measured]),
                              "server_cpu_ms": distribution([row["timing"]["server_cpu_ms"] for row in measured])})
        fingerprints = sorted({row["result_sha256"] for row in rows})
        sides[side] = {
            "binary_sha256": report["provenance"]["binary_sha256"],
            "result_sha256": fingerprints,
            "recovery_ms": distribution([row["recovery_ms"] for row in rows]),
            "server_cpu_ms": distribution([row["timing"]["server_cpu_ms"] for row in rows]),
            "workloads": tails,
        }
    if (len(sides["control"]["result_sha256"]) != 1
            or sides["control"]["result_sha256"] != sides["candidate"]["result_sha256"]):
        raise ValueError("candidate and control result fingerprints differ")
    changes = {}
    for metric in ("recovery_ms", "server_cpu_ms"):
        changes[metric] = {}
        for statistic in ("median", "p95"):
            before = sides["control"][metric][statistic]
            after = sides["candidate"][metric][statistic]
            changes[metric][statistic] = {"delta": after - before,
                                         "percent": (after / before - 1) * 100 if before else None}
    return {"identity": identity, "sides": sides, "changes": changes,
            "notes": ["Three alternating repetitions of fixed/churn workloads; five warmup cycles excluded.",
                      "Same driver and fixture; distinct release server binaries; default yield policy.",
                      "CPU is server process work per complete editing cycle, not energy use."]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reports", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.write_text(json.dumps(summarize(args.reports), indent=2) + "\n")


if __name__ == "__main__":
    main()
