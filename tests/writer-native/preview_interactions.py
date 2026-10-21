"""Source-language runtime playback through an owned Writer window."""

import json
import tempfile
import traceback
from pathlib import Path

from native_window import key, type_text

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
        window.reveal_end("Start / restart preview")
        window.act("Start / restart preview")
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
        window.reveal_start("Manuscript")
        window.focus(node)
        original = window.text(node)
        key("End", control=True)
        type_text("x")
        window.reveal_end("Start / restart preview")
        window.wait(
            lambda: window.find_text("This preview uses older inputs.", prefix=True),
            "stale preview",
        )
        capture(output, "-stale")
        window.act("Start / restart preview")
        window.wait(
            lambda: window.find_text("Apply or discard", prefix=True), "unapplied preview guard"
        )
        results["preview_restart_preserves_unapplied_draft_and_previous_run"] = True
        window.reveal_start("Manuscript")
        window.focus(node)
        key("z", control=True)
        window.wait(lambda: window.text(node) == original, "typing undo through preview")
        window.reveal_end("Hide preview")
        window.act("Hide preview")
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
            capture(output, "-preview-error")
        raise
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
