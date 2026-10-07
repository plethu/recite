#!/usr/bin/env python3
"""Compare two release servers in alternating pairs, confirming suspected regressions."""

import argparse
import json
import math
import tempfile
from pathlib import Path

from . import edits, latency
from .fanout import generate as generate_fanout
from .lifecycle import measure as measure_lifecycle
from .measurement import environment, provenance
from .regression import regressions


def compare(control, candidate, root, output, policy):
    before = environment()
    report = {
        "policy": policy,
        "control": provenance(control, root),
        "candidate": provenance(candidate, root),
        "rounds": [],
        "status": "incomplete",
    }
    output.parent.mkdir(parents=True, exist_ok=True)

    def save():
        output.write_text(json.dumps(report, indent=2) + "\n")

    save()
    try:
        for round_index in range(2):
            pairs = []
            report["rounds"].append({"pairs": pairs})
            for pair_index in range(3):
                pair = {}
                pairs.append(pair)
                order = (
                    ("control", "candidate")
                    if (pair_index + round_index) % 2 == 0
                    else ("candidate", "control")
                )
                for side in order:
                    binary = control if side == "control" else candidate
                    pair[side] = edits.measure(binary, root, policy["samples"])
                    negotiated = edits.measure(binary, root, policy["samples"], ranged=None)
                    if negotiated["files"] != pair[side]["files"]:
                        raise RuntimeError("fixture changed between sync modes")
                    pair[side]["negotiated_sync_mode"] = negotiated["sync_mode"]
                    for name, rows in negotiated["workloads"].items():
                        pair[side]["workloads"][f"negotiated/{name}"] = rows
                        pair[side]["diagnostics"][f"negotiated/{name}"] = negotiated["diagnostics"][
                            name
                        ]
                    with tempfile.TemporaryDirectory(prefix="recite-lsp-queries-") as config:
                        interactive = latency.run(binary, root, policy["samples"], Path(config))
                    with tempfile.TemporaryDirectory(prefix="recite-lsp-fanout-") as fixture:
                        generate_fanout(Path(fixture), **policy["fanout"])
                        fanout = edits.measure(
                            binary,
                            Path(fixture),
                            policy["samples"],
                            ranged=None,
                            workloads=("block_topology",),
                        )
                    pair[side]["fanout_fixture_files"] = fanout["files"]
                    pair[side]["workloads"]["fanout/block_topology"] = fanout["workloads"][
                        "block_topology"
                    ]
                    pair[side]["diagnostics"]["fanout/block_topology"] = fanout["diagnostics"][
                        "block_topology"
                    ]
                    pair[side]["interactive"] = interactive
                    for name, rows in interactive["requests"].items():
                        key = f"query/{name}"
                        pair[side]["workloads"][key] = [row["ms"] for row in rows]
                        pair[side]["diagnostics"][key] = [
                            {"sha256": row["result_sha256"], "bytes": row["result_bytes"]}
                            for row in rows
                        ]
                    lifecycle = measure_lifecycle(edits.probe, binary, root, policy["samples"])
                    pair[side]["workloads"].update(lifecycle["workloads"])
                    pair[side]["diagnostics"].update(lifecycle["diagnostics"])
                    pair[side]["environment_after"] = environment()
                    save()
                    print(
                        f"round {round_index + 1}, pair {pair_index + 1}, {side} complete",
                        flush=True,
                    )
                    if environment() != before:
                        raise RuntimeError("execution profile changed during measurement")
            findings = regressions(
                pairs,
                policy["ratio"],
                policy["absolute_ms"],
                policy["workloads"],
                policy.get("limits", {}),
            )
            report["rounds"][-1]["regressions"] = findings
            if not findings:
                report["status"] = "pass"
                return True
            if round_index == 1:
                confirmed = {r["workload"] for r in report["rounds"][0]["regressions"]} & {
                    r["workload"] for r in findings
                }
                report["status"] = "regression" if confirmed else "unstable"
                return False
            print("Suspected regression; repeating three pairs before deciding.", flush=True)
        raise AssertionError("unreachable comparison state")
    except BaseException as error:
        report["error"] = str(error)
        raise
    finally:
        save()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--control", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--project", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--policy", type=Path, default=Path("scripts/lsp-performance-policy.json"))
    args = parser.parse_args(argv)
    policy = json.loads(args.policy.read_text())
    if (
        not isinstance(policy["samples"], int)
        or policy["samples"] < 21
        or not math.isfinite(policy["ratio"])
        or policy["ratio"] <= 1
        or not math.isfinite(policy["absolute_ms"])
        or policy["absolute_ms"] <= 0
    ):
        parser.error("policy requires >=21 samples, ratio >1 and positive absolute tolerance")
    passed = compare(
        args.control.resolve(),
        args.candidate.resolve(),
        args.project.resolve(),
        args.output,
        policy,
    )
    raise SystemExit(0 if passed else 1)
