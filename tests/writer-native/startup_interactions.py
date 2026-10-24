"""Welcome, project creation, native activation and genuinely temporary examples."""

import json
import subprocess
import tempfile
import traceback
from pathlib import Path

from native_window import attach, key, type_text
from project_interactions import document_caption, manuscript
from writer_interactions import SOURCE, capture


def field(window, name):
    return window.wait(
        lambda: next(
            (
                node
                for node, label in window.nodes()
                if label == name and window.call(node, "Accessible", "GetRole")[0] == 79
            ),
            None,
        ),
        name + " editable field",
    )


def fill(window, name, value):
    node = field(window, name)
    window.focus(node)
    key("a", control=True)
    type_text(value)
    window.wait(lambda: window.text(node) == value, name + " entered")


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "startup"}
    process, window = None, None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-startup-") as temporary:
            root = Path(temporary)
            project = root / "project"
            project.mkdir()
            (project / "recite.project.toml").write_text(
                'format_version = 1\n[project]\ncontent_set = "startup"\nversion = "1"\n'
            )
            (project / "scene.recite").write_text(SOURCE)
            (project / "zz-other.recite").write_text(
                SOURCE.replace("The last tram waits.", "Another document.")
                .replace("opening", "other")
                .replace("11111111111111111111", "33333333333333333333")
                .replace("22222222222222222222", "44444444444444444444")
            )
            with output.with_suffix(".writer.log").open("w") as log:
                process = subprocess.Popen([str(binary)], stdout=log, stderr=subprocess.STDOUT)
                window = attach(api, bus, registry, process)
                field(window, "Project folder")
                assert not window.find("Save")
                capture(output, "-welcome")
                results["welcome_has_no_fictitious_document"] = True
                fill(window, "Project folder", str(project))
                window.act("Open selected project")
                node = manuscript(window)
                window.wait(lambda: window.text(node) == "The last tram waits.", "opened project")
                window.focus(node)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(node).endswith("waits.x"), "native draft")
                window.workspace_action("Projects")
                field(window, "Project folder")
                capture(output, "-projects")
                window.act("Return to Writer")
                node = manuscript(window)
                window.focus(node)
                key("z", control=True)
                window.wait(
                    lambda: window.text(node) == "The last tram waits.", "retained native undo"
                )
                results["project_form_preserves_native_history"] = True
                forwarded = subprocess.run(
                    [str(binary), "--project", str(project), "--document", "zz-other.recite"],
                    stdout=log,
                    stderr=subprocess.STDOUT,
                    timeout=10,
                )
                assert forwarded.returncode == 0
                window.wait(
                    lambda: document_caption(window, "zz-other.recite", selected=True),
                    "forwarded document selector",
                )
                results["second_launch_selects_requested_document"] = True
                capture(output, "-opened")
                window.workspace_action("Projects")
                field(window, "Project folder")
                window.act("Create a new project")
                field(window, "Project name")
                capture(output, "-create-form")
                created = root / "created"
                fill(window, "Project folder", str(created))
                fill(window, "Project name", "A native beginning")
                window.act("Create project")
                window.wait(
                    lambda: created.joinpath("recite.project.toml").is_file(),
                    "starter project files",
                )
                window.wait(lambda: not window.find("Create project"), "created project opened")
                manuscript(window)
                assert (
                    'content_set = "a_native_beginning"'
                    in created.joinpath("recite.project.toml").read_text()
                )
                results["created_project_uses_shared_starter"] = True
                capture(output, "-created")
                window.workspace_action("Projects")
                field(window, "Project folder")
                window.act("Try temporary examples")
                window.wait(
                    lambda: window.find("Temporary examples"), "temporary example navigation"
                )
                assert not window.find("Save")
                node = manuscript(window)
                before = window.text(node)
                window.focus(node)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(node) == before + "x", "temporary example draft")
                capture(output, "-examples")
                window.document_action("Close")
                window.wait(
                    lambda: window.find("Close and discard example changes"),
                    "temporary close choice",
                )
                assert not window.find("Save all and close")
                assert not window.find("Close and keep recovery")
                capture(output, "-example-close")
                window.act("Close and discard example changes")
                assert process.wait(timeout=5) == 0
                results["temporary_examples_have_explicit_discard_boundary"] = True
    except Exception as error:
        results["error"] = repr(error)
        results["error_traceback"] = traceback.format_exc()
        if process is not None and process.poll() is None:
            capture(output, "-error")
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
    output.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(results, ensure_ascii=False))
    return "error" not in results
