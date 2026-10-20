"""Focus and Vim pane movement preserve native fields and reject cancelled sequences."""

import json
import os
import tempfile
import traceback
from pathlib import Path

from native_window import key, type_text
from project_interactions import manuscript
from writer_interactions import SOURCE, capture, launch


def focused(window, node):
    bit = 1 << int(window.api.Atspi.StateType.FOCUSED)
    return bool(window.call(node, "Accessible", "GetState")[0][0] & bit)


def prefix(window):
    key("w", control=True)
    window.wait(lambda: window.find_text("Pane navigation", prefix=True), "pane prefix hint")


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "focus", "physical_ime": False}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-focus-") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text("format_version = 1\n")
            source = project / "scene.recite"
            source.write_text(SOURCE)
            config = Path(os.environ["XDG_CONFIG_HOME"]) / "recite/config.toml"
            config.parent.mkdir(parents=True, exist_ok=True)
            config.write_text('config_version = 1\n[ui]\nkeymap = "vim"\n')
            with output.with_suffix(".writer.log").open("w") as log:
                process, window, node = launch(
                    api, bus, registry, [str(binary), "--project", str(project)], log
                )
                window.focus(node)
                key("End", control=True)
                type_text("x")
                window.wait(
                    lambda: window.text(node) == "The last tram waits.x", "field before focus"
                )
                key("f", control=True, shift=True)
                window.wait(lambda: window.find("Exit focus"), "writing focus")
                assert manuscript(window) == node
                assert not window.find("Project files")
                window.wait(lambda: focused(window, node), "native focused manuscript")
                key("z", control=True)
                window.wait(lambda: window.text(node) == "The last tram waits.", "retained undo")
                key("y", control=True)
                window.wait(lambda: window.text(node) == "The last tram waits.x", "retained redo")
                results["focus_stashes_navigation_and_preserves_native_undo_redo"] = True
                capture(output, "-writing")
                key("p", control=True)
                window.wait(lambda: window.find("Search query"), "nested focus lookup")
                key("Escape")
                window.wait(lambda: focused(window, node), "picker returns to focused field")
                assert window.find("Exit focus")
                results["nested_picker_restores_focus_without_leaving_writing_mode"] = True
                key("f", control=True, shift=True)
                window.wait(lambda: window.find("Project files"), "workspace restored")
                window.focus(node)
                prefix(window)
                capture(output, "-prefix")
                key("h")
                window.wait(lambda: not focused(window, node), "pane left")
                prefix(window)
                key("l")
                window.wait(lambda: focused(window, node), "pane right")
                results["vim_pane_movement_uses_native_focus"] = True
                prefix(window)
                key("h")
                previous = window.wait(
                    lambda: next(
                        (
                            address
                            for address, _ in window.nodes()
                            if focused(window, address) and address != node
                        ),
                        None,
                    ),
                    "pre-focus navigation target",
                )
                key("f", control=True, shift=True)
                window.wait(lambda: window.find("Exit focus"), "focus before window blur")
                auxiliary, other_window, _ = launch(
                    api, bus, registry, [str(binary), "--examples"], log
                )
                try:
                    other_window.close_clean()
                    assert auxiliary.wait(timeout=5) == 0
                finally:
                    if auxiliary.poll() is None:
                        auxiliary.terminate()
                        auxiliary.wait(timeout=5)
                window.wait(lambda: window.find("Exit focus"), "focused window resumed")
                scrollbar = window.wait(window._outer_scrollbar, "outer focus scrollbar")
                window.focus(scrollbar)
                key("End")
                key("f", control=True, shift=True)
                window.wait(
                    lambda: focused(window, previous), "revealed pre-focus navigation after blur"
                )
                results["window_blur_keeps_focus_return_target_and_restoration_reveals_it"] = True
                prefix(window)
                key("l")
                window.wait(lambda: focused(window, node), "return to manuscript after blur")
                prefix(window)
                key("Escape")
                window.wait(
                    lambda: not window.find_text("Pane navigation", prefix=True), "cancelled prefix"
                )
                assert window.text(node) == "The last tram waits.x"
                type_text("/")
                window.wait(
                    lambda: window.text(node) == "The last tram waits.x/", "literal slash in field"
                )
                key("z", control=True)
                window.wait(lambda: window.text(node) == "The last tram waits.x", "slash undo")
                prefix(window)
                key("h")
                window.wait(lambda: not focused(window, node), "navigation focus")
                type_text("/")
                window.wait(lambda: window.find("Search query"), "Vim normal search")
                key("Escape")
                window.wait(lambda: not window.find("Search query"), "normal lookup closed")
                window.wait(lambda: focused(window, previous), "normal lookup returns to navigation")
                results["cancelled_prefix_and_normal_text_keep_distinct_ownership"] = True
                key("F6")
                window.wait(lambda: focused(window, node), "standard next pane to manuscript")
                key("F6")
                window.wait(lambda: not focused(window, node), "standard next pane to preview")
                key("F6", shift=True)
                window.wait(lambda: focused(window, node), "standard previous pane to manuscript")
                results["standard_f6_cycles_and_reveals_native_panes"] = True
                assert source.read_text() == SOURCE
                window.reveal_start("Close")
                window.act("Close")
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
