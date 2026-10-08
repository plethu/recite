#!/usr/bin/env python3
"""Validate required CI results against the scope job's explicit decisions."""

import json
import os
import sys

from ci_scope_config import LANES


def failures(needs):
    if not isinstance(needs, dict):
        return ["required job results must be an object"]
    errors = []
    jobs = {job: value if isinstance(value, dict) else {} for job, value in needs.items()}
    for job in ("changes", "git-policy"):
        if jobs.get(job, {}).get("result") != "success":
            errors.append(f"{job} did not succeed")
    scope = jobs.get("changes", {}).get("outputs", {})
    if not isinstance(scope, dict):
        scope = {}
    if set(scope) != LANES:
        errors.append("scope outputs are missing or unexpected")
    if set(needs) != LANES | {"changes", "git-policy"}:
        errors.append("required jobs are missing or unexpected")
    for lane in sorted(LANES):
        selection = scope.get(lane)
        if selection not in ("true", "false"):
            errors.append(f"{lane}: invalid selection {selection!r}")
            continue
        result = jobs.get(lane, {}).get("result")
        expected = "success" if selection == "true" else "skipped"
        if result != expected:
            errors.append(f"{lane}: expected {expected}, got {result}")
    return errors


if __name__ == "__main__":
    errors = failures(json.loads(os.environ["RECITE_CI_NEEDS"]))
    if errors:
        print("\n".join(errors), file=sys.stderr)
        sys.exit(1)
    print("Every selected CI lane passed; unaffected lanes were skipped.")
