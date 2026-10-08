"""Commands for external LSP measurements and fail-closed report checks."""

import argparse
import sys

from . import (
    comparison,
    edits,
    endurance,
    faults,
    latency,
    matrix,
    native_report,
    performance,
    recovery,
    session,
)

COMMANDS = {
    "latency": latency.main,
    "edits": edits.main,
    "session": session.main,
    "endurance": endurance.main,
    "matrix": matrix.main,
    "compare": performance.main,
    "recovery": recovery.main,
    "faults": faults.main,
    "server-comparison": comparison.main,
    "native-summary": native_report.main,
}


def main():
    args = sys.argv[1:]
    if args and args[0] in COMMANDS:
        COMMANDS[args[0]](args[1:])
        return
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=COMMANDS)
    parser.parse_args(args)
