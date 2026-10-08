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
from .measurement import environment, fixture_files, provenance
from .regression import regressions


def family(workload):
    prefix, separator, _ = workload.partition("/")
    if not separator:
        return "edits"
    return "startup" if prefix in ("lifecycle", "memory") else prefix


def measure(binary, root, policy, workloads):
    families = {family(name) for name in workloads}
    files = fixture_files(root)
    result = {"workloads": {}, "diagnostics": {}}
    if "edits" in families:
        result = edits.measure(binary, root, policy["samples"])
    if "negotiated" in families:
        negotiated = edits.measure(binary, root, policy["samples"], ranged=None)
        result["negotiated_sync_mode"] = negotiated["sync_mode"]
        for name, rows in negotiated["workloads"].items():
            result["workloads"][f"negotiated/{name}"] = rows
            result["diagnostics"][f"negotiated/{name}"] = negotiated["diagnostics"][name]
    if "query" in families:
        with tempfile.TemporaryDirectory(prefix="recite-lsp-queries-") as config:
            interactive = latency.run(binary, root, policy["samples"], Path(config))
        result["interactive"] = interactive
        for name, rows in interactive["requests"].items():
            key = f"query/{name}"
            result["workloads"][key] = [row["ms"] for row in rows]
            result["diagnostics"][key] = [
                {"sha256": row["result_sha256"], "bytes": row["result_bytes"]} for row in rows
            ]
    if "fanout" in families:
        with tempfile.TemporaryDirectory(prefix="recite-lsp-fanout-") as fixture:
            generate_fanout(Path(fixture), **policy["fanout"])
            fanout = edits.measure(
                binary,
                Path(fixture),
                policy["samples"],
                ranged=None,
                workloads=("block_topology",),
            )
        result["fanout_fixture_files"] = fanout["files"]
        result["workloads"]["fanout/block_topology"] = fanout["workloads"]["block_topology"]
        result["diagnostics"]["fanout/block_topology"] = fanout["diagnostics"]["block_topology"]
    if "startup" in families:
        lifecycle = measure_lifecycle(edits.probe, binary, root, policy["samples"])
        result["workloads"].update(lifecycle["workloads"])
        result["diagnostics"].update(lifecycle["diagnostics"])
    if fixture_files(root) != files:
        raise RuntimeError("fixture changed during measurement")
    result["files"] = files
    return result


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
        workloads = policy["workloads"]
        suspected = []
        for round_index in range(2):
            pairs = []
            report["rounds"].append(
                {"pairs": pairs, "workloads": workloads, "suspected_workloads": suspected}
            )
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
                    pair[side] = measure(binary, root, policy, workloads)
                    if pair[side]["files"] != report[side]["files"]:
                        raise ValueError("fixture identity differs from comparison provenance")
                    pair[side]["environment_after"] = environment()
                    save()
                    print(
                        f"round {round_index + 1}, pair {pair_index + 1}, {side} complete",
                        flush=True,
                    )
                    if environment() != before:
                        raise RuntimeError("execution profile changed during measurement")
            limits = policy.get("limits", {})
            if round_index:
                limits = {name: limit for name, limit in limits.items() if name in workloads}
            findings = regressions(
                pairs,
                policy["ratio"],
                policy["absolute_ms"],
                workloads,
                limits,
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
            suspected = [finding["workload"] for finding in findings]
            families = {family(name) for name in suspected}
            workloads = [name for name in policy["workloads"] if family(name) in families]
            print("Suspected regression; confirming affected probe families.", flush=True)
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
