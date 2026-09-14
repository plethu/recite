#!/usr/bin/env python3
"""Launch one writer on a silent floating workspace; never inject desktop input."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import time
import uuid

BASE = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("smoke", Path(__file__).with_name("smoke-candidates.py"))
smoke = importlib.util.module_from_spec(spec)
spec.loader.exec_module(smoke)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", choices=("freya", "gpui"))
    parser.add_argument("--smoke", action="store_true", help="Verify placement and close without interaction.")
    parser.add_argument("--project", type=Path, help="Open Freya's file-backed mode at this path.")
    args = parser.parse_args()
    if args.project and args.candidate != "freya":
        parser.error("--project is only available for the selected Freya entry")
    identity = "recite-writer-" + uuid.uuid4().hex
    handle = identity.replace("-", "_")
    selector = "title" if args.candidate == "freya" else "class"
    target = BASE if args.candidate == "freya" else BASE / "candidates/gpui"
    binary = target / "target/debug" / f"recite-bakeoff-{args.candidate}"
    if not binary.is_file():
        parser.error(f"Build the candidate first: {binary}")
    process = None
    try:
        smoke.hypr("eval", f'{handle} = hl.window_rule({{name="{handle}", match={{{selector}="{identity}"}}, float=true, size={{1200,800}}, workspace="name:{identity} silent", no_initial_focus=true, focus_on_activate=false, no_follow_mouse=true, render_unfocused=true, no_anim=true}})')
        env = os.environ | {"RECITE_BAKEOFF_WINDOW_ID": identity}
        env.pop("RECITE_BAKEOFF_CAPTURE_VIEW", None)
        command = [str(binary)]
        if args.project:
            command.extend(["--project", str(args.project.resolve())])
        process = subprocess.Popen(command, env=env, start_new_session=True)
        client = None
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline and process.poll() is None:
            client = next((c for c in json.loads(smoke.hypr("-j", "clients")) if c[selector] == identity), None)
            if client:
                break
            time.sleep(.1)
        if client is None or not client["floating"] or client["size"] != [1200, 800] or client["workspace"]["name"] != identity:
            raise RuntimeError("Writer did not open at the expected isolated allocation")
        notice = "File-backed mode; use Save explicitly." if args.project else "Temporary session; no files are saved."
        print(f"{args.candidate}: floating, 1200 × 800. {notice}", flush=True)
        if args.smoke:
            time.sleep(1)
            if process.poll() is not None:
                raise RuntimeError("Writer exited during startup check")
        else:
            print(f"Switch when ready: hyprctl eval 'hl.dispatch(hl.dsp.focus({{workspace=\"name:{identity}\"}}))'", flush=True)
            print("Close the window or press Ctrl+C here to end the session.", flush=True)
            if process.wait() != 0:
                raise RuntimeError("Writer exited unsuccessfully")
    finally:
        if process:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=5)
            finally:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait()
        smoke.hypr("eval", f'if {handle} then {handle}:set_enabled(false); {handle}=nil end')


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, smoke.interrupted)
    main()
