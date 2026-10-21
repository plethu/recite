"""Explicit disk reload archives local work and starts a fresh native field lifetime."""

import json
import tempfile
import time
import traceback
from pathlib import Path

from native_window import key, type_text
from project_interactions import manuscript
from writer_interactions import SOURCE, capture, launch


def run(api, bus, registry, binary, output):
    results = {"host": "owned Cage, D-Bus and project", "physical_ime": False}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-reload-") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text("format_version = 1\n")
            source = project / "scene.recite"
            source.write_text(SOURCE)
            arguments = [str(binary), "--project", str(project)]
            with output.with_suffix(".log").open("w") as log:
                process, window, node = launch(api, bus, registry, arguments, log)
                window.focus(node)
                key("a", control=True)
                type_text("Preserve this unapplied field.")
                window.wait(
                    lambda: window.text(node) == "Preserve this unapplied field.", "local draft"
                )
                disk = SOURCE.replace("The last tram waits.", "The tram has left.")
                source.write_text(disk)
                started = time.monotonic()
                assert window.document_action("Reload (keep recovery copy)")
                window.wait(
                    lambda: window.text(manuscript(window)) == "The tram has left.", "disk adoption"
                )
                window.wait(lambda: window.enabled("Save file"), "reload action completed")
                results["reload_seconds"] = time.monotonic() - started
                copies = list(project.glob(".recite-recovered-*.json"))
                assert len(copies) == 1
                archived = json.loads(copies[0].read_text())
                assert archived["baseline"] == SOURCE
                assert archived["draft"]["draft"] == "Preserve this unapplied field."
                assert source.read_text() == disk
                results["checked_reload_keeps_complete_independent_archive"] = True
                node = manuscript(window)
                window.focus(node)
                key("z", control=True)
                time.sleep(0.1)  # A no-op native undo emits no text event.
                assert window.text(node) == "The tram has left."
                results["reload_resets_native_field_history"] = True
                capture(output, "-accepted")
                key("End", control=True)
                type_text(" New typing.")
                window.wait(
                    lambda: window.text(node) == "The tram has left. New typing.",
                    "post-reload draft",
                )
                window.document_action("Close")
                window.wait(lambda: window.find("Close and keep recovery"), "recovery close")
                window.act("Close and keep recovery")
                assert process.wait(timeout=5) == 0
                process, window, node = launch(api, bus, registry, arguments, log)
                window.wait(
                    lambda: window.text(node) == "The tram has left. New typing.", "draft restart"
                )
                results["post_reload_typing_survives_restart"] = True
                capture(output, "-recovered")
                window.document_action("Close")
                window.wait(lambda: window.find("Save all and close"), "final close")
                window.save_and_close(results, "post_reload_save_close")
                assert "The tram has left. New typing." in source.read_text()
    except Exception as error:
        results["error"] = repr(error)
        results["error_traceback"] = traceback.format_exc()
        if process is not None and process.poll() is None:
            capture(output, "-error")
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
    output.write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results))
    return "error" not in results
