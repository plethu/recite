"""File-backed migration checks using only an owned project, editor and native window."""

import json
import subprocess
import tempfile
import traceback
from collections import Counter
from pathlib import Path

from native_window import attach, key, type_text
from project_interactions import exercise as project_exercise
from project_interactions import switch

SOURCE = """:: opening default
> first@11111111111111111111
  The last tram waits.
> second@22222222222222222222
  A second paragraph.
-> END
"""


def launch(api, bus, registry, arguments, log, manuscript=True):
    process = subprocess.Popen(arguments, stdout=log, stderr=subprocess.STDOUT)

    window = attach(api, bus, registry, process)
    if not manuscript:
        window.wait(lambda: window.find("Open in editor"), "source repair controls")
        return process, window, None
    node = window.wait(
        lambda: next(
            (
                address
                for address, name in window.nodes()
                if name == "Manuscript" and window.call(address, "Accessible", "GetRole")[0] == 79
            ),
            None,
        ),
        "native manuscript Text",
    )
    return process, window, node


def capture(output, suffix=""):
    path = output.with_name(output.stem + suffix).with_suffix(".png")
    subprocess.run(["grim", str(path)], check=True, timeout=10)


def focus_close_choice(window):
    focused = 1 << int(window.api.Atspi.StateType.FOCUSED)
    # The outer portal's scrollbar can precede the choices in the native Tab cycle.
    for steps in range(1, 4):
        key("Tab")
        try:
            window.api.wait_for(
                lambda: (
                    window.call(window.find("Keep editing"), "Accessible", "GetState")[0][0]
                    & focused
                ),
                "keyboard focus on Keep editing",
                window.process,
                timeout=0.5,
            )
            return steps
        except RuntimeError:
            if window.process.poll() is not None:
                raise
    raise RuntimeError("Keep editing was not reachable within three Tab presses")


