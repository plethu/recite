"""Native PO editing, exact disk decisions, Save All and recoverable close."""

import json
import subprocess
import tempfile
import traceback
from pathlib import Path

from native_window import attach, key, type_text
from project_interactions import manuscript
from project_tools_interactions import command
from startup_interactions import field, fill
from writer_interactions import SOURCE, capture

PO = """#. file: scene.recite
#. block: opening
# translator note
msgctxt "11111111111111111111"
msgid "The last tram waits."
msgstr "Le dernier tram attend."
"""


def reveal(window, label, prefix=False):
    """Page through the native scrollbar to expose controls between Home and End."""
    if window.find(label, prefix=prefix):
        return
    scrollbar = window.wait(window._outer_scrollbar, "outer scrollbar")
    window.focus(scrollbar)
    key("Home")

    def page():
        if window.find(label, prefix=prefix):
            return True
        key("Page_Down")
        key("Up")
        return bool(window.find(label, prefix=prefix))

    window.wait(page, "scrolled control: " + label)


def open_catalogue(window, path):
    command(window, "Localise")
    reveal(window, "Catalogue path")
    fill(window, "Catalogue path", str(path))
    window.reveal_end("Open catalogue")
    window.act("Open catalogue")
    reveal(window, "Catalogue files")


def open_entry(window):
    reveal(window, "Open translation · ", prefix=True)
    window.wait(
        lambda: any(name.startswith("Open translation · ") for _, name in window.nodes()),
        "queue entry",
    )
    caption = next(name for _, name in window.nodes() if name.startswith("Open translation · "))
    window.act(caption)
    reveal(window, "Return to translation queue")
    reveal(window, "Translation form 1")
    return field(window, "Translation form 1")


def edit(window, value):
    reveal(window, "Translation form 1")
    fill(window, "Translation form 1", value)


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "localisation"}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-po-", dir="/tmp") as temporary:
            root = Path(temporary)
            project = root / "project"
            project.mkdir()
            (project / "recite.project.toml").write_text("format_version=1\n")
            source = project / "scene.recite"
            source.write_text(SOURCE)
            po = project / "fr.po"
            po.write_text(PO)
            with output.with_suffix(".writer.log").open("w") as log:
                process = subprocess.Popen(
                    [str(binary), "--project", str(project)], stdout=log, stderr=subprocess.STDOUT
                )
                window = attach(api, bus, registry, process)
                native = manuscript(window)
                before = window.text(native)
                window.focus(native)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(native) == before + "x", "retained source draft")
                open_catalogue(window, po)
                capture(output, "-queue")
                open_entry(window)
                edit(window, "Le dernier tram part.")
                window.reveal_end("Reviewed translation")
                window.act("Reviewed translation")
                window.act("Save translation")
                window.wait(
                    lambda: "Le dernier tram part." in po.read_text(), "explicit translation save"
                )
                assert "# translator note" in po.read_text()
                assert "fuzzy" not in po.read_text()
                assert source.read_text() == SOURCE
                results[
                    "translation_save_preserves_metadata_and_keeps_source_draft_independent"
                ] = True
                capture(output, "-entry")
                window.reveal_start("Return to writing")
                window.act("Return to writing")
                current = manuscript(window)
                assert tuple(current) == tuple(native)
                assert window.text(current) == before + "x"
                window.focus(current)
                key("z", control=True)
                window.wait(lambda: window.text(current) == before, "retained manuscript undo")
                key("y", control=True)
                window.wait(
                    lambda: window.text(current) == before + "x", "retained manuscript redo"
                )
                results["translation_workspace_retains_mounted_manuscript_and_native_undo"] = True
                command(window, "Localise")
                edit(window, "Un brouillon conservé.")
                disk = (
                    po.read_text()
                    .replace("Le dernier tram part.", "Le tram est parti.")
                    .replace("# translator note", "# external translator note")
                )
                po.write_text(disk)
                window.reveal_end("Save translation")
                window.act("Save translation")
                window.wait(
                    lambda: window.find_text(
                        "Catalogue save or recovery is incomplete:", prefix=True
                    ),
                    "conflict reported",
                )
                assert po.read_text() == disk
                window.reveal_start("Return to translation queue")
                window.act("Return to translation queue")
                window.wait(
                    lambda: window.find_text("Catalogue ready.", prefix=True), "queue restored"
                )
                window.reveal_start("Compare catalogue disk changes")
                window.act("Compare catalogue disk changes")
                window.wait(
                    lambda: window.find_text("Catalogue comparison ready.", prefix=True),
                    "catalogue comparison",
                )
                window.reveal_end("Keep translation drafts")
                capture(output, "-comparison")
                window.act("Keep translation drafts")
                window.wait(
                    lambda: window.find_text("Catalogue ready.", prefix=True), "kept local drafts"
                )
                assert po.read_text() == disk
                open_entry(window)
                assert window.text(field(window, "Translation form 1")) == "Un brouillon conservé."
                results[
                    "disk_conflict_requires_inspection_and_keeps_the_chosen_draft_without_writing_po"
                ] = True
                window.reveal_start("Save all")
                window.act("Save all")
                window.wait(
                    lambda: (
                        "Un brouillon conservé." in po.read_text()
                        and "The last tram waits.x" in source.read_text()
                    ),
                    "source and catalogue Save All",
                )
                assert "# external translator note" in po.read_text()
                results["save_all_publishes_both_layers_and_preserves_external_po_metadata"] = True
                edit(window, "À retrouver après réouverture.")
                window.reveal_start("Close")
                window.act("Close")
                window.wait(
                    lambda: window.find("Close and keep recovery"), "translation close guard"
                )
                capture(output, "-recovery-close")
                window.act("Close and keep recovery")
                assert process.wait(timeout=10) == 0
                process = subprocess.Popen(
                    [str(binary), "--project", str(project)], stdout=log, stderr=subprocess.STDOUT
                )
                window = attach(api, bus, registry, process)
                manuscript(window)
                open_catalogue(window, po)
                open_entry(window)
                assert (
                    window.text(field(window, "Translation form 1"))
                    == "À retrouver après réouverture."
                )
                assert "Un brouillon conservé." in po.read_text()
                results["close_recovery_restores_translation_drafts_without_publishing_them"] = True
                window.reveal_start("Close")
                window.act("Close")
                window.wait(lambda: window.find("Save all and close"), "saved close continuation")
                window.act("Save all and close")
                assert process.wait(timeout=10) == 0
                assert "À retrouver après réouverture." in po.read_text()
                results["save_all_and_close_harvests_catalogue_and_source_before_exit"] = True
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
