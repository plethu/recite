"""Overlapping manuscript windows retain native editors and guard outgoing drafts."""

import json
import os
import tempfile
import traceback
from pathlib import Path

from native_window import key, type_text
from passage_lookup import editor_visible
from picker_interactions import focused
from project_interactions import manuscript
from writer_interactions import capture, launch


def bounds(window, node):
    coordinates = window.api.GLib.Variant("(u)", (int(window.api.Atspi.CoordType.WINDOW),))
    return window.call(node, "Component", "GetExtents", coordinates)[0]


def step(window, direction, caption):
    assert window.act(direction + " beat entries")
    window.wait(lambda: window.find_text(caption), "overlapping reading range: " + caption)
    fields = [
        node
        for node, name in window.nodes()
        if name == "Manuscript" or name.startswith("Edit passage ")
    ]
    assert sum(window.call(node, "Accessible", "GetRole")[0] == 79 for node in fields) <= 16


def anchored_step(window, node, direction, caption):
    before = bounds(window, node)
    step(window, direction, caption)
    assert manuscript(window) == node
    window.wait(
        lambda: abs(bounds(window, node)[1] - before[1]) <= 1, "retained visible passage anchor"
    )
    assert editor_visible(window, node)


def leave_anchor_room(window, node):
    """Keep a fully visible overlap near the scroll end, above the action tail."""
    window.focus(window.section_scrollbar("Script pane"))
    key("End")

    def reveal_overlap():
        try:
            if editor_visible(window, node):
                return True
        except window.api.GLib.Error:
            # A clipped native field is absent from AT-SPI until it re-enters the viewport.
            pass
        window.focus(window.section_scrollbar("Script pane"))
        key("Up")
        return False

    window.wait(reveal_overlap, "visible overlap above the reading action tail")
    window.focus(node)
    assert editor_visible(window, node)


def exercise(window, source, output, scale, results):
    window.reveal_down("Edit passage 13")
    window.edit_passage(13)
    node = manuscript(window)
    window.wait(lambda: editor_visible(window, node), "visible passage before boundary")
    key("End", control=True)
    type_text("x")
    window.wait(lambda: window.text(node) == "Courier 13 waits in the café.x", "pending passage 13")
    original = source.read_text()
    leave_anchor_room(window, node)
    capture(output, f"-{scale}-before")
    anchored_step(window, node, "Next", "Entries 9–24 of 41")
    capture(output, f"-{scale}-after")
    anchored_step(window, node, "Previous", "Entries 1–16 of 41")
    anchored_step(window, node, "Next", "Entries 9–24 of 41")
    assert source.read_text() == original
    window.focus(node)
    key("z", control=True)
    window.wait(
        lambda: window.text(node) == "Courier 13 waits in the café.", "Undo across both boundaries"
    )
    key("y", control=True)
    window.wait(lambda: window.text(node).endswith("café.x"), "Redo across boundaries")
    window.reveal_down("Edit passage 21")
    window.edit_passage(21)
    node = manuscript(window)
    window.wait(
        lambda: window.text(node) == "Courier 21 waits in the café.", "next admitted passage"
    )
    key("End", control=True)
    type_text("z")
    window.wait(lambda: window.text(node).endswith("café.z"), "pending passage 21")
    leave_anchor_room(window, node)
    anchored_step(window, node, "Next", "Entries 17–32 of 41")
    window.focus(node)
    key("a", control=True)
    type_text("-> syntax")
    window.wait(lambda: window.text(node) == "-> syntax", "invalid outgoing draft")
    window.act("Next beat entries")
    window.wait(lambda: window.find_text("Can't continue yet"), "local rejected boundary feedback")
    window.wait(lambda: window.find_text("Continue paused"), "refused continuation status")
    window.wait(lambda: focused(window, node), "rejection returns to outgoing passage")
    assert window.find_text("Entries 17–32 of 41") and manuscript(window) == node
    assert window.text(node) == "-> syntax" and "-> syntax" not in source.read_text()
    capture(output, f"-{scale}-rejected")
    window.act("Show reason")
    window.wait(lambda: window.find_text("your editor", contains=True), "expanded refusal reason")
    window.act("Hide reason")
    window.act("Edit text")
    window.wait(lambda: focused(window, node), "local recovery restores native editing")
    key("a", control=True)
    type_text("Courier 21 repaired.")
    window.wait(lambda: window.text(node) == "Courier 21 repaired.", "repair outgoing prose")
    step(window, "Next", "Entries 25–40 of 41")
    node = window.wait(
        lambda: (
            current
            if window.text(current := manuscript(window)) == "Courier 25 waits in the café."
            else None
        ),
        "checked incoming passage",
    )
    window.wait(lambda: focused(window, node), "incoming native focus")
    step(window, "Previous", "Entries 17–32 of 41")
    assert manuscript(window) == node
    window.act("Save")
    window.wait(
        lambda: "Courier 21 repaired." in source.read_text(), "checked repaired source saved"
    )
    assert "Courier 13 waits in the café.x" in source.read_text()
    results[f"overlap_anchor_history_admission_and_save_at_{scale}"] = True


def run(api, bus, registry, binary, output):
    results = {"scenario": "reading-continuity", "backend": "owned Wayland"}
    process = window = None
    try:
        for scale in (100, 200):
            with tempfile.TemporaryDirectory(
                prefix="recite-reading-continuity-", dir="/tmp"
            ) as temporary:
                project = Path(temporary)
                (project / "recite.project.toml").write_text("format_version=1\n")
                source = project / "scene.recite"
                source.write_text(
                    ":: start default\n"
                    + "".join(
                        f"> @line_{n}\n  Courier {n} waits in the café.\n" for n in range(1, 41)
                    )
                    + "-> END\n"
                )
                config = Path(os.environ["XDG_CONFIG_HOME"]) / "recite/config.toml"
                config.parent.mkdir(parents=True, exist_ok=True)
                config.write_text(
                    f"config_version=1\n[writer]\nconfirm_exit=false\n[writer.presentation]\nui_scale={scale}\n"
                )
                with output.with_name(output.stem + f"-{scale}.writer.log").open("w") as log:
                    process, window, _ = launch(
                        api,
                        bus,
                        registry,
                        [str(binary), "--project", str(project), "--document", "scene.recite"],
                        log,
                    )
                    exercise(window, source, output, scale, results)
                    window.close_clean()
                    assert process.wait(timeout=5) == 0
                    process = window = None
        results["passed"] = True
    except Exception:
        results["passed"] = False
        results["error"] = traceback.format_exc()
        if window:
            capture(output, "-error")
    finally:
        if process and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
        output.write_text(json.dumps(results, indent=2) + "\n")
    return results["passed"]
