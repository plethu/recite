"""Source-language runtime playback through an owned Writer window."""

import json
import tempfile
import traceback
from pathlib import Path

from localisation_interactions import reveal_text
from native_window import key, type_text
from preview_controls import diagnostic_focus, submission
from preview_controls import start as start_preview

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
        reveal_text(window, "Apply or discard", prefix=True)
        results["preview_restart_preserves_unapplied_draft_and_previous_run"] = True
        window.act("Review current inputs")
        window.wait(
            lambda: window.find("Preview preparation diagnostic"),
            "retained capture failure diagnostic",
        )
        window.wait(lambda: window.find("Hide preview error"), "explicit diagnostic disclosure")
        diagnostic_focus(window)
        results["short_diagnostic_preserves_the_disclosure_focus_anchor"] = True
        capture(output, "-failure-details")
        window.act("Hide preview error")
        window.act("Return to writing")
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
        window.close_clean()
        assert process.wait(timeout=5) == 0
    except Exception:
        if process.poll() is None:
            results["preview_visible_names_at_failure"] = [name for _, name in window.nodes()]
            focused = 1 << int(window.api.Atspi.StateType.FOCUSED)
            results["preview_focus_at_failure"] = []
            for target, name in window.nodes():
                state = window.call(target, "Accessible", "GetState")[0][0]
                if name == "Hide preview error" or state & focused:
                    results["preview_focus_at_failure"].append(
                        {
                            "name": name,
                            "role": window.call(target, "Accessible", "GetRole")[0],
                            "interfaces": window.call(target, "Accessible", "GetInterfaces")[0],
                            "focused": bool(state & focused),
                        }
                    )
            capture(output, "-preview-error")
        raise
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
