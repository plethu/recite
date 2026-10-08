"""Run the session matrix with one fresh driver process per observation."""

import argparse
import os
import subprocess
import sys
from pathlib import Path


def run(binary, output, *, control=None, native_trace=False):
    entry = Path(__file__).parents[1] / "lsp.py"
    for repetition in range(1, 4):
        before = ["native", "control"] if repetition % 2 else []
        after = ["control", "native"] if not repetition % 2 else []
        for side in [*before, "candidate", *after]:
            if side == "control" and control is None or side == "native" and not native_trace:
                continue
            server = control if side == "control" else binary
            reports = output if side == "candidate" else output / side
            for mode in ("fixed", "churn"):
                name = f"{mode}-{repetition}"
                command = [
                    sys.executable,
                    str(entry),
                    "endurance",
                    "--binary",
                    str(server),
                    "--output",
                    str(reports / f"{name}.json"),
                ]
                if mode == "churn":
                    command.append("--churn")
                environment = dict(os.environ)
                environment.pop("RECITE_LSP_TRACE_DIR", None)
                if side == "native":
                    trace = reports / name
                    trace.mkdir(parents=True, exist_ok=True)
                    environment["RECITE_LSP_TRACE_DIR"] = str(trace)
                subprocess.run(command, env=environment, check=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--control", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--native-trace", action="store_true")
    args = parser.parse_args(argv)
    run(
        args.binary.resolve(strict=True),
        args.output.resolve(),
        control=args.control.resolve(strict=True) if args.control else None,
        native_trace=args.native_trace,
    )
