"""Whole-beat native translation fields, targeted saves and retained history."""

import json
import os
import subprocess
import tempfile
import traceback
from pathlib import Path

from localisation_interactions import reveal, reveal_field
from native_window import attach, key
from project_tools_interactions import command
from startup_interactions import field, fill
from writer_interactions import capture

FIRST = "Translation · passage 1 · form 1"
SECOND = "Translation · passage 2 · form 1"
SOURCE = """:: opening default
> @ffee6p8
  The last tram waits.
? @rthh7ab
  Shall we go?
  -> later
:: later
> @bqyvpqq
  A passage without a catalogue entry.
-> opening
"""
PO = """msgid ""
msgstr "Language: fr\\n"

msgctxt "ffee6p8&formal"
msgid "Different source wording."
msgstr "Une version."

msgctxt "ffee6p8"
msgid "The last tram waits."
msgstr "Le dernier tram attend."

msgctxt "rthh7ab"
msgid "Shall we go?"
msgstr "On y va ?"
"""


def reveal_destination_body(window):
    coordinates = window.api.GLib.Variant("(u)", (int(window.api.Atspi.CoordType.WINDOW),))

    def visible():
        body = window.find_text("A passage without a catalogue entry.")
        scrollbar = window._outer_scrollbar()
        if body is not None:
            bounds = window.call(body, "Component", "GetExtents", coordinates)[0]
            clip = window.call(scrollbar, "Component", "GetExtents", coordinates)[0]
            if bounds[1] >= clip[1] and bounds[1] + bounds[3] <= clip[1] + clip[3]:
                return True
        window.focus(scrollbar)
        key("Down")
        return False

    window.wait(visible, "complete destination preview body within viewport")


def exercise(window, po, output, results, scale):
    reveal_field(window, FIRST)
    first = field(window, FIRST)
    assert window.text(first) == "Le dernier tram attend."
    reveal_field(window, SECOND)
    second = field(window, SECOND)
    assert window.text(second) == "On y va ?"
    capture(output, f"-{scale}-beat")
    reveal(window, "→ Later")
    window.act("→ Later")
    window.wait(
        lambda: window.find_text("Destination preview · read only · Later"),
        "read-only destination context",
    )
    capture(output, f"-{scale}-destination")
    reveal_destination_body(window)
    capture(output, f"-{scale}-destination-body")
    reveal(window, "▾ Later")
    window.act("▾ Later")
    reveal_field(window, FIRST)
    fill(window, FIRST, "Premier brouillon indépendant.")
    fill(window, SECOND, "Deuxième brouillon publié.")
    key("s", control=True)
    window.wait(lambda: "Deuxième brouillon publié." in po.read_text(), "focused row saved")
    assert "Premier brouillon indépendant." not in po.read_text()
    assert "Une version." in po.read_text()
    reveal(window, "Open translation queue")
    window.wait(lambda: window.enabled("Open translation queue"), "save receipt collected")
    window.act("Open translation queue")
    reveal(window, "Translate current beat")
    capture(output, f"-{scale}-queue")
    window.act("Translate current beat")
    reveal_field(window, FIRST)
    assert tuple(field(window, FIRST)) == tuple(first)
    reveal_field(window, SECOND)
    assert tuple(field(window, SECOND)) == tuple(second)
    reveal_field(window, FIRST)
    window.focus(first)
    changed = "Premier brouillon indépendant."
    for _ in range(len(changed) + 2):
        if window.text(first) == "Le dernier tram attend.":
            break
        previous = window.text(first)
        key("z", control=True)
        window.wait(lambda: window.text(first) != previous, "next native Undo transaction")
    assert window.text(first) == "Le dernier tram attend."
    for _ in range(len(changed) + 2):
        if window.text(first) == changed:
            break
        previous = window.text(first)
        key("y", control=True)
        window.wait(lambda: window.text(first) != previous, "next native Redo transaction")
    assert window.text(first) == changed
    results[f"independent_native_history_targeted_save_and_worker_checkout_{scale}"] = True
    if scale == 100:
        reveal(window, "Later")
        window.act("Later")
    else:
        window.act("Scenes…")
        fill(window, "Search query", "later")
        key("Return")
    window.wait(
        lambda: window.find_text("A passage without a catalogue entry."), "next beat source"
    )
    window.wait(
        lambda: window.find_text("No matching entry", prefix=True), "explicit missing entry"
    )
    command(window, "Localise")
    window.wait(
        lambda: window.find_text("A passage without a catalogue entry."),
        "Localise remains on chosen beat",
    )
    capture(output, f"-{scale}-missing")
    if window.find("Scenes…"):
        window.act("Scenes…")
    else:
        window.act("Find scene or beat…")
    fill(window, "Search query", "opening")
    key("Return")
    reveal_field(window, FIRST)
    assert window.text(field(window, FIRST)) == "Premier brouillon indépendant."
    results[f"checked_beat_navigation_preserves_drafts_and_reports_missing_entry_{scale}"] = True
    capture(output, f"-{scale}-returned")


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "localisation-beat"}
    process = None
    try:
        for scale in (100, 200):
            with tempfile.TemporaryDirectory(prefix="recite-bilingual-", dir="/tmp") as temporary:
                root = Path(temporary)
                (root / "recite.project.toml").write_text("format_version=1\n")
                source = root / "scene.recite"
                source.write_text(SOURCE)
                po = root / "fr.po"
                po.write_text(PO)
                config = root / "ui.toml"
                config.write_text(f"config_version=1\n[writer.presentation]\nui_scale={scale}\n")
                with (
                    output.with_name(output.stem + f"-{scale}")
                    .with_suffix(".writer.log")
                    .open("w") as log
                ):
                    process = subprocess.Popen(
                        [
                            str(binary),
                            "--project",
                            str(root),
                            "--route",
                            "recite://writer/localise?scene=scene.recite&beat=opening&catalogue=fr.po",
                        ],
                        env=dict(os.environ, RECITE_CONFIG=str(config)),
                        stdout=log,
                        stderr=subprocess.STDOUT,
                    )
                    window = attach(api, bus, registry, process)
                    exercise(window, po, output, results, scale)
                    assert source.read_text() == SOURCE
                    assert "Premier brouillon indépendant." not in po.read_text()
                    process.terminate()
                    process.wait(timeout=10)
                    process = None
    except Exception:
        results["error"] = traceback.format_exc()
        if process and process.poll() is None:
            capture(output, "-error")
    finally:
        if process and process.poll() is None:
            process.terminate()
            process.wait(timeout=10)
    output.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(results, ensure_ascii=False))
    return "error" not in results
