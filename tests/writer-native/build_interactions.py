"""Small native build workflow; large-project stress belongs to project-tools."""

import json
import subprocess
import tempfile
import traceback
from pathlib import Path

from native_window import attach, key, type_text
from project_interactions import manuscript
from project_tools_interactions import command
from writer_interactions import SOURCE, capture, capture_settled


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "build"}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-native-build-") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text(
                "format_version=1\n[project]\ncontent_set='native-build'\n"
                "[[scenes]]\nid='opening'\nasset='build/first.recitec'\nblock='opening'\nparticipants=['mara']\n"
                "[[scenes]]\nid='elsewhere'\nasset='build/second.recitec'\nblock='elsewhere'\nparticipants=['mara']\n"
            )
            source = project / "scene.recite"
            source.write_text(SOURCE)
            (project / "zz-other.recite").write_text(":: elsewhere\n-> END\n")
            with output.with_suffix(".writer.log").open("w") as log:
                process = subprocess.Popen(
                    [
                        str(binary),
                        "--project",
                        str(project),
                        "--route",
                        "recite://writer/write?scene=scene.recite&passage=11111111111111111111",
                    ],
                    stdout=log,
                    stderr=subprocess.STDOUT,
                )
                window = attach(api, bus, registry, process)
                node = manuscript(window)
                window.focus(node)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(node) == "The last tram waits.x", "pending prose")
                command(window, "Build scenes")
                window.wait(lambda: window.find("Build saved"), "build workspace")
                window.act("Output · opening · build/first.recitec")
                window.act("Build saved")
                first = project / "build/first.recitec"
                second = project / "build/second.recitec"
                window.wait(first.is_file, "selected output published")
                window.wait(lambda: not window.find("Cancel build"), "selected build finished")
                assert not second.exists()
                assert source.read_text() == SOURCE
                before = first.read_bytes()
                capture_settled(window, output, "-selected")
                results["selected_target_limits_outputs_and_build_saved_keeps_pending_prose"] = True
                window.act("Return to writing")
                assert manuscript(window) == node
                window.focus(node)
                key("z", control=True)
                window.wait(lambda: window.text(node) == "The last tram waits.", "retained undo")
                key("y", control=True)
                window.wait(lambda: window.text(node) == "The last tram waits.x", "retained redo")
                results["build_workspace_preserves_native_field_and_history"] = True
                command(window, "Build scenes")
                window.wait(lambda: window.find("All outputs"), "retained build target")
                window.act("All outputs")
                window.act("Save all and build")
                window.wait(
                    lambda: source.read_text() == SOURCE.replace("waits.", "waits.x"),
                    "explicit save before build",
                )
                window.wait(second.is_file, "all output targets published")
                window.wait(lambda: first.read_bytes() != before, "saved prose in build")
                window.wait(lambda: not window.find("Cancel build"), "save-build finished")
                capture_settled(window, output, "-all")
                results["save_all_and_build_publishes_prose_and_all_targets"] = True
                window.close_clean()
                assert process.wait(timeout=5) == 0
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
        if process is not None and process.poll() is None:
            capture(output, "-error")
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
    output.write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results, indent=2))
    return "error" not in results
