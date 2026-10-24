"""Map focus, personal placement and manuscript continuity on owned Wayland."""

import json
import os
import tempfile
import traceback
from pathlib import Path

from native_window import key, type_text
from picker_interactions import focused
from project_interactions import manuscript
from startup_interactions import field
from writer_interactions import capture, launch

SOURCE = """:: start default
> line@11111111111111111111
  At the gate.
? reply@22222222222222222222
  Continue.
  -> later
:: later
> line@33333333333333333333
  At the bridge.
-> END
"""


def card(window, title):
    return next(
        (
            node
            for node, name in window.nodes()
            if name.startswith(title)
            and window.call(node, "Accessible", "GetRole")[0]
            == int(window.api.Atspi.Role.PUSH_BUTTON)
            and window.call(
                window.api.property_value(window.bus, node, window.api.ACCESSIBLE, "Parent"),
                "Accessible",
                "GetRole",
            )[0]
            == int(window.api.Atspi.Role.LIST_ITEM)
        ),
        None,
    )


def focused_card(window, title):
    node = card(window, title)
    return node if node and focused(window, node) else None


def beat_item(window, title):
    return next(
        (
            node
            for node, name in window.nodes()
            if name.startswith(title)
            and window.call(node, "Accessible", "GetRole")[0]
            == int(window.api.Atspi.Role.LIST_ITEM)
        ),
        None,
    )


def selected_item(window, title):
    node = beat_item(window, title)
    bit = 1 << int(window.api.Atspi.StateType.SELECTED)
    return node if node and window.call(node, "Accessible", "GetState")[0][0] & bit else None


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "map", "physical_ime": False}
    process, window = None, None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-map-") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text("format_version = 1\n")
            source = project / "scene.recite"
            source.write_text(SOURCE)
            with output.with_suffix(".writer.log").open("w") as log:
                process, window, node = launch(
                    api, bus, registry, [str(binary), "--project", str(project)], log
                )
                window.focus(node)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(node) == "At the gate.x", "initial typing")
                window.act("Map")
                selected = window.wait(lambda: card(window, "Start"), "accepted Map card")
                window.wait(lambda: selected_item(window, "Start"), "selected native beat item")
                capture(output, "-map")
                item = beat_item(window, "Later")
                window.call(item, "Action", "DoAction", api.GLib.Variant("(i)", (0,)))
                window.wait(
                    lambda: selected_item(window, "Later"),
                    "accessible item selects without opening Script",
                )
                assert window.find("Script pane") is None
                item = beat_item(window, "Start")
                window.call(item, "Action", "DoAction", api.GLib.Variant("(i)", (0,)))
                window.wait(
                    lambda: selected_item(window, "Start"), "accessible item restores selection"
                )
                selector = window.find_action("Script")
                window.focus(selector)
                window.act("Script")
                current = window.wait(lambda: manuscript(window), "return to manuscript")
                assert current == node
                window.wait(
                    lambda: focused(window, selector), "Script selection preserves selector focus"
                )
                window.focus(current)
                key("z", control=True)
                window.wait(
                    lambda: window.text(current) == "At the gate.", "native undo across Map"
                )
                results["map_and_script_preserve_native_field_and_undo"] = True
                read = window.document_control("Read beat")
                window.focus(read)
                key("Return")
                window.wait(
                    lambda: window.find("Manuscript") is None,
                    "Read beat survives focus and native Enter release",
                )
                results["read_beat_remains_actionable_after_clean_field_blur"] = True
                window.act("Map")
                selected = window.wait(lambda: card(window, "Start"), "return to Map")
                window.focus(selected)
                key("Down")
                selected = window.wait(lambda: card(window, "Later"), "spatial beat selection")
                window.wait(lambda: focused_card(window, "Later"), "focus follows selected beat")
                window.wait(
                    lambda: selected_item(window, "Later"), "native selected state follows beat"
                )
                assert selected_item(window, "Start") is None
                results["native_beat_item_selected_state_tracks_keyboard_selection"] = True
                capture(output, "-selected")
                grip = window.wait(
                    lambda: window.find("Move Later", prefix=True), "selected move grip"
                )
                window.focus(grip)
                key("Right")
                layout = Path(os.environ["XDG_CONFIG_HOME"]) / "recite/writer-layouts.json"
                window.wait(lambda: layout.is_file(), "durable personal placement")
                saved = json.loads(layout.read_text())
                scenes = saved["scenes"]
                assert len(scenes) == 1
                positions = next(iter(scenes.values()))
                assert len(positions) == 1
                assert next(iter(positions)).startswith("passage:")
                assert source.read_text() == SOURCE
                results["keyboard_move_persists_outside_dialogue_source"] = True
                window.focus(window.wait(lambda: card(window, "Later"), "moved beat"))
                for _ in range(4):
                    window.act("Zoom out")
                viewport = window.wait(lambda: window.find("Scene map"), "overview viewport")
                window.wait(
                    lambda: focused(window, viewport), "overview receives removed-card focus"
                )
                capture(output, "-overview")
                key("Up")
                selected = window.wait(lambda: card(window, "Start"), "overview keyboard promotion")
                window.wait(lambda: focused_card(window, "Start"), "promoted native card focus")
                key("Down")
                selected = window.wait(lambda: card(window, "Later"), "return to selected beat")
                window.wait(lambda: focused_card(window, "Later"), "returned native focus")
                results["overview_native_focus_and_keyboard_promotion"] = True
                window.focus(window.wait(lambda: card(window, "Later"), "moved beat"))
                key("slash")
                query = field(window, "Search query")
                window.wait(lambda: focused(window, query), "Map slash opens Go to")
                key("Escape")
                selected = window.wait(lambda: card(window, "Later"), "Map focus after lookup")
                window.focus(selected)
                key("Return")
                # Split can place its manuscript below the map in a short window.
                window.reveal_start("Script")
                window.act("Script")
                window.wait(lambda: window.find_text("At the bridge."), "opened selected beat")
                capture(output, "-script")
                results["arrows_native_focus_enter_and_slash_lookup"] = True
                window.reveal_start("Save")
                window.act("Save")
                window.wait(lambda: window.find_text("Saved"), "save before closing")
                window.close_clean()
                assert process.wait(timeout=5) == 0
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
        if process is not None and process.poll() is None:
            capture(output, "-error")
            results["visible_names"] = [name for _, name in window.nodes()]
            results["focused_names"] = [
                name for node, name in window.nodes() if focused(window, node)
            ]
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
    output.write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results))
    return "error" not in results
