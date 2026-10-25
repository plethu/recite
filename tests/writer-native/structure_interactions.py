"""Create and inspect accepted Script content through native Writer controls."""

import json
import tempfile
import time
import traceback
from pathlib import Path

from native_window import key, type_text
from project_interactions import manuscript


def run(api, bus, registry, binary, output):
    from writer_interactions import capture, launch

    results = {"host": "owned Cage, D-Bus and project", "physical_ime": False}
    try:
        with tempfile.TemporaryDirectory(prefix="recite-structure-") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text(
                'format_version = 1\n[project]\ncontent_set = "trial"\nversion = "1"\n'
            )
            source = project / "scene.recite"
            arguments = [str(binary), "--project", str(project)]
            with output.with_suffix(".log").open("w") as log:
                exercise(
                    api, bus, registry, arguments, log, source, output, results, launch, capture
                )
    except Exception as error:
        results["error"] = repr(error)
        results["error_traceback"] = traceback.format_exc()
    output.write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results))
    return "error" not in results


def exercise(api, bus, registry, arguments, log, source, output, results, launch, capture):
    source.write_text("# A new scene.\n")
    process, window, _ = launch(api, bus, registry, arguments, log, manuscript=False)
    try:
        window.wait(lambda: window.find("No beats yet"), "valid empty document")
        window.document_action("Configure preview")
        window.wait(lambda: window.find("Start preview"), "initial preview action")
        assert not window.enabled("Start preview")
        capture(output, "-empty")
        window.act("Return to writing")
        window.reveal_start("Add beat")
        window.act("Add beat")
        window.reveal_end("Edit passage 1")
        capture(output, "-new-beat")
        window.edit_passage(1)
        node = window.wait(lambda: manuscript(window), "created line editor")
        window.focus(node)
        key("a", control=True)
        type_text("The new scene begins.")
        window.wait(lambda: window.text(node) == "The new scene begins.", "new line draft")
        window.act("Add reply")

        def selected_reply():
            current = manuscript(window)
            return current if window.text(current) != "The new scene begins." else None

        node = window.wait(selected_reply, "new reply selection")
        window.focus(node)
        key("a", control=True)
        type_text("Stay for a moment.")
        window.wait(lambda: window.text(node) == "Stay for a moment.", "reply draft")
        window.document_action("Apply draft")
        window.document_action("Read beat")
        window.wait(lambda: window.find_text("Stay for a moment."), "accepted reply reading")
        capture(output, "-written-beat")
        started = time.monotonic()
        window.reveal_start("Save")
        window.act("Save")
        window.wait(
            lambda: "Stay for a moment." in source.read_text(), "structured save", timeout=30
        )
        results["structured_save_seconds"] = time.monotonic() - started
        window.wait(lambda: window.find_text("Saved"), "save completion", timeout=30)
        assert source.read_text().startswith("# A new scene.")
        results[
            "structured_creation_applies_prose_before_leaving_and_preserves_unsaved_boundary"
        ] = True
        window.close_clean()
        assert process.wait(timeout=5) == 0
    except Exception:
        if process.poll() is None:
            capture(output, "-structure-error")
        raise
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=5)

    source.write_text(
        ":: start default\n:if trusts(player)\n"
        "  > first@77777777777777777777\n    A conditional line.\n"
        "! blocking overlay(work)\n-> END\n:: empty\n"
    )
    process, window, _ = launch(api, bus, registry, arguments, log)
    try:
        window.document_action("Read beat")
        window.wait(lambda: window.find_text(":if trusts(player)"), "conditional Script heading")
        window.wait(lambda: window.find_text("Effect ·", prefix=True), "read-only effect")
        capture(output, "-script")
        window.act("Empty")
        window.wait(
            lambda: window.find_text("No dialogue yet.", prefix=True), "empty beat inspection"
        )
        capture(output, "-empty-beat")
        window.act("Add line")
        node = window.wait(lambda: manuscript(window), "line in previously empty beat")
        window.wait(lambda: window.text(node) == "New dialogue.", "new line selection")
        window.document_action("Read beat")
        window.wait(lambda: window.find("Edit passage 1"), "new line reading")
        results["script_groups_effects_and_empty_beat_inspection"] = True
        window.document_action("Close")
        window.wait(lambda: window.find("Save all and close"), "structural close guard")
        window.save_and_close(results, "structured_save_close")
    except Exception:
        if process.poll() is None:
            capture(output, "-script-error")
        raise
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
