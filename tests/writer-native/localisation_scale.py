"""Doubled UI scale and independently enlarged reading through the native command picker."""

import json
import os
import subprocess
import tempfile
import traceback
from pathlib import Path

from localisation_interactions import PO, edit, open_catalogue, open_entry, reveal
from native_window import attach
from project_interactions import manuscript
from writer_interactions import SOURCE, capture


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "localisation-scale"}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-po-scale-", dir="/tmp") as temporary:
            root = Path(temporary)
            (root / "recite.project.toml").write_text("format_version=1\n")
            (root / "scene.recite").write_text(SOURCE)
            po = root / "fr.po"
            po.write_text(PO)
            config = root / "large-ui.toml"
            config.write_text(
                "config_version=1\n[writer.presentation]\nui_scale=200\nreading_size=32\n"
            )
            with output.with_suffix(".writer.log").open("w") as log:
                process = subprocess.Popen(
                    [str(binary), "--project", str(root)],
                    env=dict(os.environ, RECITE_CONFIG=str(config)),
                    stdout=log,
                    stderr=subprocess.STDOUT,
                )
                window = attach(api, bus, registry, process)
                manuscript(window)
                open_catalogue(window, po)
                open_entry(window)
                edit(window, "Le dernier tram attend dans la nuit.")
                reveal(window, "Save translation")
                capture(output, "-entry200-reading32")
                window.act("Save translation")
                window.wait(
                    lambda: "Le dernier tram attend dans la nuit." in po.read_text(),
                    "large-reading translation saved",
                )
                results["command_search_catalogue_edit_and_save_at_200_ui_and_32_reading"] = True
                window.reveal_start("Close")
                window.close_clean()
                assert process.wait(timeout=10) == 0
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
