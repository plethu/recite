"""Source-language runtime playback through an owned Writer window."""

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


def exercise(api, bus, registry, arguments, log, source, output, results, launch, capture):
    source.write_text(SOURCE)
    process, window, node = launch(api, bus, registry, arguments, log)
    try:
        window.act("Start / restart preview")
        window.wait(lambda: window.find("Continue preview"), "prepared runtime line")
        window.act("Continue preview")
        window.wait(lambda: window.find("Choose the branch"), "runtime choices")
        window.act("Choose the branch")
        window.wait(lambda: window.find("Answer true"), "typed condition request")
        capture(output, "-condition")
        window.act("Answer true")
        window.wait(lambda: window.find_text("Yes."), "condition result")
        window.act("Continue preview")
        window.wait(lambda: window.find("Acknowledge effect"), "blocking effect request")
        capture(output, "-effect")
        window.act("Acknowledge effect")
        window.wait(lambda: window.find("Continue preview"), "acknowledged effect")
        window.act("Continue preview")
        window.wait(lambda: window.find("Conversation ended"), "conversation ending")
        results["preview_choices_conditions_and_effect_ack"] = True
        window.focus(node)
        original = window.text(node)
        key("End", control=True)
        type_text("x")
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
        window.focus(node)
        key("z", control=True)
        window.wait(lambda: window.text(node) == original, "typing undo through preview")
        window.act("Hide preview")
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
            capture(output, "-preview-error")
        raise
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
