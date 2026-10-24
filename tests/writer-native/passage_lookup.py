"""Off-page passage lookup and accepted rail reveal on a larger owned project."""

import json
import os
import tempfile
import traceback
from pathlib import Path

from native_window import key, type_text
from picker_interactions import focused
from project_interactions import document_caption, manuscript
from startup_interactions import field, fill
from writer_interactions import capture, capture_settled, launch


def editor_visible(window, node):
    pane = window.find("Script pane")
    if pane is None:
        return False
    rects = [
        window.call(item, "Component", "GetExtents", window.api.GLib.Variant("(u)", (0,)))[0]
        for item in (pane, node)
    ]
    (x, y, width, height), (left, top, field_width, field_height) = rects
    return (
        left >= x - 1
        and left + field_width <= x + width + 1
        and top >= y - 1
        and top + min(field_height, height) <= y + height + 1
    )


def run(api, bus, registry, binary, output):
    scale = int(os.environ.get("RECITE_PASSAGE_SCALE", "100"))
    results = {"backend": "owned Wayland", "scenario": "passage-lookup", "ui_scale": scale}
    process = window = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-passage-lookup-") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text("format_version=1\n")
            source = project / "scene.recite"
            source.write_text(
                ":: start default\n"
                + "".join(
                    f"> line_{n}@{n:020x}\n  Courier {n} waits in the café.\n" for n in range(1, 41)
                )
                + "-> END\n"
            )
            for n in range(9):
                (project / f"other_{n}.recite").write_text(
                    "".join(
                        f":: beat_{beat}{' default' if beat == 0 else ''}\n-> {f'beat_{beat + 1}' if beat < 19 else 'END'}\n"
                        for beat in range(20)
                    )
                )
            config = Path(os.environ["XDG_CONFIG_HOME"]) / "recite/config.toml"
            config.parent.mkdir(parents=True, exist_ok=True)
            config.write_text(
                f"config_version=1\n[writer]\nconfirm_exit=false\n[writer.presentation]\nui_scale={scale}\n"
            )
            with output.with_suffix(".writer.log").open("w") as log:
                process, window, node = launch(
                    api,
                    bus,
                    registry,
                    [str(binary), "--project", str(project), "--document", "scene.recite"],
                    log,
                )
                window.focus(node)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(node).endswith("café.x"), "outgoing native draft")
                window.act("Find in this beat")
                query = field(window, "Search query")
                window.wait(lambda: focused(window, query), "passage query autofocus")
                type_text("Courier 39")
                window.wait(
                    lambda: window.find_text("39 · line_39", contains=True),
                    "off-page passage result",
                )
                # Focused native text has an animated caret; assert its state, then
                # capture it without requiring a byte-identical sequence of frames.
                capture(output, "-passage-search")
                key("Return")
                node = manuscript(window)
                window.wait(
                    lambda: (
                        window.text(node) == "Courier 39 waits in the café."
                        and focused(window, node)
                    ),
                    "exact late passage and native focus",
                )
                window.wait(lambda: editor_visible(window, node), "complete late editor visibility")
                window.wait(lambda: window.find_text("Entries 33–41 of 41"), "late reading page")
                capture(output, "-late-passage")
                assert editor_visible(window, node) and focused(window, node)
                key("End", control=True)
                type_text("y")
                window.wait(lambda: window.text(node).endswith("café.y"), "late native typing")
                key("z", control=True)
                window.wait(lambda: window.text(node).endswith("café."), "late native undo")
                window.act("Save")
                window.wait(
                    lambda: "Courier 1 waits in the café.x" in source.read_text(),
                    "outgoing draft retained through checked navigation",
                )
                results["exact_passage_focus_native_undo_and_outgoing_save"] = True
                window.act("Previous beat entries")
                window.wait(lambda: window.find_text("Entries 17–32 of 41"), "preceding page")
                capture_settled(window, output, "-previous-page")
                window.workspace_action("Find project text")
                fill(window, "Search query", "courier 39")
                key("Return")
                node = manuscript(window)
                window.wait(
                    lambda: window.text(node) == "Courier 39 waits in the café.",
                    "saved search opens its exact passage",
                )
                results["saved_search_targets_the_matching_passage"] = True
                key("p", control=True)
                fill(window, "Search query", "other_8 18")
                key("Return")
                if scale == 100:
                    window.wait(
                        lambda: document_caption(window, "other_8.recite", selected=True),
                        "off-page file revealed",
                    )
                    window.wait(lambda: window.find("Beat 18"), "off-page beat revealed")
                    results["accepted_destination_reveals_file_and_beat_pages"] = True
                else:
                    window.wait(
                        lambda: window.find_text("other_8.recite", contains=True),
                        "selected document in compact shell",
                    )
                capture_settled(window, output, "-rail-destination")
                window.close_clean()
                assert process.wait(timeout=5) == 0
                results["passed"] = True
    except Exception:
        results["passed"] = False
        results["error"] = traceback.format_exc()
        if window:
            results["exposure"] = window.exposure_diagnostic()
    finally:
        if process and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
        output.write_text(json.dumps(results, indent=2) + "\n")
    return results["passed"]
