"""Native Writer timings and memory on a large generated project (#109).

The project comes from `recite-fixturegen` (`fixtures/synthetic/profiles.toml`). Each timer starts
when the probe sends its input and stops when the accessibility tree first shows the result, polled
every 10 ms, so it includes Writer's work, its frame, the AT-SPI update and up to one poll. Frame
readiness after typing comes from the client's own Wayland log, as in `frame_timing.py`.
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

from frame_timing import protocol_events, timings, unwrap, wayland_ms
from native_window import attach, key
from project_interactions import manuscript
from startup_interactions import fill

ROOT = Path(__file__).resolve().parents[2]
LINE = re.compile(r"^\s*> [^\n]*@(\w+)\n\s+([^\n]+)$", re.M)
BLOCK = re.compile(r"^:: (\w+)", re.M)
POLL_SECONDS = 0.01
TYPED = "steady "
KEY_DELAY_MS = 60
SWITCHES = 4
SEARCHES = 3


def fixturegen():
    for profile in ("release", "debug"):
        path = ROOT / "target" / profile / "recite-fixturegen"
        if path.is_file():
            return path
    raise RuntimeError("build recite-fixturegen first: cargo build --release -p recite-fixturegen")


def generate(profile, directory):
    project = directory / "project"
    subprocess.run(
        [
            str(fixturegen()),
            "--profiles",
            str(ROOT / "fixtures/synthetic/profiles.toml"),
            "--profile",
            profile,
            "--output",
            str(project),
        ],
        check=True,
        timeout=300,
        capture_output=True,
    )
    manifest = project / "recite.project.toml"
    manifest.write_text(manifest.read_text() + '\n[discovery]\ncatalogs = ["locales/*.po"]\n')
    return project


def inventory(project):
    """Every line with its file, block and text, in source order."""
    lines = []
    for path in sorted((project / "src").glob("*.recite")):
        text = path.read_text()
        blocks = [(match.start(), match.group(1)) for match in BLOCK.finditer(text)]
        for match in LINE.finditer(text):
            block = next(name for start, name in reversed(blocks) if start <= match.start())
            lines.append(
                {
                    "file": "src/" + path.name,
                    "block": block,
                    "id": match.group(1),
                    "text": match.group(2).strip(),
                }
            )
    return lines


def memory(pid):
    fields = {}
    for line in Path(f"/proc/{pid}/status").read_text().splitlines():
        name, _, value = line.partition(":")
        if name in ("VmRSS", "VmHWM", "Threads"):
            fields[name] = int(value.split()[0])
    return {
        "rss_kib": fields["VmRSS"],
        "peak_rss_kib": fields["VmHWM"],
        "threads": fields["Threads"],
    }


def timed(window, condition, label, timeout=float(os.environ.get("RECITE_LARGE_TIMEOUT", "120"))):
    start = time.monotonic()
    deadline = start + timeout
    last_error = None
    while time.monotonic() < deadline:
        if window.process.poll() is not None:
            raise RuntimeError(f"Writer exited while waiting for {label}")
        try:
            value = condition()
            if value:
                return value, round((time.monotonic() - start) * 1000, 1)
        except window.api.GLib.Error as error:
            last_error = error
        time.sleep(POLL_SECONDS)
    visible = []
    try:
        visible = [name for _, name in window.nodes() if name][:80]
        node = manuscript_now(window)
        visible.append("manuscript: " + (window.text(node) if node else "<none>"))
    except Exception as error:  # noqa: BLE001 - diagnostics only
        visible.append(repr(error))
    raise RuntimeError(
        f"Timed out after {timeout}s: {label}; last bus error: {last_error}; visible: {visible}"
    )


def summary(values):
    ordered = sorted(values)
    return {
        "samples": ordered,
        "median": round(statistics.median(ordered), 1),
        "max": ordered[-1],
    }


def manuscript_now(window):
    """The current manuscript field, or None without waiting."""
    return next(
        (
            node
            for node, name in window.nodes()
            if name == "Manuscript" and window.call(node, "Accessible", "GetRole")[0] == 79
        ),
        None,
    )


def showing(window, text):
    def condition():
        node = manuscript_now(window)
        return node if node is not None and window.text(node) == text else None

    return condition


def run(api, bus, registry, binary, output):
    profile = os.environ.get("RECITE_LARGE_PROFILE", "large")
    results = {"scenario": "large-project", "profile": profile, "binary": str(binary)}
    process = window = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-large-", dir="/tmp") as temporary:
            directory = Path(temporary)
            project = generate(profile, directory)
            lines = inventory(project)
            files = sorted({line["file"] for line in lines})
            results["project"] = {
                "files": len(files),
                "lines": len(lines),
                "source_bytes": sum(path.stat().st_size for path in project.rglob("*.recite")),
                "catalogue_bytes": sum(path.stat().st_size for path in project.rglob("*.po")),
            }
            config = directory / "config.toml"
            config.write_text(
                "config_version=1\n[writer]\nconfirm_exit=false\n"
                "[writer.presentation]\nui_scale=100\nsplit=false\n"
            )
            first = next(line for line in lines if line["file"] == files[0])
            environment = dict(os.environ, RECITE_CONFIG=str(config), WAYLAND_DEBUG="client")
            environment.pop("RUST_LOG", None)
            log_path = output.with_suffix(".writer.log")
            with log_path.open("w") as log:
                launched = wayland_ms()
                started = time.monotonic()
                process = subprocess.Popen(
                    [str(binary), "--project", str(project), "--document", first["file"]],
                    env=environment,
                    stdout=log,
                    stderr=subprocess.STDOUT,
                )
                window = attach(api, bus, registry, process)
                window.process = process
                _, _ = timed(window, showing(window, first["text"]), "first manuscript line")
                results["cold_open_to_first_line_ms"] = round(
                    (time.monotonic() - started) * 1000, 1
                )
                _, diagnostics = timed(
                    window,
                    lambda: window.find_text("no problems", contains=True),
                    "project diagnostics",
                )
                results["cold_open_to_diagnostics_ms"] = round(
                    results["cold_open_to_first_line_ms"] + diagnostics, 1
                )
                results["memory_after_open"] = memory(process.pid)

                switches = []
                for index in range(1, SWITCHES + 1):
                    target = next(
                        line
                        for line in lines
                        if line["file"] == files[index * (len(files) - 1) // SWITCHES]
                    )
                    key("p", control=True)
                    fill(
                        window,
                        "Search query",
                        f"{Path(target['file']).stem} {target['block'].removeprefix('block_')}",
                    )
                    key("Return")
                    header = f"{target['file']}  ::  {target['block']}"
                    _, elapsed = timed(
                        window,
                        lambda header=header: window.find_text(header, prefix=True),
                        "scene switch",
                    )
                    switches.append(elapsed)
                results["scene_switch_ms"] = summary(switches)

                searches = []
                for index in range(SEARCHES):
                    target = lines[len(lines) * (index + 1) // (SEARCHES + 1)]
                    window.workspace_action("Find project text")
                    fill(window, "Search query", target["text"])
                    key("Return")
                    _, elapsed = timed(window, showing(window, target["text"]), "search reveal")
                    searches.append(elapsed)
                results["search_to_passage_ms"] = summary(searches)
                results["memory_after_navigation"] = memory(process.pid)

                node = manuscript(window)
                window.focus(node)
                key("End", control=True)
                time.sleep(1.0)
                start = wayland_ms()
                subprocess.run(
                    ["wtype", "-d", str(KEY_DELAY_MS), "--", TYPED * 6], check=True, timeout=120
                )
                end = wayland_ms()
                _, settle = timed(
                    window,
                    lambda: window.text(manuscript(window)).endswith(TYPED * 6),
                    "typed text in the accessibility tree",
                )
                results["typing_settle_ms"] = settle
                results["memory_after_typing"] = memory(process.pid)
                for _ in range(len(TYPED) * 6):
                    key("z", control=True)
                timed(
                    window,
                    lambda: window.text(manuscript(window)) == target["text"],
                    "undo to the original text",
                )
                results["memory_after_undo"] = memory(process.pid)
                results["total_seconds"] = round(time.monotonic() - started, 1)
                process.terminate()
                process.wait(timeout=10)
                process = None
            text = log_path.read_text(errors="replace")
            events = protocol_events(text)
            base = events[0]["t"]
            commits = [
                event["t"]
                for event in events
                if event["request"]
                and event["interface"] == "wl_surface"
                and event["message"] == "commit"
            ]
            results["cold_open_to_first_commit_ms"] = (
                round(commits[0] - unwrap(launched, base), 1) if commits else None
            )
            results.update(
                {
                    f"typing_{name}": value
                    for name, value in timings(
                        events, unwrap(start, base), unwrap(end, base)
                    ).items()
                }
            )
            shutil.rmtree(project, ignore_errors=True)
        results["passed"] = True
    except Exception:
        results["passed"] = False
        results["error"] = traceback.format_exc()
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=10)
        output.write_text(json.dumps(results, indent=2) + "\n")
    return results["passed"]
