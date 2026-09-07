#!/usr/bin/env python3
"""Capture only GPUI's foreign toplevel, on a silent floating workspace."""
import importlib.util
import json
import os
from pathlib import Path
import signal
import struct
import subprocess
import time
import uuid

BASE = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("smoke", Path(__file__).with_name("smoke-candidates.py"))
smoke = importlib.util.module_from_spec(spec)
spec.loader.exec_module(smoke)


def main():
    output = BASE / "captures/gpui"
    output.mkdir(parents=True, exist_ok=True)
    evidence = []
    capture_size = None
    for mode in ("script-light", "script-dark", "source-light", "source-dark"):
        identity = "recite-capture-" + uuid.uuid4().hex
        handle = identity.replace("-", "_")
        process = None
        try:
            smoke.hypr("eval", f'{handle} = hl.window_rule({{name="{handle}", match={{class="{identity}"}}, float=true, size={{1200,800}}, workspace="name:{identity} silent", no_initial_focus=true, focus_on_activate=false, no_follow_mouse=true, render_unfocused=true, no_anim=true}})')
            with (output / f"{mode}.log").open("w") as log:
                process = subprocess.Popen(
                    [str(BASE / "candidates/gpui/target/debug/recite-bakeoff-gpui")],
                    env=os.environ | {"RECITE_BAKEOFF_WINDOW_ID": identity, "RECITE_BAKEOFF_CAPTURE_VIEW": mode},
                    stdout=log, stderr=log, start_new_session=True)
                client = None
                deadline = time.monotonic() + 15
                while time.monotonic() < deadline and process.poll() is None:
                    client = next((c for c in json.loads(smoke.hypr("-j", "clients")) if c["class"] == identity), None)
                    if client:
                        break
                    time.sleep(.1)
                if client is None:
                    raise RuntimeError(f"{mode}: no mapped window")
                if not client["floating"] or client["workspace"]["name"] != identity or client["size"] != [1200, 800]:
                    raise RuntimeError(f"{mode}: unexpected placement")
                time.sleep(2)
                path = output / f"{mode}.png"
                subprocess.run(["grim", "-s", "1", "-T", client["stableId"], str(path)], check=True, timeout=10)
                data = path.read_bytes()
                size = struct.unpack(">II", data[16:24])
                if data[:8] != b"\x89PNG\r\n\x1a\n" or size[0] < 1200 or size[1] < 800 or abs(size[0] / size[1] - 1.5) > .002 or (capture_size is not None and size != capture_size):
                    raise RuntimeError(f"{mode}: unexpected capture size")
                capture_size = size
                evidence.append({"view": mode, "logical_size": client["size"], "pixel_size": size, "floating": True, "workspace": "isolated", "capture": "foreign toplevel only"})
                print(path, flush=True)
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

    (output / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")

if __name__ == "__main__":
    signal.signal(signal.SIGTERM, smoke.interrupted)
    main()
