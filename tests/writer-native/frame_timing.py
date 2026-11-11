"""Software-rendering frame pacing and edit-to-paint for typing in the Script view.

The client's own Wayland protocol log times each key press against the next main-surface commit
that attaches a buffer. These are client frame-readiness timings, not input-to-photon latency.
"""

import json
import os
import re
import shutil
import statistics
import subprocess
import tempfile
import time
import traceback
from pathlib import Path

from native_window import attach

ROOT = Path(__file__).resolve().parents[2]
TEXT = "The quick brown fox jumps over the lazy dog"
KEY_DELAY_MS = 60
# Budgets for the debug probe build on a CPU adapter. Vello CPU keeps frame pacing near the
# compositor's 60 Hz (about 17 ms); GPU Vello emulated by llvmpipe paces near 47 ms (#283).
FRAME_INTERVAL_P50_BUDGET_MS = 30.0
EDIT_TO_PAINT_P50_BUDGET_MS = 80.0
CPU_BACKEND = "imaging_vello_cpu"
LINE = re.compile(
    r"^\[\s*(\d+)\.(\d+)\]\s+(?:\{[^}]*\}\s+)?(->\s+)?([a-z_0-9]+)@(\d+)\.([a-z_0-9]+)\((.*)\)"
)
SELECTION = re.compile(r'Selected Masonry imaging backend backend="([^"]+)".*?device_type=(\w+)')
PRESENTER = re.compile(r'Selected Masonry imaging backend .*?presenter="([^"]+)"')
ANSI = re.compile(r"\x1b\[[0-9;]*m")
# libwayland stamps CLOCK_REALTIME microseconds modulo 2**32, printed as milliseconds.
WRAP_MS = 2**32 / 1000


