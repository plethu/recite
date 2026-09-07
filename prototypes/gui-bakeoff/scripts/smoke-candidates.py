#!/usr/bin/env python3
"""Check native startup in silent, floating Hyprland workspaces. No input injection."""
import json
import os
from pathlib import Path
import signal
import subprocess
import time
import uuid

BASE = Path(__file__).resolve().parents[1]

def hypr(*args):
    result = subprocess.run(["hyprctl", *args], capture_output=True, text=True, check=True)
    if args[0] == "eval" and result.stdout.strip() != "ok":
        raise RuntimeError(result.stdout)
    return result.stdout


def main():
    results = []
    for name in ("floem", "gpui", "xilem"):
        identity = "recite-probe-" + uuid.uuid4().hex
        handle = identity.replace("-", "_")
        workspace = "name:" + identity
        selector = "class" if name == "gpui" else "title"
        process = None
        log_path = BASE / "candidates" / name / "startup.log"
        try:
            hypr("eval", f'{handle} = hl.window_rule({{name="{handle}", match={{{selector}="{identity}"}}, float=true, size={{1200,800}}, workspace="{workspace} silent", no_initial_focus=true, focus_on_activate=false, no_follow_mouse=true, render_unfocused=true, no_anim=true}})')
            with log_path.open("w") as log:
                process = subprocess.Popen([str(BASE / "candidates" / name / "target/debug" / f"recite-bakeoff-{name}")],
                    env=os.environ | {"RECITE_BAKEOFF_WINDOW_ID": identity, "WINIT_UNIX_BACKEND": "wayland"},
                    stdout=log, stderr=log, start_new_session=True)
                client = None
                deadline = time.monotonic() + 15
                while time.monotonic() < deadline and process.poll() is None:
                    clients = json.loads(hypr("-j", "clients"))
                    client = next((c for c in clients if c[selector] == identity), None)
                    if client:
                        break
                    time.sleep(.1)
                if client is None:
                    raise RuntimeError(f"{name}: no mapped window; see {log_path}")
                if not client["floating"] or client["workspace"]["name"] != identity or client["size"] != [1200,800]:
                    raise RuntimeError(f"{name}: unexpected window placement")
                time.sleep(2)
                if process.poll() is not None:
                    raise RuntimeError(f"{name}: exited after mapping; see {log_path}")
                results.append({"candidate": name, "mapped": True, "floating": True, "size": client["size"], "workspace": "isolated", "input_tested": False})
        except RuntimeError as error:
            results.append({"candidate": name, "mapped": False, "error": str(error), "input_tested": False})
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
            hypr("eval", f'if {handle} then {handle}:set_enabled(false); {handle}=nil end')
    (BASE / "candidates/startup-evidence.json").write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results, indent=2))


def interrupted(signum, _frame):
    raise SystemExit(128 + signum)


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, interrupted)
    main()
