"""Regression budgets for the native large-project probe (#109).

Budgets are deliberately generous: several times the timings measured on a 4-core software-rendered
development VM, so they catch order-of-magnitude regressions on shared CI runners without tracking
host noise. They are not release baselines; a stable reference machine owns those.
RECITE_LARGE_BUDGET_SCALE multiplies every budget, so a scale below 1 shows the gate failing.
"""

import os

# Measured on the 4-core development VM (bench build, cage pixman), large profile: cold open to
# diagnostics 0.66-0.74 s, switch and search medians 0.2-0.27 s, typing edit-to-paint p95 18-22 ms,
# RSS 304 MiB after open and 334 MiB after editing.
BUDGETS = {
    "large": {
        "cold_open_to_diagnostics_median_ms": 4000.0,
        "scene_switch_median_ms": 1000.0,
        "search_to_passage_median_ms": 1000.0,
        "typing_edit_to_paint_p95_ms": 80.0,
        "rss_after_open_mib": 640.0,
        "edit_cycle_rss_growth_mib": 48.0,
    },
    "medium": {
        "cold_open_to_diagnostics_median_ms": 2500.0,
        "scene_switch_median_ms": 1000.0,
        "search_to_passage_median_ms": 1000.0,
        "typing_edit_to_paint_p95_ms": 80.0,
        "rss_after_open_mib": 400.0,
        "edit_cycle_rss_growth_mib": 48.0,
    },
}


def measured(results):
    cycles = results.get("edit_cycles", [])
    return {
        "cold_open_to_diagnostics_median_ms": results.get("cold_open_to_diagnostics_ms", {}).get(
            "median"
        ),
        "scene_switch_median_ms": results.get("scene_switch_ms", {}).get("median"),
        "search_to_passage_median_ms": results.get("search_to_passage_ms", {}).get("median"),
        "typing_edit_to_paint_p95_ms": (results.get("typing_edit_to_paint_ms") or {}).get("p95"),
        "rss_after_open_mib": results.get("memory_after_open", {}).get("rss_kib", 0) / 1024 or None,
        "edit_cycle_rss_growth_mib": (cycles[-1]["rss_kib"] - cycles[0]["rss_kib"]) / 1024
        if len(cycles) > 1
        else None,
    }


def check(profile, results):
    """Record each budget beside its measurement; return the failures."""
    budgets = BUDGETS.get(profile)
    if budgets is None:
        results["budgets"] = None
        return []
    scale = float(os.environ.get("RECITE_LARGE_BUDGET_SCALE", "1"))
    values = measured(results)
    failures = []
    results["budgets"] = {"scale": scale, "checks": {}}
    for name, budget in budgets.items():
        limit = round(budget * scale, 1)
        value = values[name]
        passed = value is not None and value <= limit
        results["budgets"]["checks"][name] = {
            "measured": None if value is None else round(value, 1),
            "budget": limit,
            "passed": passed,
        }
        if not passed:
            failures.append(f"{name} {value} exceeds its budget {limit}")
    return failures