def wayland_ms():
    return (time.time_ns() // 1000 % 2**32) / 1000


def unwrap(value, base):
    while value < base - WRAP_MS / 2:
        value += WRAP_MS
    return value


def protocol_events(log):
    events, base = [], None
    for raw in log.splitlines():
        match = LINE.match(raw)
        if not match:
            continue
        stamp = int(match.group(1)) + int(match.group(2)) / 1000
        base = stamp if base is None else base
        events.append(
            {
                "t": unwrap(stamp, base),
                "request": bool(match.group(3)),
                "interface": match.group(4),
                "id": int(match.group(5)),
                "message": match.group(6),
                "args": match.group(7),
            }
        )
    return events


def summary(values):
    if not values:
        return {"count": 0}
    ordered = sorted(values)

    def pick(fraction):
        return round(ordered[min(len(ordered) - 1, round(fraction * (len(ordered) - 1)))], 2)

    return {
        "count": len(values),
        "p50": pick(0.5),
        "p95": pick(0.95),
        "max": round(ordered[-1], 2),
        "mean": round(statistics.fmean(values), 2),
    }


def timings(events, start, end):
    commits = {}
    for event in events:
        if event["request"] and event["interface"] == "wl_surface" and event["message"] == "commit":
            commits[event["id"]] = commits.get(event["id"], 0) + 1
    surface = max(commits, key=commits.get) if commits else None
    frames, attached = [], False
    for event in events:
        if not event["request"] or event["interface"] != "wl_surface" or event["id"] != surface:
            continue
        if event["message"] == "attach":
            attached = True
        elif event["message"] == "commit" and attached:
            attached = False
            frames.append(event["t"])
    presses = [
        e["t"]
        for e in events
        if not e["request"]
        and e["interface"] == "wl_keyboard"
        and e["message"] == "key"
        and e["args"].rstrip().endswith(", 1")
        and start <= e["t"] <= end
    ]
    painted = [t for t in frames if start <= t]
    latencies, unpainted = [], 0
    for press in presses:
        frame = next((t for t in painted if t >= press), None)
        if frame is None:
            unpainted += 1
        else:
            latencies.append(frame - press)
    window = [t for t in painted if t <= end]
    return {
        "key_presses": len(presses),
        "unpainted_presses": unpainted,
        "edit_to_paint_ms": summary(latencies),
        "frame_interval_ms": summary([b - a for a, b in zip(window, window[1:])]),
    }


def project(temporary):
    root = Path(temporary) / "project"
    (root / "src").mkdir(parents=True)
    (root / "schema").mkdir()
    (root / "recite.project.toml").write_text(
        'format_version=1\n[project]\ncontent_set="relay-hub"\nschema="schema/writer_examples.json"\n'
        '[discovery]\nsource_roots=["src"]\n'
    )
    shutil.copyfile(
        ROOT / "fixtures/schema/valid/writer_examples.json",
        root / "schema/writer_examples.json",
    )
    shutil.copyfile(
        ROOT / "fixtures/recite/valid/writer_examples/relay_hub.recite",
        root / "src/relay_hub.recite",
    )
    return root


def run(api, bus, registry, binary, output):
    results = {
        "scope": "typing into Mara's inline line in the Relay Hub Script view",
        "text": TEXT,
        "key_delay_ms": KEY_DELAY_MS,
        "budgets_ms": {
            "frame_interval_p50": FRAME_INTERVAL_P50_BUDGET_MS,
            "edit_to_paint_p50": EDIT_TO_PAINT_P50_BUDGET_MS,
        },
    }
    process = None
    log_path = output.with_suffix(".writer.log")
    try:
        with tempfile.TemporaryDirectory(prefix="recite-frames-", dir="/tmp") as temporary:
            root = project(temporary)
            config = Path(temporary) / "config.toml"
            config.write_text(
                "config_version=1\n[writer]\nconfirm_exit=false\n"
                "[writer.presentation]\nui_scale=100\nsplit=false\n"
            )
            environment = dict(os.environ, RECITE_CONFIG=str(config), WAYLAND_DEBUG="client")
            environment.pop("RUST_LOG", None)
            route = "recite://writer/write?scene=src/relay_hub.recite&view=script"
            with log_path.open("w") as log:
                process = subprocess.Popen(
                    [str(binary), "--project", str(root), "--document", "src/relay_hub.recite"]
                    + ["--route", route],
                    env=environment,
                    stdout=log,
                    stderr=subprocess.STDOUT,
                )
                window = attach(api, bus, registry, process)
                window.wait(
                    lambda: window.find_text("If you're here", prefix=True, contains=True),
                    "Relay Desk line",
                )
                time.sleep(1.5)
                start = wayland_ms()
                subprocess.run(
                    ["wtype", "-d", str(KEY_DELAY_MS), "--", TEXT], check=True, timeout=60
                )
                end = wayland_ms()
                time.sleep(1.0)
                process.terminate()
                process.wait(timeout=10)
                process = None
        text = log_path.read_text(errors="replace")
        events = protocol_events(text)
        base = events[0]["t"]
        results.update(timings(events, unwrap(start, base), unwrap(end, base)))
        selection = SELECTION.search(ANSI.sub("", text))
        results["imaging_backend"] = selection.group(1) if selection else None
        results["adapter_device_type"] = selection.group(2) if selection else None
        presenter = PRESENTER.search(ANSI.sub("", text))
        results["presenter"] = presenter.group(1) if presenter else "wgpu"
        failures = []
        if selection is None:
            failures.append("Writer did not report its imaging backend")
        elif selection.group(2) == "Cpu" and selection.group(1) != CPU_BACKEND:
            failures.append(f"a CPU adapter selected {selection.group(1)}, not {CPU_BACKEND}")
        if results["key_presses"] < len(TEXT) or results["unpainted_presses"]:
            failures.append("every typed key must be followed by a painted frame")
        frame = results["frame_interval_ms"].get("p50")
        if frame is None or frame > FRAME_INTERVAL_P50_BUDGET_MS:
            failures.append(f"frame interval p50 {frame} ms exceeds its budget")
        latency = results["edit_to_paint_ms"].get("p50")
        if latency is None or latency > EDIT_TO_PAINT_P50_BUDGET_MS:
            failures.append(f"edit-to-paint p50 {latency} ms exceeds its budget")
        if failures:
            results["error"] = "; ".join(failures)
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
    finally:
        if process is not None:
            process.terminate()
            process.wait(timeout=10)
    output.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")
    return "error" not in results
