"""Catalogue-to-source navigation and staged previews across native document fields."""

import json
import os
import subprocess
import tempfile
import traceback
from pathlib import Path

from localisation_interactions import (
    edit,
    open_catalogue,
    open_entry,
    reveal,
    reveal_field,
    reveal_text,
    search_entries,
)
from localisation_sources import action
from native_window import attach, key
from preview_inputs import capture_end, restart
from project_interactions import manuscript
from project_tools_interactions import command
from startup_interactions import field, fill
from writer_interactions import capture

SOURCE = ":: start default\n> @ffee6p8\n  Hello.\n-> END\n"
OTHER = ":: letters\n> @rthh7ab\n  A letter.\n-> END\n"
PO = """msgid ""
msgstr "Language: fr\\n"

#. file: zz-other.recite
#. block: letters
# translator note
msgctxt "rthh7ab&formal"
msgid "A letter."
msgstr "Un courrier."
"""


def exercise(window, project, output, results):
    original = manuscript(window)
    fill(window, "Manuscript", "Kept source draft.")
    po = project / "fr.po"
    open_catalogue(window, po)
    search_entries(window, "formal")
    reveal_text(window, "1 matching entries", prefix=True)
    open_entry(window)
    edit(window, "Un brouillon distant.")
    capture(output, "-editing")
    reveal(window, "Preview this passage")
    capture(output, "-actions")
    action(window, "Show source passage")
    reveal_field(window, "Manuscript")
    window.wait(lambda: window.text(manuscript(window)) == "A letter.", "linked source selected")
    assert manuscript(window) != original
    assert (project / "scene.recite").read_text() == SOURCE
    results["source_action_selects_cross_file_passage_without_saving"] = True
    capture(output, "-source")
    command(window, "Localise")
    search_entries(window, "formal")
    open_entry(window)
    reveal_field(window, "Translation form 1")
    assert window.text(field(window, "Translation form 1")) == "Un brouillon distant."
    action(window, "Preview this passage")
    window.wait(lambda: window.find("Start preview"), "initial preview action")
    action(window, "Configure preview")
    reveal_field(window, "Preview wording")
    assert window.text(field(window, "Preview wording")) == "formal"
    capture(output, "-inputs")
    restart(window, "Un courrier.")
    results["preview_action_stages_catalogue_language_and_wording"] = True
    action(window, "Include unsaved translations")
    restart(window, "Un brouillon distant.")
    capture_end(window, output, "-draft-preview")
    assert po.read_text() == PO
    results["preview_can_capture_draft_without_publishing_po"] = True
    action(window, "Return to writing")
    key("p", control=True)
    fill(window, "Search query", "scene.recite")
    key("Return")
    window.wait(lambda: not window.find("Search query"), "explicit source document navigation")
    reveal_field(window, "Manuscript")
    window.wait(
        lambda: window.text(manuscript(window)) == "Kept source draft.",
        "outgoing source draft restored",
    )
    assert (project / "scene.recite").read_text() == SOURCE
    results["original_source_edit_restored"] = True


def run(api, bus, registry, binary, output, scale=100):
    results = {"backend": "owned Wayland", "scenario": "localisation-navigation", "ui_scale": scale}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-po-navigation-", dir="/tmp") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text("format_version=1\n")
            (project / "scene.recite").write_text(SOURCE)
            (project / "zz-other.recite").write_text(OTHER)
            (project / "fr.po").write_text(PO)
            config = project / "ui.toml"
            config.write_text(f"config_version=1\n[writer.presentation]\nui_scale={scale}\n")
            with output.with_suffix(".writer.log").open("w") as log:
                process = subprocess.Popen(
                    [str(binary), "--project", str(project), "--document", "scene.recite"],
                    env=dict(os.environ, RECITE_CONFIG=str(config)),
                    stdout=log,
                    stderr=subprocess.STDOUT,
                )
                window = attach(api, bus, registry, process)
                try:
                    exercise(window, project, output, results)
                except Exception:
                    results["visible_at_failure"] = [name for _, name in window.nodes()]
                    capture(output, "-error")
                    raise
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
    finally:
        if process and process.poll() is None:
            process.terminate()
            process.wait(timeout=10)
    output.write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results))
    return "error" not in results


def run_scale(api, bus, registry, binary, output):
    return run(api, bus, registry, binary, output, 200)
