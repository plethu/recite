#!/usr/bin/env python3
"""Exercise accumulated state in bounded cycles, keeping one server per workload."""

import argparse
import importlib.util
import json
import math
import os
import platform
import sys
from pathlib import Path
import random
import tempfile
import time

from lsp_fanout import generate
from lsp_measurement import provenance
from lsp_session_health import assess, resources
from lsp_session_workload import Session

spec = importlib.util.spec_from_file_location("latency", Path(__file__).with_name("measure-lsp-latency.py"))
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


def settled_idle(session):
    cpu = session.timing.cpu_seconds()
    started = time.perf_counter()
    time.sleep(3)
    return {"elapsed_ms": (time.perf_counter() - started) * 1000,
            "server_cpu_ms": (session.timing.cpu_seconds() - cpu) * 1000}


def run(binary, root, output, cycles, edits, seed, churn, server_env, edit_interval_ms=5,
        editing_cpu_accounting=False, startup_idle_seconds=0):
    report = {"provenance": provenance(binary, root), "result_fingerprint_version": 2,
              "cycles": cycles, "edits_per_cycle": edits,
              "seed": seed, "churn": churn, "checkpoints": [], "status": "incomplete",
              "driver": {"python": platform.python_version(), "switch_interval_ms": sys.getswitchinterval() * 1000,
                         "native_trace": bool(os.environ.get("RECITE_LSP_TRACE_DIR")),
                         "server_environment": server_env, "edit_interval_ms": edit_interval_ms,
                         "editing_cpu_accounting": editing_cpu_accounting,
                         "settled_idle_seconds": 3,
                         "settled_idle_location": "startup" if startup_idle_seconds else "after_cycles"}}
    generator = random.Random(seed)
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="recite-session-config-") as config:
        with output.with_suffix(".jsonl").open("w") as trace:
            session = Session(probe, binary, root, Path(config), trace, server_env=server_env)
            report["server_pid"] = session.client.process.pid
            try:
                session.start()
                baseline = session.checkpoint()["result_sha256"]
                if startup_idle_seconds:
                    report["settled_idle"] = settled_idle(session)
                for cycle in range(cycles):
                    assert time.monotonic() - started < 600, "session exceeded ten-minute budget"
                    recovery = session.cycle(cycle, churn, edits, generator, edit_interval_ms=edit_interval_ms,
                                             editing_cpu_accounting=editing_cpu_accounting)
                    checkpoint = {"cycle": cycle, **recovery, **session.checkpoint(),
                                  **resources(session.client.process.pid)}
                    assert checkpoint["result_sha256"] == baseline, "persistent results drifted"
                    if cycle % 10 == 0 or cycle == cycles - 1:
                        with tempfile.TemporaryDirectory(prefix="recite-session-oracle-") as oracle_config:
                            oracle = Session(probe, binary, root, Path(oracle_config), trace, server_env=server_env)
                            try:
                                oracle.start()
                                assert oracle.checkpoint()["result_sha256"] == baseline, "fresh server disagrees"
                                oracle.client.close()
                            except BaseException:
                                oracle.client.abort()
                                raise
                        checkpoint["fresh_oracle_matched"] = True
                    report["checkpoints"].append(checkpoint)
                    output.write_text(json.dumps(report, indent=2) + "\n")
                    print(f"{'churn' if churn else 'fixed'} cycle {cycle + 1}/{cycles}: "
                          f"{checkpoint['rss_bytes'] / 1024**2:.1f} MiB", flush=True)
                report["health"] = assess(report["checkpoints"])
                report["status"] = report["health"]["status"]
                if not startup_idle_seconds:
                    report["settled_idle"] = settled_idle(session)
                # Includes initialization and checkpoint queries outside the
                # timed lifecycle stages, with only one accounting read here.
                report["server_cpu_ms_total"] = session.timing.cpu_seconds() * 1000
                session.client.close()
            except BaseException as error:
                report["status"] = "error"
                report["error"] = str(error)
                session.client.abort()
                raise
            finally:
                report["duration_seconds"] = time.monotonic() - started
                output.write_text(json.dumps(report, indent=2) + "\n")
    return report["status"] == "pass"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cycles", type=int, default=40)
    parser.add_argument("--edits", type=int, default=50)
    parser.add_argument("--edit-interval-ms", type=float, default=5, help="Pacing between burst edits; default remains the calibrated 200 Hz workload")
    parser.add_argument("--editing-cpu-accounting", action="store_true",
                        help="Opt-in CPU reads around the editing stage; may perturb scheduling")
    parser.add_argument("--startup-idle-seconds", type=int, choices=(0, 3), default=0,
                        help="Diagnostic: move the three-second idle sample before timed cycles")
    parser.add_argument("--seed", type=int, default=7203)
    parser.add_argument("--churn", action="store_true")
    parser.add_argument("--server-yield-to-zero", choices=("0", "1"),
                        help="Diagnostic macOS libpthread setting applied only to server children")
    parser.add_argument("--driver-switch-ms", type=float, default=None,
                        help="Override Python driver thread-switch interval for experiments (default: interpreter setting)")
    args = parser.parse_args()
    if args.cycles < 20 or args.edits < 1:
        parser.error("require >=20 cycles and positive edits per cycle")
    if not math.isfinite(args.edit_interval_ms) or not 1 <= args.edit_interval_ms <= 1000:
        parser.error("edit interval must be finite and between 1 and 1000 ms")
    if args.driver_switch_ms is not None:
        if not math.isfinite(args.driver_switch_ms) or args.driver_switch_ms <= 0:
            parser.error("driver switch interval must be positive and finite")
        sys.setswitchinterval(args.driver_switch_ms / 1000)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="recite-endurance-") as directory:
        root = Path(directory).resolve()
        generate(root, documents=40, blocks=20, lines=20, shared_destinations=10)
        server_env = ({"PTHREAD_YIELD_TO_ZERO": args.server_yield_to_zero}
                      if args.server_yield_to_zero is not None else {})
        passed = run(args.binary.resolve(), root, args.output, args.cycles, args.edits, args.seed, args.churn, server_env, args.edit_interval_ms,
                     args.editing_cpu_accounting, args.startup_idle_seconds)
    raise SystemExit(0 if passed else 1)


if __name__ == "__main__":
    main()
