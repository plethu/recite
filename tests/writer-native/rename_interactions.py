"""Grouped rename reviews, native field history and explicit multi-file publication."""

import json
import subprocess
import tempfile
import tomllib
import traceback
from pathlib import Path

from native_window import attach, key, type_text
from project_interactions import manuscript
from startup_interactions import fill
from writer_interactions import SOURCE, capture


def command(window, query):
    key("p", control=True, shift=True)
    fill(window, "Search query", query)
    key("Return")


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "rename"}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-rename-") as temporary:
            root = Path(temporary)
            before = SOURCE.replace("-> END", "-> old") + "\n:: old\n-> END\n"
            reference = ":: elsewhere\n-> scene.recite::old\n"
            original = "format_version=1\n[[scenes]]\nid='ending'\nasset='scene.recitec'\nblock='old'\nparticipants=['mara']\n"
            source = root / "scene.recite"
            other = root / "zz-other.recite"
            manifest = root / "recite.project.toml"
            source.write_text(before)
            other.write_text(reference)
            manifest.write_text(original)
            with output.with_suffix(".writer.log").open("w") as log:
                process = subprocess.Popen(
                    [
                        str(binary),
                        "--project",
                        str(root),
                        "--route",
                        "recite://writer/write?scene=scene.recite&passage=11111111111111111111",
                    ],
                    stdout=log,
                    stderr=subprocess.STDOUT,
                )
                window = attach(api, bus, registry, process)
                try:
                    node = manuscript(window)
                except RuntimeError:
                    capture(output, "-startup-failure")
                    results["startup_nodes"] = [name for _, name in window.nodes()]
                    raise
                prose = window.text(node)
                window.focus(node)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(node) == prose + "x", "native draft")
                key("z", control=True)
                window.wait(lambda: window.text(node) == prose, "native undo")
                command(window, "Rename beat")
                fill(window, "Current beat name", "old")
                fill(window, "New beat name", "new")
                window.act("Review project rename")
                window.wait(
                    lambda: window.find_text("Rename reviewed.", prefix=True), "reviewed rename"
                )
                capture(output, "-review")
                visited = set()
                for _ in range(30):
                    caption = window.wait(
                        lambda: window.find_text("Review ·", prefix=True), "review caption"
                    )
                    visited.add(window.text(caption))
                    if not window.enabled("More rename changes"):
                        break
                    previous = window.text(caption)
                    window.act("More rename changes")
                    window.wait(
                        lambda: (
                            window.find_text("Review ·", prefix=True)
                            and window.text(window.find_text("Review ·", prefix=True)) != previous
                        ),
                        "next rename change",
                    )
                else:
                    raise AssertionError("rename review did not finish")
                assert any("zz-other.recite" in caption for caption in visited)
                assert any("recite.project.toml" in caption for caption in visited)
                assert source.read_text() == before and other.read_text() == reference
                assert manifest.read_text() == original
                results["review_covers_references_and_manifest_without_writing_sources"] = True
                window.reveal_start("Apply project rename")
                window.act("Apply project rename")
                window.wait(
                    lambda: window.find_text("Project rename staged.", prefix=True),
                    "grouped rename",
                )
                capture(output, "-staged")
                assert source.read_text() == before and other.read_text() == reference
                assert manifest.read_text() == original
                window.act("Return to writing")
                current = manuscript(window)
                assert tuple(current) == tuple(node)
                window.focus(current)
                key("y", control=True)
                window.wait(lambda: window.text(current) == prose + "x", "retained native redo")
                key("z", control=True)
                window.wait(lambda: window.text(current) == prose, "retained native undo")
                results["exclusive_worker_preserves_mounted_field_and_native_history"] = True
                journal = root / ".recite-manifest-draft.json"
                command(window, "Undo")
                window.wait(
                    lambda: (
                        tomllib.loads(json.loads(journal.read_text())["text"])["scenes"][0]["block"]
                        == "old"
                    ),
                    "grouped global undo",
                )
                command(window, "Redo")
                window.wait(
                    lambda: (
                        tomllib.loads(json.loads(journal.read_text())["text"])["scenes"][0]["block"]
                        == "new"
                    ),
                    "grouped global redo",
                )
                results["global_undo_redo_restores_grouped_project_change"] = True
                window.act("Save all")
                window.wait(
                    lambda: (
                        ":: new" in source.read_text()
                        and "scene.recite::new" in other.read_text()
                        and tomllib.loads(manifest.read_text())["scenes"][0]["block"] == "new"
                    ),
                    "all renamed files saved",
                )
                results["save_all_publishes_manuscript_references_and_manifest"] = True
                window.close_clean()
                assert process.wait(timeout=5) == 0
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
    output.write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results, indent=2))
    return "error" not in results
