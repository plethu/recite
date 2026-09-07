#!/usr/bin/env python3
"""Capture both entries without using the active Hyprland workspace (0.56+)."""
import json
import os
from pathlib import Path
import signal
import struct
import subprocess
import tempfile
import uuid

BASE = Path(__file__).resolve().parents[1]
ROOT = BASE.parents[1]


def hypr(*args):
    result = subprocess.run(["hyprctl", *args], check=True, capture_output=True, text=True)
    if args[0] == "eval" and result.stdout.strip() != "ok":
        raise RuntimeError(result.stdout.strip())
    return result.stdout


def test(candidate, directory, extra_env=None):
    env = os.environ | {"RECITE_BAKEOFF_CAPTURE_DIR": str(directory)} | (extra_env or {})
    args = ["mise", "exec", "--", "cargo", "test", "--locked", "--manifest-path",
            str(BASE / "Cargo.toml"), "-p", f"recite-bakeoff-{candidate}", "--test", "scene"]
    args += (["whole_scene_source_and_dark_theme_render"] if candidate == "freya"
             else ["--", "--ignored", "--test-threads=1"])
    process = subprocess.Popen(args, cwd=ROOT, env=env, start_new_session=True)
    try:
        code = process.wait(timeout=180)
        if code:
            raise subprocess.CalledProcessError(code, args)
    finally:
        # Cargo can leave its test child alive on timeout or interruption.
        # This process group belongs exclusively to this invocation.
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


def interrupted(signum, _frame):
    raise SystemExit(128 + signum)


def main():
    signal.signal(signal.SIGTERM, interrupted)
    token = uuid.uuid4().hex
    app_id = f"org.recite.Capture.r{token}"
    handle = f"recite_capture_{token}"
    workspace = f"recite-capture-{token}"
    # The rule is installed before GTK maps, and matches only this invocation.
    class_pattern = app_id.replace(".", "[.]")
    rule = (f'{handle} = hl.window_rule({{ name = "{handle}", '
            f'match = {{ class = "{class_pattern}" }}, '
            f'workspace = "name:{workspace} silent", float = true, '
            'size = {1200, 800}, no_initial_focus = true, focus_on_activate = false, '
            'no_follow_mouse = true, no_anim = true, render_unfocused = true })')
    with tempfile.TemporaryDirectory(prefix="recite-captures-") as temporary:
        staging = Path(temporary)
        test("freya", staging / "freya")
        try:
            hypr("eval", rule)
            test("gtk", staging / "gtk", {"RECITE_BAKEOFF_CAPTURE_APP_ID": app_id,
                                           "GDK_BACKEND": "wayland"})
        finally:
            # 0.56 exposes disabling, not removal. Drop the Lua reference too;
            # the disabled rule is discarded on the next normal config reload.
            hypr("eval", f'if {handle} then {handle}:set_enabled(false); {handle} = nil end')
        clients = json.loads(hypr("-j", "clients"))
        if any(client["class"] == app_id for client in clients):
            raise RuntimeError("Capture window still exists after test completion")
        for candidate in ("freya", "gtk"):
            paths = sorted((staging / candidate).glob("*.png"))
            if len(paths) != 4:
                raise RuntimeError(f"Expected four {candidate} captures")
            for path in paths:
                if struct.unpack(">II", path.read_bytes()[16:24]) != (1200, 800):
                    raise RuntimeError(f"Unexpected capture size: {path}")
        for candidate in ("freya", "gtk"):
            destination = BASE / "captures" / candidate
            destination.mkdir(parents=True, exist_ok=True)
            for path in (staging / candidate).glob("*.png"):
                (destination / path.name).write_bytes(path.read_bytes())
    print("Saved eight 1200 × 800 captures; capture rule disabled and window closed.")


if __name__ == "__main__":
    main()
