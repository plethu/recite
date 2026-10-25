"""Native command search, focus restoration, saved-text lookup and checked history."""

import json
import os
import tempfile
import traceback
from pathlib import Path

from native_window import key, type_text
from project_interactions import document_caption, manuscript
from startup_interactions import field, fill
from writer_interactions import SOURCE, capture, launch


def focused(window, node):
    bit = 1 << int(window.api.Atspi.StateType.FOCUSED)
    return bool(window.call(node, "Accessible", "GetState")[0][0] & bit)


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "picker"}
    process, window = None, None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-picker-") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text("format_version = 1\n")
            source = project / "scene.recite"
            source.write_text(SOURCE)
            config = Path(os.environ["XDG_CONFIG_HOME"]) / "recite/config.toml"
            config.parent.mkdir(parents=True, exist_ok=True)
            config.write_text('config_version = 1\n[ui]\nkeymap = "vim"\n')
            (project / "zz-other.recite").write_text(
                ":: other\n> third@33333333333333333333\n  Another scene waits.\n-> END\n"
            )
            with output.with_suffix(".writer.log").open("w") as log:
                process, window, node = launch(
                    api, bus, registry, [str(binary), "--project", str(project)], log
                )
                window.focus(node)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(node).endswith("waits.x"), "initial native draft")
                key("p", control=True, shift=True)
                query = field(window, "Search query")
                window.wait(lambda: focused(window, query), "automatic query focus")
                capture(output, "-commands")
                type_text("Save")
                window.wait(lambda: window.text(query) == "Save", "query typing")
                capture(output, "-filtered")
                key("Escape")
                node = manuscript(window)
                window.wait(lambda: focused(window, node), "return to original manuscript")
                key("z", control=True)
                window.wait(
                    lambda: window.text(node) == "The last tram waits.", "native undo across search"
                )
                key("y", control=True)
                window.wait(
                    lambda: window.text(node).endswith("waits.x"), "native redo across search"
                )
                results["query_focus_escape_restoration_and_native_history"] = True
                key("p", control=True, shift=True)
                query = field(window, "Search query")
                window.wait(lambda: focused(window, query), "reopened query focus")
                type_text("Save")
                key("Return")
                window.wait(
                    lambda: "The last tram waits.x" in source.read_text(), "command search save"
                )
                results["command_search_runs_checked_save"] = True
                key("p", control=True)
                query = field(window, "Search query")
                window.wait(lambda: focused(window, query), "go-to query focus")
                type_text("other")
                key("Down")
                window.wait(
                    lambda: window.find_text("Selected · zz-other.recite · Other"),
                    "keyboard result selection",
                )
                capture(output, "-go-to")
                key("Return")
                window.wait(
                    lambda: document_caption(window, "zz-other.recite", selected=True),
                    "accepted go-to document",
                )
                window.act("Back")
                window.wait(
                    lambda: document_caption(window, "scene.recite", selected=True),
                    "history back document",
                )
                node = manuscript(window)
                assert window.text(node) == "The last tram waits.x"
                window.act("Forward")
                window.wait(
                    lambda: document_caption(window, "zz-other.recite", selected=True),
                    "history forward document",
                )
                results["go_to_keyboard_selection_and_back_forward"] = True
                window.workspace_action("Find project text")
                fill(window, "Search query", "tram")
                window.wait(
                    lambda: window.find_text("The last tram waits.x", contains=True),
                    "saved text result",
                )
                capture(output, "-project-text")
                key("Return")
                window.wait(
                    lambda: document_caption(window, "scene.recite", selected=True),
                    "saved text destination",
                )
                window.wait(
                    lambda: window.text(manuscript(window)) == "The last tram waits.x",
                    "saved text opens its matching manuscript passage",
                )
                results["saved_project_text_opens_matching_passage"] = True
                capture(output, "-writer")
                key("p", control=True)
                query = field(window, "Search query")
                window.wait(lambda: focused(window, query), "repeat-search query focus")
                type_text("recite")
                key("Down")
                window.wait(
                    lambda: window.find_text("Selected · scene.recite · Opening"),
                    "accepted repeated scene target",
                )
                key("Return")
                window.wait(lambda: not window.find("Search query"), "lookup accepted")
                window.focus(window.find_action("Find scene or beat…"))
                type_text("n")
                window.wait(
                    lambda: document_caption(window, "zz-other.recite", selected=True),
                    "next lookup match",
                )
                window.focus(window.find_action("Find scene or beat…"))
                type_text("N")
                window.wait(
                    lambda: document_caption(window, "scene.recite", selected=True),
                    "previous lookup match",
                )
                key("p", control=True)
                query = field(window, "Search query")
                window.wait(lambda: focused(window, query), "cancelled lookup query focus")
                type_text("other")
                key("Escape")
                window.wait(lambda: not window.find("Search query"), "cancelled search")
                window.focus(window.find_action("Find scene or beat…"))
                type_text("n")
                window.wait(
                    lambda: document_caption(window, "zz-other.recite", selected=True),
                    "cancelled search preserves accepted repeat query",
                )
                results["vim_repeat_uses_accepted_lookup_and_escape_preserves_it"] = True
                capture(output, "-repeated")
                window.close_clean()
                assert process.wait(timeout=5) == 0
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
        if process is not None and process.poll() is None:
            capture(output, "-error")
            results["visible_names"] = [name for _, name in window.nodes()]
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
    output.write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results))
    return "error" not in results
