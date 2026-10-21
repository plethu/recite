"""Catalogue history, field-local Save, and explicit draft discard on owned Wayland."""

import json
import os
import subprocess
import tempfile
import time
import traceback
from pathlib import Path

from localisation_interactions import (
    PO,
    edit,
    open_catalogue,
    open_entry,
    reveal,
    reveal_field,
    search_entries,
)
from native_window import attach, key
from project_interactions import manuscript
from project_tools_interactions import command
from startup_interactions import field, fill
from writer_interactions import SOURCE, capture


def queue_ready(window):
    window.reveal_start("Copy link")
    window.wait(lambda: window.find_text("Catalogue ready.", prefix=True), "queue publication")


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "localisation-final"}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-po-final-", dir="/tmp") as temporary:
            for scale in [100, 200]:
                project = Path(temporary) / str(scale)
                project.mkdir()
                (project / "recite.project.toml").write_text("format_version=1\n")
                source = project / "scene.recite"
                source.write_text(SOURCE)
                po = project / "fr.po"
                po.write_text(
                    PO
                    + '\n#. file: scene.recite\n#. block: opening\nmsgctxt "22222222222222222222"\nmsgid "A second paragraph."\nmsgstr "Un second paragraphe."\n'
                )
                config = project / "config.toml"
                config.write_text(f"config_version=1\n[writer.presentation]\nui_scale={scale}\n")
                environment = dict(os.environ, RECITE_CONFIG=str(config))
                arguments = [str(binary), "--project", str(project)]
                with output.with_suffix(f".{scale}.writer.log").open("w") as log:
                    process = subprocess.Popen(
                        arguments, env=environment, stdout=log, stderr=subprocess.STDOUT
                    )
                    window = attach(api, bus, registry, process)
                    fill(window, "Manuscript", "Unapplied source stays private.")
                    open_catalogue(window, po)
                    search_entries(window, "tram")
                    open_entry(window)
                    edit(window, "Saved by the translation shortcut.")
                    window.focus(field(window, "Translation form 1"))
                    key("s", control=True)
                    window.wait(
                        lambda: "Saved by the translation shortcut." in po.read_text(),
                        "translation PO write",
                    )
                    window.reveal_start("Copy link")
                    window.wait(
                        lambda: window.find_text("Translations saved."),
                        "translation-local shortcut completion",
                    )
                    assert "Saved by the translation shortcut." in po.read_text()
                    assert source.read_text() == SOURCE
                    window.reveal_start("Return to writing")
                    window.act("Return to writing")
                    assert window.text(manuscript(window)) == "Unapplied source stays private."
                    results[f"translation_shortcut_preserves_source_and_manuscript_{scale}"] = True
                    command(window, "Localise")
                    window.reveal_start("Return to translation queue")
                    window.act("Return to translation queue")
                    queue_ready(window)
                    open_entry(window)
                    key("Left", alt=True)
                    reveal_field(window, "Search translations")
                    assert window.text(field(window, "Search translations")) == "tram"
                    key("Right", alt=True)
                    reveal_field(window, "Translation form 1")
                    assert (
                        window.text(field(window, "Translation form 1"))
                        == "Saved by the translation shortcut."
                    )
                    capture(output, f"-history-{scale}")
                    results[f"ordinary_history_and_alt_arrows_restore_catalogue_query_{scale}"] = (
                        True
                    )
                    edit(window, "Discard first translation.")
                    window.reveal_start("Return to translation queue")
                    window.act("Return to translation queue")
                    queue_ready(window)
                    search_entries(window, "second")
                    open_entry(window)
                    edit(window, "Discard second translation.")
                    window.document_action("Close")
                    window.wait(
                        lambda: window.find("Discard translations and close"),
                        "explicit translation discard",
                    )
                    window.focus(window._outer_scrollbar())
                    key("Home")
                    time.sleep(0.15)
                    capture(output, f"-close-context-{scale}")
                    if scale == 200:
                        key("Page_Down")
                        time.sleep(0.15)
                    reveal(window, "Discard translations and close")
                    time.sleep(0.15)
                    capture(output, f"-discard-{scale}")
                    window.act("Discard translations and close")
                    assert process.wait(timeout=10) == 0
                    assert source.read_text() == SOURCE
                    assert "Discard" not in po.read_text()
                    assert "Saved by the translation shortcut." in po.read_text()
                    process = subprocess.Popen(
                        arguments, env=environment, stdout=log, stderr=subprocess.STDOUT
                    )
                    window = attach(api, bus, registry, process)
                    manuscript(window)
                    open_catalogue(window, po)
                    search_entries(window, "tram")
                    open_entry(window)
                    assert (
                        window.text(field(window, "Translation form 1"))
                        == "Saved by the translation shortcut."
                    )
                    window.reveal_start("Return to translation queue")
                    window.act("Return to translation queue")
                    queue_ready(window)
                    search_entries(window, "second")
                    open_entry(window)
                    assert (
                        window.text(field(window, "Translation form 1")) == "Un second paragraphe."
                    )
                    results[
                        f"discard_all_translations_does_not_restore_po_drafts_or_write_source_{scale}"
                    ] = True
                    window.document_action("Close")
                    window.wait(
                        lambda: window.find("Close and keep recovery"), "retained source recovery"
                    )
                    window.act("Close and keep recovery")
                    assert process.wait(timeout=10) == 0
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
        if process is not None and process.poll() is None:
            capture(output, "-error")
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
    output.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(results, ensure_ascii=False))
    return "error" not in results
