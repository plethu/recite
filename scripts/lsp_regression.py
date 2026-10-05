"""Paired LSP latency decisions; independent of the measurement transport."""

import math
import statistics


def regressions(pairs, ratio, absolute_ms, expected_workloads, limits=None):
    if len(pairs) != 3:
        raise ValueError("a decision requires three alternating pairs")
    kinds = set(pairs[0]["control"]["workloads"])
    if not kinds or kinds != set(expected_workloads):
        raise ValueError("workload coverage differs from policy")
    if set(limits or {}) - kinds:
        raise ValueError("threshold names differ from workload coverage")
    findings = []
    for pair in pairs:
        for side in ("control", "candidate"):
            result = pair[side]
            if "fanout/block_topology" in kinds and not result.get("fanout_fixture_files"):
                raise ValueError("fanout fixture identity is missing")
            if set(result["workloads"]) != kinds or set(result["diagnostics"]) != kinds:
                raise ValueError("workload coverage differs")
            for kind in kinds:
                values = result["workloads"][kind]
                if len(values) < 21 or len(values) != len(result["diagnostics"][kind]):
                    raise ValueError("incomplete observations")
                if any(not math.isfinite(value) or value <= 0 for value in values):
                    raise ValueError("invalid timing observation")
        if pair["control"].get("fanout_fixture_files") != pair["candidate"].get("fanout_fixture_files"):
            raise ValueError("fanout fixture identity differs")
        if pair["control"]["files"] != pair["candidate"]["files"]:
            raise ValueError("fixture identity differs")
        if pair["control"]["diagnostics"] != pair["candidate"]["diagnostics"]:
            raise ValueError("diagnostics differ between revisions")
    for kind in sorted(kinds):
        limit = (limits or {}).get(kind, {})
        relative = limit.get("ratio", ratio)
        absolute = limit.get("absolute", absolute_ms)
        if limit.get("unit", "ms") not in ("ms", "KiB"):
            raise ValueError("unknown measurement unit")
        if not math.isfinite(relative) or relative <= 1 or not math.isfinite(absolute) or absolute <= 0:
            raise ValueError("invalid regression threshold")
        controls = [statistics.median(p["control"]["workloads"][kind]) for p in pairs]
        candidates = [statistics.median(p["candidate"]["workloads"][kind]) for p in pairs]
        misses = sum(c > b * relative and c - b > absolute for b, c in zip(controls, candidates))
        before, after = statistics.median(controls), statistics.median(candidates)
        if misses >= 2 and after > before * relative and after - before > absolute:
            findings.append({"workload": kind, "control": before, "candidate": after, "unit": limit.get("unit", "ms"),
                             "ratio": after / before, "regressing_pairs": misses})
    return findings
