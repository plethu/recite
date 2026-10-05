#!/usr/bin/env python3
"""Compare paired tracing overhead and attribute the recovery requests."""

import argparse
from collections import defaultdict
import json
from pathlib import Path

from lsp_session_native import distribution, recovery_samples, summarize_samples


def summarize(directory):
    output, collected = {}, defaultdict(list)
    identity = None
    for label, root in (("disabled", directory), ("enabled", directory / "native")):
        recoveries, cpu, tails = [], [], []
        for mode in ("fixed", "churn"):
            for repetition in range(1, 4):
                path = root / f"{mode}-{repetition}.json"
                report = json.loads(path.read_text())
                observed = {key: report["provenance"][key] for key in ("binary_sha256", "files", "harness_revision")}
                if identity is not None and observed != identity:
                    raise ValueError("native experiment requires the same binary, fixture and harness")
                identity = observed
                if (report["cycles"] != 40 or report["edits_per_cycle"] != 50
                        or report["status"] != "pass" or report["health"]["status"] != "pass"
                        or len(report["checkpoints"]) != 40
                        or report["driver"]["native_trace"] != (label == "enabled")):
                    raise ValueError("native experiment requires complete matching workloads")
                rows = report["checkpoints"][5:]
                values = [row["recovery_ms"] for row in rows]
                recoveries.extend(values)
                tails.append({"workload": path.stem, "p95_ms": distribution(values)["p95"]})
                cpu.extend(row["timing"]["server_cpu_ms"] for row in rows)
                if label == "enabled":
                    for operation, samples in recovery_samples(path).items():
                        collected[operation].extend(samples)
        output[label] = {"recovery_ms": distribution(recoveries), "server_cpu_ms": distribution(cpu),
                         "workload_p95": tails}
    return {"identity": identity, "tracing_overhead": output,
            "recovery_requests": {key: summarize_samples(samples) for key, samples in collected.items()},
            "notes": ["Warmup: five cycles per workload; six workloads per tracing mode.",
                      "Native ingress begins after the lsp-server reader channel, handoff ends after the writer channel send.",
                      "Outside-server time is signed and includes uninstrumented transport and client scheduling.",
                      "Synchronous trace output can perturb scheduling; enabled/disabled runs alternate ordering."]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reports", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.write_text(json.dumps(summarize(args.reports), indent=2) + "\n")


if __name__ == "__main__":
    main()
