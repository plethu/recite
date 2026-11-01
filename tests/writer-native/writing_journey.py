"""Writing, scoped search, native history and saving in an owned branching project."""

import json
import os
import shutil
import subprocess
import tempfile
import traceback
from pathlib import Path

from native_window import attach, key, type_text
from picker_interactions import focused
from project_interactions import manuscript
from startup_interactions import field, fill
from writer_interactions import capture

ROOT = Path(__file__).resolve().parents[2]


def selected(window, word):
    return any(name.startswith("Selected · ") and word in name for _, name in window.nodes())


def dialog_focus(window):
    for node, name in window.nodes():
        if name not in ("Find project text", "Find in this beat", "Go to scene or beat"):
            continue
        if window.call(node, "Accessible", "GetRole")[0] == int(window.api.Atspi.Role.DIALOG):
            return next(
                (
                    child
                    for child, _ in window.api.descendants(window.bus, node)
                    if focused(window, child)
                ),
                None,
            )
    return None


def exercise(window, document, original, output, scale, results):
    window.wait(lambda: window.find_text("If you're here", prefix=True), "branching manuscript")
    if not window.find("Manuscript"):
        window.edit_passage(1)
    node = manuscript(window)
    window.focus(node)
    before = window.text(node)
    coordinates = window.api.GLib.Variant("(u)", (int(window.api.Atspi.CoordType.WINDOW),))
    save_bounds = window.call(window.find_action("Save"), "Component", "GetExtents", coordinates)[0]
    key("End", control=True)
    type_text("x")
    window.wait(lambda: window.text(node) == before + "x", "visible prose edit")
    assert (
        window.call(window.find_action("Save"), "Component", "GetExtents", coordinates)[0]
        == save_bounds
    )
    key("p", control=True)
    fill(window, "Search query", "bucket.x")
    query = field(window, "Search query")
    window.act("This beat")
    window.wait(lambda: focused(window, query), "scope switch returns query focus")
    assert window.text(query) == "bucket.x"
    window.wait(lambda: selected(window, "bucket.x"), "current edit appears in beat search")
    capture(output, f"-{scale}-current-text")
    type_text("y")
    window.wait(lambda: window.text(query) == "bucket.xy", "query after scope change")
    key("z", control=True)
    window.wait(lambda: window.text(query) == "bucket.x", "native query Undo across scope change")
    window.act("Saved project")
    window.wait(lambda: focused(window, query), "project scope query focus")
    window.wait(
        lambda: window.find_text("No matching results."), "saved scope excludes pending text"
    )
    assert window.text(query) == "bucket.x"
    capture(output, f"-{scale}-saved-scope")
    save = window.find_action("Save")
    assert save and not window.enabled("Save")
    window.call(save, "Action", "DoAction", window.api.GLib.Variant("(i)", (0,)))
    assert document.read_text() == original and window.find("Search query")
    for _ in range(8):
        prior = dialog_focus(window)
        assert prior is not None
        key("Tab")
        window.wait(
            lambda: (current := dialog_focus(window)) is not None and current != prior,
            "Tab stays inside search",
        )
    assert document.read_text() == original
    key("Escape")
    window.wait(lambda: focused(window, node), "cancel returns to original prose field")
    assert manuscript(window) == node and window.text(node) == before + "x"
    key("z", control=True)
    window.wait(lambda: window.text(node) == before, "native prose Undo survives search and scopes")
    key("y", control=True)
    window.wait(lambda: window.text(node) == before + "x", "native prose Redo survives search")
    key("s", control=True)
    window.wait(
        lambda: before + "x" in document.read_text(), "Save includes visible prose without Apply"
    )
    window.wait(lambda: window.find_text("Saved to disk", prefix=True), "saved feedback")
    capture(output, f"-{scale}-saved-writing")
    key("p", control=True)
    fill(window, "Search query", "bucket.x")
    window.act("Saved project")
    window.wait(lambda: selected(window, "bucket.x"), "saved search includes published text")
    key("Escape")
    window.wait(lambda: focused(window, node), "second return to prose")
    rounds = int(os.environ.get("RECITE_WRITING_ROUNDS", "6"))
    expected = before + "x"
    for index in range(rounds):
        key("End", control=True)
        type_text("y")
        expected += "y"
        window.wait(lambda: window.text(node) == expected, "continued writing")
        key("p", control=True)
        fill(window, "Search query", "missing courier")
        key("Escape")
        window.wait(lambda: focused(window, node), "repeated return to prose")
        key("s", control=True)
        window.wait(lambda: expected in document.read_text(), "repeated complete save")
        window.wait(lambda: window.find_text("Saved to disk", prefix=True), "idle saved receipt")
        assert manuscript(window) == node
    results[f"scope_focus_history_save_and_{rounds}_cycles_at_{scale}"] = True


def run(api, bus, registry, binary, output):
    results = {"scenario": "writing-journey", "backend": "owned Wayland"}
    process = None
    try:
        for scale in [100, 200]:
            with tempfile.TemporaryDirectory(
                prefix="recite-writing-journey-", dir="/tmp"
            ) as temporary:
                project = Path(temporary)
                (project / "src").mkdir()
                (project / "schema").mkdir()
                (project / "recite.project.toml").write_text(
                    'format_version=1\n[project]\nschema="schema/writer_examples.json"\n[discovery]\nsource_roots=["src"]\n'
                )
                shutil.copyfile(
                    ROOT / "fixtures/schema/valid/writer_examples.json",
                    project / "schema/writer_examples.json",
                )
                original = (
                    ROOT / "fixtures/recite/valid/writer_examples/relay_hub.recite"
                ).read_text()
                document = project / "src/relay_hub.recite"
                document.write_text(original)
                config = project / "ui.toml"
                config.write_text(
                    f"config_version=1\n[writer]\nconfirm_exit=false\n[writer.presentation]\nui_scale={scale}\n"
                )
                with output.with_name(output.stem + f"-{scale}.writer.log").open("w") as log:
                    process = subprocess.Popen(
                        [
                            str(binary),
                            "--project",
                            str(project),
                            "--document",
                            "src/relay_hub.recite",
                            "--route",
                            "recite://writer/write?scene=src/relay_hub.recite&view=script",
                        ],
                        env=dict(os.environ, RECITE_CONFIG=str(config), RUST_LOG="warn"),
                        stdout=log,
                        stderr=subprocess.STDOUT,
                    )
                    window = attach(api, bus, registry, process)
                    exercise(window, document, original, output, scale, results)
                    window.close_clean()
                    assert process.wait(timeout=5) == 0
                    process = None
        results["passed"] = True
    except Exception:
        results["passed"] = False
        results["error"] = traceback.format_exc()
        if process is not None and process.poll() is None:
            capture(output, "-error")
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
        output.write_text(json.dumps(results, indent=2) + "\n")
    return results["passed"]
