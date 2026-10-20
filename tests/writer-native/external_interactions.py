"""Observed disk decisions and an IDE merge preserve pending manuscript and explicit saves."""

import json
import os
import subprocess
import tempfile
import traceback
from pathlib import Path

from native_window import attach, key, type_text
from project_interactions import manuscript
from writer_interactions import SOURCE, capture


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "external"}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-external-", dir="/tmp") as temporary:
            root = Path(temporary)
            project = root / "Project [literal] $source"
            project.mkdir()
            (project / "recite.project.toml").write_text("format_version=1\n")
            source = project / "scene.recite"
            source.write_text(SOURCE)
            receipt = root / "editor-argv.json"
            editor = root / "owned editor"
            editor.write_text(
                "#!/usr/bin/python3\nimport json,sys\nfrom pathlib import Path\n"
                f"Path({str(receipt)!r}).write_text(json.dumps(sys.argv[1:]))\n"
            )
            editor.chmod(0o755)
            with output.with_suffix(".writer.log").open("w") as log:
                process = subprocess.Popen(
                    [str(binary), "--project", str(project), "--editor", str(editor)],
                    stdout=log, stderr=subprocess.STDOUT,
                )
                window = attach(api, bus, registry, process)
                node = manuscript(window)
                prose = window.text(node)
                window.focus(node)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(node) == prose + "x", "pending prose")
                disk = SOURCE.replace("The last tram waits.", "The external tram left.")
                source.write_text(disk)
                window.act("Compare disk changes")
                window.wait(lambda: window.find_text("Disk comparison ready.", prefix=True), "observational comparison")
                assert source.read_text() == disk
                window.reveal_start("Return to writing")
                capture(output, "-comparison")
                window.reveal_start("Keep Writer version")
                window.act("Keep Writer version")
                window.wait(lambda: window.find_text("Kept Writer's document", prefix=True), "keep Writer adoption")
                window.reveal_start("Return to writing")
                window.act("Return to writing")
                current = manuscript(window)
                assert tuple(current) == tuple(node)
                assert window.text(current) == prose + "x"
                assert source.read_text() == disk
                results["comparison_and_keep_preserve_mounted_pending_field_without_source_write"] = True
                window.focus(current)
                key("z", control=True)
                window.wait(lambda: window.text(current) == prose, "retained native undo")
                key("y", control=True)
                window.wait(lambda: window.text(current) == prose + "x", "retained native redo")
                results["keep_retains_native_field_undo_and_redo"] = True
                window.act("Compare disk changes")
                window.wait(lambda: window.find_text("Disk comparison ready.", prefix=True), "fresh merge comparison")
                window.reveal_end("Export merge draft")
                window.act("Export merge draft")
                window.wait(lambda: window.find_text("Merge draft exported.", prefix=True), "detached merge draft")
                window.reveal_end("Open merge draft in editor")
                window.act("Open merge draft in editor")
                window.wait(receipt.exists, "literal IDE argument")
                arguments = json.loads(receipt.read_text())
                assert len(arguments) == 1
                draft = Path(arguments[0])
                assert draft.parent == project and draft.suffix == ".txt"
                assert prose + "x" in draft.read_text()
                merged = SOURCE.replace("The last tram waits.", "Reviewed merged tram.")
                draft.write_text(merged)
                window.wait(lambda: window.enabled("Open merge draft in editor"), "launcher completion")
                window.reveal_end("Import reviewed merge")
                window.act("Import reviewed merge")
                window.wait(lambda: window.find_text("Reviewed merge adopted", prefix=True), "checked merge adoption")
                assert source.read_text() == disk
                assert list(project.glob(".recite-recovered-*.json"))
                window.reveal_start("Return to writing")
                window.act("Return to writing")
                window.wait(lambda: window.text(manuscript(window)) == "Reviewed merged tram.", "selected merged prose")
                results["ide_merge_import_validates_and_adopts_without_writing_source"] = True
                capture(output, "-adopted")
                window.reveal_start("Save file")
                window.act("Save file")
                window.wait(lambda: source.read_text() == merged, "explicit merge save")
                results["explicit_save_publishes_merge_against_inspected_disk_baseline"] = True
                window.close_clean()
                assert process.wait(timeout=5) == 0
                config = root / "large-ui.toml"
                config.write_text("config_version=1\n[writer.presentation]\nui_scale=200\n")
                process = subprocess.Popen(
                    [str(binary), "--project", str(project)],
                    env=dict(os.environ, RECITE_CONFIG=str(config)),
                    stdout=log, stderr=subprocess.STDOUT,
                )
                window = attach(api, bus, registry, process)
                manuscript(window)
                source.write_text(merged.replace("Reviewed merged tram.", "A newer external tram."))
                window.reveal_start("Compare disk changes")
                window.act("Compare disk changes")
                window.wait(lambda: window.find_text("Disk comparison ready.", prefix=True), "large-scale comparison")
                capture(output, "-comparison200")
                window.reveal_start("Return to writing")
                capture(output, "-comparison200-top")
                scrollbar = window.wait(window._outer_scrollbar, "large comparison scrollbar")
                window.focus(scrollbar)
                for _ in range(3):
                    key("Down")
                window.wait(lambda: window.find_text("A newer external tram.", contains=True), "large source comparison")
                capture(output, "-comparison200-sources")
                window.reveal_start("Close")
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