def run(api, bus, registry, binary, output):
    results = {"host": "owned Cage, D-Bus, project and editor", "physical_ime": False}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-file-writer-") as temporary:
            project = Path(temporary) / "Project [owned] $test"
            project.mkdir()
            source = project / "scene.recite"
            source.write_text(SOURCE)
            other = project / "zz-other.recite"
            other.write_text(
                SOURCE.replace(":: opening default", ":: other")
                .replace("11111111111111111111", "33333333333333333333")
                .replace("22222222222222222222", "44444444444444444444")
                .replace("The last tram waits.", "An unapplied Writer draft.")
            )
            (project / "recite.project.toml").write_text(
                'format_version = 1\n[project]\ncontent_set = "trial"\nversion = "1"\n'
            )
            editor = Path(temporary) / "owned_editor.py"
            receipt = Path(temporary) / "editor-argv.json"
            editor.write_text(
                "import json,sys\nfrom pathlib import Path\n"
                "path = Path(sys.argv[2])\n"
                "Path(sys.argv[1]).write_text(json.dumps(sys.argv[2:]))\n"
                "path.write_text(path.read_text() + '\\n# Owned external edit\\n')\n"
            )
            arguments = [
                str(binary),
                "--project",
                str(project),
                "--editor",
                "/usr/bin/python3",
                "--editor-arg",
                str(editor),
                "--editor-arg",
                str(receipt),
            ]
            with output.with_suffix(".log").open("w") as log:
                process, window, node = launch(api, bus, registry, arguments, log)
                assert api.property_value(bus, node, api.ACCESSIBLE, "Name") == "Manuscript"
                results["manuscript_accessible_name"] = True
                window.focus(node)
                original = window.text(node)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(node) == original + "x", "typing")
                key("z", control=True)
                window.wait(lambda: window.text(node) == original, "native undo")
                key("y", control=True)
                window.wait(lambda: window.text(node) == original + "x", "native redo")
                results["typing_undo_redo"] = True
                key("a", control=True)
                key("c", control=True)
                window.wait(
                    lambda: (
                        subprocess.run(
                            ["wl-paste", "--no-newline"], capture_output=True, text=True
                        ).stdout
                        == original + "x"
                    ),
                    "copy out",
                )
                results["copy_out"] = True
                incoming = "Saved Café 日本語 مرحبا"
                subprocess.run(["wl-copy", "--", incoming], check=True, timeout=3)
                key("v", control=True)
                window.wait(lambda: window.text(node) == incoming, "Unicode paste")
                window.act("Save file")
                window.wait(lambda: incoming in source.read_text(), "checked native save")
                results["unicode_paste_and_save"] = True
                key("a", control=True)
                type_text("An unapplied Writer draft.")
                window.wait(
                    lambda: window.text(node) == "An unapplied Writer draft.", "unapplied draft"
                )
                node = project_exercise(window, "An unapplied Writer draft.", results)
                capture(output, "-documents")
                window.act("Open in editor")
                window.wait(
                    lambda: "# Owned external edit" in source.read_text(), "owned editor completion"
                )
                assert json.loads(receipt.read_text()) == [str(source)]
                assert "An unapplied Writer draft." not in source.read_text()
                results["editor_argv_and_unapplied_draft_preserved"] = True
                window.act("Reload (keep recovery copy)")
                window.wait(lambda: window.text(node) == incoming, "explicit reload")
                copies = list(project.glob(".recite-recovered-*.json"))
                assert len(copies) == 1
                assert (
                    json.loads(copies[0].read_text())["draft"]["draft"]
                    == "An unapplied Writer draft."
                )
                results["reload_keeps_independent_recovery"] = True
                capture(output)
                window.focus(node)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(node) == incoming + "x", "closing draft")
                save_control = window.find("Save file")
                assert window.enabled("Save file")
                saved_before_close = source.read_text()
                window.act("Close")
                window.wait(lambda: window.find("Close and keep recovery"), "close guard")
                assert window.find("Save file") == save_control
                assert not window.enabled("Save file")
                window.act("Save file")
                results["close_choice_tab_steps"] = focus_close_choice(window)
                assert source.read_text() == saved_before_close
                assert window.text(node) == incoming + "x"
                results["disabled_native_save_rejects_activation"] = True
                results["close_choice_keyboard_focus"] = True
                key("Return")
                window.wait(lambda: not window.find("Keep editing"), "cancel closing")
                window.wait(lambda: window.enabled("Save file"), "save enabled after cancel")
                assert window.find("Save file") == save_control
                results["native_enabled_sensitive_transition_keeps_control_identity"] = True
                window.focus(node)
                key("z", control=True)
                window.wait(
                    lambda: window.text(node) == incoming, "undo retained after cancelling close"
                )
                key("y", control=True)
                window.wait(
                    lambda: window.text(node) == incoming + "x",
                    "redo retained after cancelling close",
                )
                results["cancel_close_retains_text_history"] = True
                window.act("Close")
                window.wait(lambda: window.find("Keep editing"), "close again")
                focus_close_choice(window)
                capture(output, "-close")
                window.act("Close and keep recovery")
                assert process.wait(timeout=5) == 0
                process, window, node = launch(api, bus, registry, arguments, log)
                window.wait(lambda: window.text(node) == incoming + "x", "restored draft")
                node = switch(window, "zz-other.recite", "An unapplied Writer draft.x")
                node = switch(window, "scene.recite · Unsaved", incoming + "x")
                results["inactive_document_recovery_survives_restart"] = True
                results["close_guard_and_reopen_recovery"] = True
                capture(output, "-recovered")
                window.act("Close")
                window.wait(lambda: window.find("Save all and close"), "save-close guard")
                window.save_and_close(results, "recovered_save_close")
                assert incoming + "x" in source.read_text()
                assert not source.with_name("scene.recite.recite-editor-recovery.json").exists()
                assert "An unapplied Writer draft.x" in other.read_text()
                assert not other.with_name("zz-other.recite.recite-editor-recovery.json").exists()
                results["save_and_close_clears_recovery"] = True
                results["save_and_close_covers_inactive_files"] = True
                malformed = ":: scene default\n->\n"
                source.write_text(malformed)
                process, window, _ = launch(api, bus, registry, arguments, log, manuscript=False)
                assert not any(
                    window.call(address, "Accessible", "GetRole")[0] == 79
                    for address, _ in window.nodes()
                )
                assert source.read_text() == malformed
                assert not window.enabled("Add beat")
                assert not window.enabled("Start / restart preview")
                capture(output, "-source")
                results["malformed_source_preserved_for_external_repair"] = True
                window.close_clean()
                assert process.wait(timeout=5) == 0
                from preview_interactions import exercise as preview_exercise

                preview_exercise(
                    api, bus, registry, arguments, log, source, output, results, launch, capture
                )
                from structure_interactions import exercise as structure_exercise

                structure_exercise(
                    api, bus, registry, arguments, log, source, output, results, launch, capture
                )
    except Exception as error:
        results["error"] = repr(error)
        results["error_traceback"] = traceback.format_exc()
        if process is not None and process.poll() is None:
            waits = Counter()
            for task in Path(f"/proc/{process.pid}/task").iterdir():
                try:
                    waits[
                        (
                            task.joinpath("comm").read_text().strip(),
                            task.joinpath("wchan").read_text().strip(),
                        )
                    ] += 1
                except OSError:
                    pass
            results["owned_process_thread_waits"] = [
                [name, wait, count] for (name, wait), count in waits.items()
            ]
            focus_bit = 1 << int(api.Atspi.StateType.FOCUSED)
            results["focused_nodes_at_failure"] = [
                name
                for address, name in window.nodes()
                if window.call(address, "Accessible", "GetState")[0][0] & focus_bit
            ]
            capture(output, "-error")
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
    output.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(results, ensure_ascii=False))
    return "error" not in results
