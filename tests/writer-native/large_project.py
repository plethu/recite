"""Native Writer timings and memory on a large generated project (#109).

The project comes from `recite-fixturegen` (`fixtures/synthetic/profiles.toml`). Each timer starts
when the probe sends its input and stops when the accessibility tree first shows the result, polled
every 10 ms, so it includes Writer's work, its frame, the AT-SPI update and up to one poll. Frame
readiness after typing comes from the client's own Wayland log, as in `frame_timing.py`.

Writer opens the project several times; every open but the last closes once diagnostics appear,
and the last continues through navigation, search and repeated edit/undo cycles. Budgets live in
`large_budgets.py`. Set RECITE_LARGE_PERF to a `perf` binary to sample the last open under its own
pid, with phase boundaries in CLOCK_MONOTONIC seconds.
"""

import json
import os
import re
import shutil
import signal
import statistics
import subprocess
import tempfile
import time
import traceback
from pathlib import Path

from frame_timing import protocol_events, timings, unwrap, wayland_ms
from large_budgets import check
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
OPENS = int(os.environ.get("RECITE_LARGE_OPENS", "3"))
EDIT_CYCLES = 3
PERF_ARM_SECONDS = 3


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


def start_perf(perf, pid, output):
    """Sample Writer with `perf` when RECITE_LARGE_PERF names the perf binary."""
    return subprocess.Popen(
        [
            perf,
            "record",
            "-e",
            "cpu-clock",
            "-F",
            "999",
            "-k",
            "CLOCK_MONOTONIC",
            "--call-graph",
            "dwarf,16384",
            "-p",
            str(pid),
            "-o",
            str(output.with_suffix(".perf.data")),
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def launch(command, environment, log, output, profile):
    """Start Writer, optionally under perf, and time its first line and diagnostics."""
    perf = os.environ.get("RECITE_LARGE_PERF") if profile else None
    if perf:
        # Stop before exec so perf samples startup under Writer's own pid.
        command = ["sh", "-c", 'kill -STOP $$; exec "$@"', "sh", *command]
    process = subprocess.Popen(command, env=environment, stdout=log, stderr=subprocess.STDOUT)
    profiler = None
    if perf:
        while Path(f"/proc/{process.pid}/stat").read_text().split(") ")[1][0] != "T":
            time.sleep(POLL_SECONDS)
        profiler = start_perf(perf, process.pid, output)
        time.sleep(PERF_ARM_SECONDS)
        os.kill(process.pid, signal.SIGCONT)
    return process, profiler, wayland_ms(), time.monotonic()


def opened(api, bus, registry, process, started, first):
    window = attach(api, bus, registry, process)
    window.process = process
    timed(window, showing(window, first["text"]), "first manuscript line")
    first_line = round((time.monotonic() - started) * 1000, 1)
    timed(window, lambda: window.find_text("no problems", contains=True), "project diagnostics")
    return window, first_line, round((time.monotonic() - started) * 1000, 1)


def switch_scenes(window, lines, files):
    switches = []
    for index in range(1, SWITCHES + 1):
        target = next(
            line for line in lines if line["file"] == files[index * (len(files) - 1) // SWITCHES]
        )
        key("p", control=True)
        query = f"{Path(target['file']).stem} {target['block'].removeprefix('block_')}"
        fill(window, "Search query", query)
        key("Return")
        header = f"{target['file']}  ::  {target['block']}"
        _, elapsed = timed(
            window, lambda header=header: window.find_text(header, prefix=True), "scene switch"
        )
        switches.append(elapsed)
    return switches


def search_lines(window, lines):
    searches = []
    for index in range(SEARCHES):
        target = lines[len(lines) * (index + 1) // (SEARCHES + 1)]
        window.workspace_action("Find project text")
        fill(window, "Search query", target["text"])
        key("Return")
        _, elapsed = timed(window, showing(window, target["text"]), "search reveal")
        searches.append(elapsed)
    return searches, target


def edit_cycle(window, original, pid):
    """Type at the end of the passage, undo back to the saved text and sample memory."""
    window.focus(manuscript(window))
    key("End", control=True)
    time.sleep(1.0)
    start = wayland_ms()
    subprocess.run(["wtype", "-d", str(KEY_DELAY_MS), "--", TYPED * 6], check=True, timeout=120)
    end = wayland_ms()
    _, settle = timed(
        window,
        lambda: window.text(manuscript(window)).endswith(TYPED * 6),
        "typed text in the accessibility tree",
    )
    for _ in range(len(TYPED) * 6):
        key("z", control=True)
    timed(window, lambda: window.text(manuscript(window)) == original, "undo to the original text")
    return {"settle_ms": settle, **memory(pid)}, (start, end)


def run(api, bus, registry, binary, output):
    profile = os.environ.get("RECITE_LARGE_PROFILE", "large")
    results = {"scenario": "large-project", "profile": profile, "binary": str(binary)}
    process = None
    phases = {}
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
            command = [str(binary), "--project", str(project), "--document", first["file"]]
            first_lines, diagnostics = [], []
            for _ in range(OPENS - 1):
                process, _, _, started = launch(
                    command, environment, subprocess.DEVNULL, output, False
                )
                _, first_line, ready = opened(api, bus, registry, process, started, first)
                first_lines.append(first_line)
                diagnostics.append(ready)
                process.terminate()
                process.wait(timeout=10)
                process = None
            log_path = output.with_suffix(".writer.log")
            with log_path.open("w") as log:
                process, profiler, launched, started = launch(
                    command, environment, log, output, True
                )
                window, first_line, ready = opened(api, bus, registry, process, started, first)
                phases["open"] = (started, started + first_line / 1000)
                first_lines.append(first_line)
                diagnostics.append(ready)
                results["cold_open_to_first_line_ms"] = summary(first_lines)
                results["cold_open_to_diagnostics_ms"] = summary(diagnostics)
                results["memory_after_open"] = memory(process.pid)

                phase = time.monotonic()
                results["scene_switch_ms"] = summary(switch_scenes(window, lines, files))
                phases["switch"] = (phase, time.monotonic())
                phase = time.monotonic()
                searches, target = search_lines(window, lines)
                results["search_to_passage_ms"] = summary(searches)
                phases["search"] = (phase, time.monotonic())
                results["memory_after_navigation"] = memory(process.pid)

                cycles = []
                for index in range(EDIT_CYCLES):
                    cycle, window_ms = edit_cycle(window, target["text"], process.pid)
                    cycles.append(cycle)
                    if index == 0:
                        start, end = window_ms
                results["edit_cycles"] = cycles
                results["total_seconds"] = round(time.monotonic() - started, 1)
                process.terminate()
                process.wait(timeout=10)
                process = None
                if profiler is not None:
                    profiler.wait(timeout=120)
                    results["perf"] = {
                        "data": str(output.with_suffix(".perf.data")),
                        "phases": phases,
                    }
            events = protocol_events(log_path.read_text(errors="replace"))
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
        failures = check(profile, results)
        results["budget_failures"] = failures
        results["passed"] = not failures
    except Exception:
        results["passed"] = False
        results["error"] = traceback.format_exc()
    finally:
        if process is not None and process.poll() is None:
            process.kill()
            process.wait(timeout=10)
        output.write_text(json.dumps(results, indent=2) + "\n")
    return results["passed"]
