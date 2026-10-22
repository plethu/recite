"""Live preferences preserve mounted fields and persist through the shared user store."""

import json
import os
import tempfile
import tomllib
import traceback
from pathlib import Path

from native_window import key, type_text
from preference_controls import reveal_preference
from project_interactions import manuscript
from startup_interactions import field, fill
from writer_interactions import SOURCE, capture, capture_settled, launch


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "preferences"}
    process, window = None, None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-preferences-") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text("format_version = 1\n")
            (project / "scene.recite").write_text(SOURCE)
            config = Path(os.environ["XDG_CONFIG_HOME"]) / "recite/config.toml"
            config.parent.mkdir(parents=True, exist_ok=True)
            config.write_text("# Preserve this personal comment.\nconfig_version = 1\n")
            arguments = [str(binary), "--project", str(project)]
            with output.with_suffix(".writer.log").open("w") as log:
                process, window, node = launch(api, bus, registry, arguments, log)
                window.focus(node)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(node) == "The last tram waits.x", "native draft")
                window.workspace_action("Settings")
                window.wait(lambda: window.find("Dark theme"), "personal preferences")
                capture_settled(window, output, "-light")
                window.act("Dark theme")
                window.wait(
                    lambda: (
                        tomllib.loads(config.read_text()).get("writer", {}).get("theme") == "dark"
                    ),
                    "durable dark theme",
                )
                dark = window.wait(lambda: window.find("Dark theme"), "updated theme controls")
                window.focus(dark)
                window.act("Increase UI scale")
                window.wait(lambda: window.find("UI scale: 110%"), "live UI scale")
                focused = 1 << int(api.Atspi.StateType.FOCUSED)
                assert window.call(dark, "Accessible", "GetState")[0][0] & focused
                results["live_theme_and_scale_preserve_native_radio_focus"] = True
                capture_settled(window, output, "-dark")
                for percent in range(120, 201, 10):
                    reveal_preference(window, "Increase UI scale")
                    window.act("Increase UI scale")
                    window.wait(lambda: window.find(f"UI scale: {percent}%"), "increased scale")
                capture_settled(window, output, "-scale200")
                reveal_preference(window, "Confirm before closing")
                capture_settled(window, output, "-behaviour200")
                results["ui_scale_reaches_validated_200_percent"] = True
                for percent in range(190, 99, -10):
                    reveal_preference(window, "Decrease UI scale")
                    window.act("Decrease UI scale")
                    window.wait(
                        lambda: (
                            tomllib.loads(config.read_text())
                            .get("writer", {})
                            .get("presentation", {})
                            .get("ui_scale", 100)
                            == percent
                        ),
                        "persisted restored scale",
                    )
                window.act("Larger reading text")
                window.wait(lambda: window.find("Reading text size: 21"), "reading size")
                window.act("Keyboard shortcuts")
                fill(window, "Shortcut binding", "Primary+Shift+S")
                window.act("Save shortcut")
                window.wait(
                    lambda: window.find_text("is assigned to both", contains=True),
                    "shortcut conflict feedback",
                )
                assert not tomllib.loads(config.read_text())["writer"].get("shortcuts", {})
                results["conflicting_shortcuts_leave_durable_preferences_unchanged"] = True
                fill(window, "Shortcut binding", "Primary+Alt+S")
                window.act("Save shortcut")
                window.wait(
                    lambda: (
                        tomllib.loads(config.read_text())
                        .get("writer", {})
                        .get("shortcuts", {})
                        .get("save")
                        == "Primary+Alt+S"
                    ),
                    "saved shortcut",
                )
                window.wait(
                    lambda: window.find_text("Shortcut saved for Save.", contains=True),
                    "command-specific saved feedback",
                )
                assert not window.enabled("Save shortcut")
                capture_settled(window, output, "-keyboard")
                fill(window, "Shortcut binding", "Primary+Alt+D")
                window.act("Save project · Primary+Shift+S")
                window.act("Save · Primary+Alt+S")
                window.wait(
                    lambda: window.text(field(window, "Shortcut binding")) == "Primary+Alt+D",
                    "retained shortcut draft",
                )
                window.act("Done with settings")
                window.workspace_action("Settings")
                window.wait(
                    lambda: window.text(field(window, "Shortcut binding")) == "Primary+Alt+D",
                    "shortcut draft after reopening settings",
                )
                results["shortcut_drafts_survive_command_changes_and_settings_reopening"] = True
                window.act("Done with settings")
                node = manuscript(window)
                window.focus(node)
                key("z", control=True)
                window.wait(
                    lambda: window.text(node) == "The last tram waits.",
                    "undo after live preferences",
                )
                key("y", control=True)
                window.wait(
                    lambda: window.text(node) == "The last tram waits.x",
                    "redo after live preferences",
                )
                capture_settled(window, output, "-writer")
                results["mounted_manuscript_history_survives_preference_changes"] = True
                assert config.read_text().startswith("# Preserve this personal comment.")
                results["shared_config_owner_preserves_user_comments"] = True
                window.document_action("Close")
                window.wait(lambda: window.find("Close and keep recovery"), "close guard")
                window.act("Close and keep recovery")
                assert process.wait(timeout=5) == 0
                process, window, node = launch(api, bus, registry, arguments, log)
                window.wait(
                    lambda: window.text(node) == "The last tram waits.x",
                    "recovered draft with persisted preferences",
                )
                window.workspace_action("Settings")
                window.wait(lambda: window.find("Reading text size: 21"), "reloaded preferences")
                assert tomllib.loads(config.read_text()).get("writer", {}).get("theme") == "dark"
                results["personal_preferences_and_recovery_survive_restart"] = True
                window.act("Done with settings")
                manuscript(window)
                window.document_action("Close")
                window.wait(lambda: window.find("Save all and close"), "save close choice")
                window.save_and_close(results, "preference_save_close")
                process, window, node = launch(api, bus, registry, arguments, log)
                window.document_action("Close")
                window.wait(lambda: window.find("Close Writer"), "ordinary close confirmation")
                capture_settled(window, output, "-ordinary-close")
                window.act("Keep editing")
                window.wait(lambda: not window.find("Close Writer"), "cancel ordinary close")
                window.workspace_action("Settings")
                window.wait(
                    lambda: window.find("Dark theme"), "personal preferences after cancelling close"
                )
                scrollbar = next(
                    address
                    for address, _ in window.nodes()
                    if window.call(address, "Accessible", "GetRole")[0] == 48
                )
                window.focus(scrollbar)
                key("End")
                window.wait(lambda: window.find("Confirm before closing"), "close preference")
                capture_settled(window, output, "-behaviour")
                window.act("Monochrome colours")
                window.wait(
                    lambda: tomllib.loads(config.read_text())["writer"].get("monochrome") is True,
                    "durable monochrome preference",
                )
                checked = 1 << int(api.Atspi.StateType.CHECKED)
                window.wait(
                    lambda: (
                        window.call(window.find("Monochrome colours"), "Accessible", "GetState")[0][
                            0
                        ]
                        & checked
                    ),
                    "native monochrome checked state",
                )
                capture_settled(window, output, "-monochrome")
                results["scrolling_exposes_registered_and_actionable_preferences"] = True
                window.act("Confirm before closing")
                window.wait(
                    lambda: (
                        tomllib.loads(config.read_text())["writer"].get("confirm_exit") is False
                    ),
                    "disabled ordinary confirmation",
                )
                window.act("Done with settings")
                manuscript(window)
                window.close_clean()
                assert process.wait(timeout=5) == 0
                results["ordinary_exit_confirmation_uses_the_shared_preference"] = True
    except Exception as error:
        results["error"] = repr(error)
        results["error_traceback"] = traceback.format_exc()
        if process is not None and process.poll() is None:
            capture(output, "-error")
            if window is not None:
                results["exposure_diagnostic"] = window.exposure_diagnostic()
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
    output.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(results, ensure_ascii=False))
    return "error" not in results
