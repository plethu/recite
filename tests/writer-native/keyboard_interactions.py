"""Real native command chords coexist with field history and saved personal bindings."""

import json
import os
import tempfile
import time
import traceback
from pathlib import Path

from native_window import key, type_text
from project_interactions import manuscript
from startup_interactions import field, fill
from writer_interactions import SOURCE, capture, launch


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "keyboard"}
    process, window = None, None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-keyboard-") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text("format_version = 1\n")
            source = project / "scene.recite"
            source.write_text(SOURCE)
            with output.with_suffix(".writer.log").open("w") as log:
                process, window, node = launch(
                    api, bus, registry, [str(binary), "--project", str(project)], log
                )
                window.focus(node)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(node) == "The last tram waits.x", "typed field")
                key("Return", control=True)
                window.wait(
                    lambda: window.find_text("Applied edit updated.", contains=True),
                    "focused manuscript Apply chord",
                )
                key("s", control=True)
                window.wait(
                    lambda: "The last tram waits.x" in source.read_text(), "native Save command"
                )
                window.wait(
                    lambda: (
                        window.find_text("Saved to disk", prefix=True) and window.enabled("Save")
                    ),
                    "completed Save receipt before settings",
                )
                capture(output, "-applied")
                results["default_save_and_focused_apply_dispatch_without_another_event"] = True
                key("comma", control=True)
                window.wait(lambda: window.find("Keyboard shortcuts"), "native settings command")
                window.act("Keyboard shortcuts")
                fill(window, "Shortcut binding", "Primary+Alt+S")
                key("Return", control=True)
                time.sleep(0.1)
                assert "The last tram waits.x" in source.read_text()
                assert window.text(field(window, "Shortcut binding")) == "Primary+Alt+S"
                window.act("Save shortcut")
                window.wait(
                    lambda: window.find_text("Shortcut saved for Save.", contains=True),
                    "saved binding",
                )
                window.act("Done with settings")
                node = manuscript(window)
                window.focus(node)
                key("End", control=True)
                type_text("y")
                window.wait(
                    lambda: window.text(node) == "The last tram waits.xy", "draft after settings"
                )
                key("s", control=True)
                time.sleep(0.1)
                assert window.text(node) == "The last tram waits.xy"
                assert "The last tram waits.xy" not in source.read_text()
                key("Return", control=True)
                window.wait(
                    lambda: window.find_text("Applied edit updated.", contains=True),
                    "second Apply command",
                )
                key("s", control=True, alt=True)
                window.wait(
                    lambda: "The last tram waits.xy" in source.read_text(),
                    "live rebound Save command",
                )
                window.wait(
                    lambda: (
                        window.find_text("Saved to disk", prefix=True) and window.enabled("Save")
                    ),
                    "completed rebound Save receipt",
                )
                results["live_rebinding_removes_old_chord_without_inserting_its_character"] = True
                type_text("z")
                window.wait(
                    lambda: window.text(node) == "The last tram waits.xyz", "native history input"
                )
                key("z", control=True)
                window.wait(
                    lambda: window.text(node) == "The last tram waits.xy",
                    "native undo after commands",
                )
                key("y", control=True)
                window.wait(
                    lambda: window.text(node) == "The last tram waits.xyz",
                    "native redo after commands",
                )
                results["application_commands_preserve_native_undo_and_redo"] = True
                capture(output, "-writer")
                config = Path(os.environ["XDG_CONFIG_HOME"]) / "recite/config.toml"
                assert 'save = "Primary+Alt+S"' in config.read_text()
                window.document_action("Close")
                window.wait(lambda: window.find("Close and keep recovery"), "guarded close")
                window.act("Close and keep recovery")
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
    print(json.dumps(results))
    return "error" not in results
