"""Source-language runtime playback through an owned Writer window."""

import json
import tempfile
import traceback
from pathlib import Path

from native_window import key, type_text
from picker_interactions import focused
from preview_controls import start as start_preview
from preview_controls import submission

SOURCE = """:: start default
> line@11111111111111111111
  Hello.
? go@22222222222222222222
  Choose the branch
  -> branch
:: branch
:if trusts(player)
  > yes@55555555555555555555
    Yes.
:else
  > no@66666666666666666666
    No.
! blocking overlay(work)
-> END
"""


def run(api, bus, registry, binary, output):
    from writer_interactions import capture, launch

    results = {"backend": "owned Wayland", "scenario": "preview"}
    try:
        with tempfile.TemporaryDirectory(prefix="recite-preview-") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text("format_version = 1\n")
            with output.with_suffix(".writer.log").open("w") as log:
                exercise(
                    api,
                    bus,
                    registry,
                    [str(binary), "--project", str(project)],
                    log,
                    project / "scene.recite",
                    output,
                    results,
                    launch,
                    capture,
                )
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
    output.write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results))
    return "error" not in results


def exercise(api, bus, registry, arguments, log, source, output, results, launch, capture):
    source.write_text(SOURCE)
    process, window, node = launch(api, bus, registry, arguments, log)
    try:
        window.document_action("Configure preview")
        submission(window)
        start_preview(window)
        window.reveal_end("Continue preview")
        window.act("Continue preview")
        window.reveal_end("Choose the branch")
        window.act("Choose the branch")
        window.reveal_end("Answer true")
        capture(output, "-condition")
        window.act("Answer true")
        window.reveal_end("Continue preview")
        window.wait(lambda: window.find_text("Yes."), "condition result")
        window.act("Continue preview")
        window.reveal_end("Acknowledge effect")
        window.wait(lambda: window.find_text("Effect requested"), "blocking effect request")
        capture(output, "-effect")
        window.act("Acknowledge effect")
        window.reveal_end("Continue preview")
        window.act("Continue preview")
        window.wait(lambda: window.find("Conversation ended"), "conversation ending")
        results["preview_choices_conditions_and_effect_ack"] = True
        window.act("Return to writing")
        window.reveal_start("Manuscript")
        node = window.wait(lambda: window.find("Manuscript"), "returned manuscript")
        window.focus(node)
        original = window.text(node)
        key("End", control=True)
        type_text("x")
        window.document_action("Configure preview")
        submission(window)
        window.reveal_down("This preview uses older inputs. Restart to use your accepted edits.")
        window.wait(
            lambda: window.find_text("This preview uses older inputs.", prefix=True),
            "stale preview",
        )
        capture(output, "-stale")
        submission(window)
        start_preview(window)
        window.wait(
            lambda: window.find("Apply edit and preview"), "draft-specific preview recovery"
        )
        assert window.find("Review current inputs") is None
        assert window.find("Find preview language") is None
        window.wait(
            lambda: focused(window, window.find("Return to unfinished edit")),
            "unfinished-edit recovery focus",
        )
        capture(output, "-unfinished-edit")
        results["preview_restart_preserves_unapplied_draft_and_previous_run"] = True
        window.act("Return to unfinished edit")
        window.reveal_start("Manuscript")
        window.focus(node)
        key("z", control=True)
        window.wait(lambda: window.text(node) == original, "typing undo through preview")
        window.document_action("Configure preview")
        window.act("Return to writing")
        window.reveal_start("Manuscript")
        window.focus(node)
        key("y", control=True)
        window.wait(lambda: window.text(node) == original + "x", "typing redo after preview")
        key("z", control=True)
        window.wait(lambda: window.text(node) == original, "restored manuscript")
        results["preview_preserves_native_typing_history"] = True

        key("End", control=True)
        type_text(" accepted")
        window.wait(lambda: window.text(node) == original + " accepted", "draft for explicit Apply")
        window.act("Try scene")
        window.wait(lambda: window.find("Apply edit and preview"), "explicit apply admission")
        window.act("Apply edit and preview")
        window.wait(
            lambda: window.find_text(original + " accepted"),
            "preview includes the checked manuscript edit",
        )
        window.wait(
            lambda: window.find_text("Applying edit before preview", prefix=True) is None,
            "terminal preview status",
        )
        assert source.read_text() == SOURCE
        capture(output, "-applied-preview")
        window.act("Return to writing")
        window.reveal_start("Manuscript")
        assert window.find("Manuscript") == node
        window.focus(node)
        key("z", control=True)
        window.wait(
            lambda: window.text(node) != original + " accepted",
            "native Undo after checked Apply and preview",
        )
        key("y", control=True)
        window.wait(
            lambda: window.text(node) == original + " accepted",
            "native Redo after checked Apply and preview",
        )
        results["explicit_apply_preview_keeps_disk_and_native_history"] = True

        key("a", control=True)
        type_text("-> syntax")
        window.wait(lambda: window.text(node) == "-> syntax", "invalid manuscript draft")
        window.act("Try scene")
        window.wait(lambda: window.find("Apply edit and preview"), "invalid edit admission")
        window.act("Apply edit and preview")
        window.wait(
            lambda: window.find("Can't preview this edit yet"), "local failed-apply recovery"
        )
        assert window.find("Manuscript") == node
        assert window.text(node) == "-> syntax"
        assert source.read_text() == SOURCE
        capture(output, "-invalid-edit")
        window.act("Show reason")
        window.wait(lambda: window.find("Hide reason"), "expanded invalid-edit reason")
        window.wait(
            lambda: window.find_text("your editor", contains=True), "actual diagnostic text"
        )
        capture(output, "-invalid-edit-reason")
        window.act("Edit text")
        window.focus(node)
        key("a", control=True)
        type_text(original + " accepted")
        window.wait(lambda: window.text(node) == original + " accepted", "repaired draft")
        results["invalid_apply_returns_to_exact_edit_and_keeps_disk"] = True
        window.act("Save")
        window.wait(
            lambda: original + " accepted" in source.read_text(),
            "explicit Save after preview recovery",
        )
        window.wait(lambda: window.enabled("Save"), "completed recovery Save")
        window.close_clean()
        assert process.wait(timeout=5) == 0
    except Exception:
        if process.poll() is None:
            results["preview_visible_names_at_failure"] = [name for _, name in window.nodes()]
            focused_bit = 1 << int(window.api.Atspi.StateType.FOCUSED)
            results["preview_focus_at_failure"] = []
            for target, name in window.nodes():
                state = window.call(target, "Accessible", "GetState")[0][0]
                if name == "Hide preview error" or state & focused_bit:
                    results["preview_focus_at_failure"].append(
                        {
                            "name": name,
                            "role": window.call(target, "Accessible", "GetRole")[0],
                            "interfaces": window.call(target, "Accessible", "GetInterfaces")[0],
                            "focused": bool(state & focused_bit),
                        }
                    )
            capture(output, "-preview-error")
        raise
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
