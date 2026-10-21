"""Automatic notices require explicit comparison and preserve source and PO drafts."""

import json
import os
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
from native_window import attach, key, type_text
from project_interactions import manuscript
from startup_interactions import field
from writer_interactions import SOURCE, capture


def replace(path, source):
    temporary = path.with_suffix(".replacement")
    temporary.write_text(source)
    temporary.replace(path)


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "disk-notices"}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-notices-", dir="/tmp") as temporary:
            root = Path(temporary)
            for scale in [100, 200]:
                project = root / str(scale)
                project.mkdir()
                (project / "recite.project.toml").write_text("format_version=1\n")
                source, po = project / "scene.recite", project / "fr.po"
                source.write_text(SOURCE)
                po.write_text(PO)
                config = root / f"ui-{scale}.toml"
                config.write_text(f"config_version=1\n[writer.presentation]\nui_scale={scale}\n")
                with output.with_suffix(f".{scale}.writer.log").open("w") as log:
                    process = subprocess.Popen(
                        [str(binary), "--project", str(project)],
                        env=dict(os.environ, RECITE_CONFIG=str(config)),
                        stdout=log,
                        stderr=subprocess.STDOUT,
                    )
                    window = attach(api, bus, registry, process)
                    native = manuscript(window)
                    original = window.text(native)
                    window.focus(native)
                    key("End", control=True)
                    type_text("x")
                    window.wait(lambda: window.text(native) == original + "x", "pending source")
                    disk = SOURCE.replace("The last tram waits.", "The external tram left.")
                    replace(source, disk)
                    window.reveal_start("Compare changed file")
                    window.wait(
                        lambda: window.find_text("Changed on disk · 1 file"), "source notice"
                    )
                    capture(output, f"-source-{scale}")
                    action(window, "Compare changed file")
                    reveal_text(window, "Disk comparison ready.", prefix=True)
                    assert source.read_text() == disk
                    action(window, "Return to writing")
                    reveal_field(window, "Manuscript")
                    native = manuscript(window)
                    assert window.text(native) == original + "x"
                    window.focus(native)
                    key("z", control=True)
                    window.wait(lambda: window.text(native) == original, "retained source Undo")
                    key("y", control=True)
                    window.wait(
                        lambda: window.text(native) == original + "x", "retained source Redo"
                    )
                    results[f"source_notice_compare_retains_draft_and_native_history_{scale}"] = (
                        True
                    )
                    action(window, "Dismiss notice")
                    open_catalogue(window, po)
                    open_entry(window)
                    edit(window, "Une traduction à conserver.")
                    po_disk = PO.replace("Le dernier tram attend.", "Une version externe.")
                    replace(po, po_disk)
                    window.reveal_start("Compare changed file")
                    window.wait(
                        lambda: window.find_text("Changed on disk · 1 file"), "catalogue notice"
                    )
                    capture(output, f"-catalogue-{scale}")
                    action(window, "Compare changed file")
                    reveal_text(window, "Catalogue comparison ready.", prefix=True)
                    assert po.read_text() == po_disk
                    assert source.read_text() == disk
                    action(window, "Keep translation drafts")
                    reveal_text(window, "Catalogue ready.", prefix=True)
                    open_entry(window)
                    assert (
                        window.text(field(window, "Translation form 1"))
                        == "Une traduction à conserver."
                    )
                    results[
                        f"catalogue_notice_explicit_compare_retains_draft_without_writes_{scale}"
                    ] = True
                    process.terminate()
                    process.wait(timeout=10)
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
        if process and process.poll() is None:
            results["exposure_diagnostic"] = window.exposure_diagnostic()
            capture(output, "-error")
    finally:
        if process and process.poll() is None:
            process.terminate()
            process.wait(timeout=10)
    output.write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results))
    return "error" not in results
