"""A saved catalogue does not prevent leaving its project; drafts still guard replacement."""

import json
import subprocess
import tempfile
import traceback
from pathlib import Path

from localisation_interactions import (
    PO,
    edit,
    open_catalogue,
    open_entry,
    reveal_field,
    reveal_text,
)
from localisation_sources import action
from native_window import attach
from project_interactions import manuscript
from startup_interactions import field, fill
from writer_interactions import SOURCE, capture


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "project-replacement"}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-replacement-", dir="/tmp") as temporary:
            root = Path(temporary)
            first, second = root / "first", root / "second"
            for project in [first, second]:
                project.mkdir()
                (project / "recite.project.toml").write_text("format_version=1\n")
                (project / "scene.recite").write_text(SOURCE)
            second_source = SOURCE.replace("The last tram waits.", "A different project opens.")
            (second / "scene.recite").write_text(second_source)
            po = first / "fr.po"
            po.write_text(PO)
            with output.with_suffix(".writer.log").open("w") as log:
                process = subprocess.Popen(
                    [str(binary), "--project", str(first)], stdout=log, stderr=subprocess.STDOUT
                )
                window = attach(api, bus, registry, process)
                manuscript(window)
                open_catalogue(window, po)
                open_entry(window)
                edit(window, "Une traduction à conserver.")
                action(window, "Projects")
                fill(window, "Project folder", str(second))
                action(window, "Open selected project")
                reveal_text(window, "Return to Writer and save changes", prefix=True)
                capture(output, "-guarded")
                action(window, "Return to Writer")
                reveal_field(window, "Translation form 1")
                assert (
                    window.text(field(window, "Translation form 1"))
                    == "Une traduction à conserver."
                )
                assert po.read_text() == PO
                results["unsaved_catalogue_blocks_replacement_and_retains_its_draft"] = True
                action(window, "Save translation")
                reveal_text(window, "Translations saved.")
                assert "Une traduction à conserver." in po.read_text()
                action(window, "Projects")
                fill(window, "Project folder", str(second))
                action(window, "Open selected project")
                node = manuscript(window)
                window.wait(
                    lambda: window.text(node) == "A different project opens.", "new project"
                )
                capture(output, "-opened")
                assert (first / "scene.recite").read_text() == SOURCE
                assert (second / "scene.recite").read_text() == second_source
                results["saved_open_catalogue_allows_project_replacement_without_manual_close"] = (
                    True
                )
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
        if process and process.poll() is None:
            capture(output, "-error")
    finally:
        if process and process.poll() is None:
            process.terminate()
            process.wait(timeout=10)
    output.write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results))
    return "error" not in results
