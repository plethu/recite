"""Checked reply and continuation changes through native search and applied history."""

import json
import tempfile
import traceback
from pathlib import Path

from native_window import key, type_text
from picker_interactions import focused
from startup_interactions import field, fill
from writer_interactions import capture, launch

SOURCE = """# Preserve comments and line IDs.
:: start default
> first@11111111111111111111
  A gate stands open.
? reply@22222222222222222222
  Let me through.
  -> accepted
:: accepted
> entered@33333333333333333333
  The gate opens.
-> END
:: alternative
> wait@44444444444444444444
  Wait here.
-> END
"""


def choose(window, query_text):
    query = field(window, "Search query")
    window.wait(lambda: focused(window, query), "destination query focus")
    type_text(query_text)
    window.wait(lambda: window.text(query) == query_text, "destination query entered")
    key("Return")


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "routes"}
    process, window = None, None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-routes-") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text("format_version = 1\n")
            source = project / "scene.recite"
            source.write_text(SOURCE)
            with output.with_suffix(".writer.log").open("w") as log:
                process, window, _ = launch(
                    api, bus, registry, [str(binary), "--project", str(project)], log
                )
                window.act("Read beat")
                window.act("Edit passage 2")
                fill(window, "Manuscript", "A revised reply.")
                window.act("Change reply destination…")
                query = field(window, "Search query")
                window.wait(lambda: focused(window, query), "destination picker rendered")
                capture(output, "-picker")
                choose(window, "Alternative")
                window.wait(
                    lambda: window.find_text("Structural edit updated.", contains=True),
                    "reply route applied",
                )
                assert source.read_text() == SOURCE
                window.reveal_end("Undo applied")
                window.act("Undo applied")
                window.wait(lambda: window.find_text("Applied edit updated."), "route undo")
                window.act("Read beat")
                window.wait(lambda: window.find_text("Reply → accepted"), "original route restored")
                window.wait(
                    lambda: window.find_text("A revised reply."), "preceding prose still accepted"
                )
                results["reply_route_undo_preserves_preceding_prose_step"] = True
                window.reveal_end("Redo applied")
                window.act("Redo applied")
                window.wait(lambda: window.find_text("Reply → alternative"), "route redo")
                capture(output, "-reply")
                key("s", control=True)
                window.wait(
                    lambda: (
                        "A revised reply." in source.read_text()
                        and "-> alternative" in source.read_text()
                    ),
                    "checked reply save",
                )
                assert "reply@22222222222222222222" in source.read_text()
                results["route_requires_explicit_save_and_preserves_ids"] = True
                window.act("Beat · alternative · END")
                window.act("Change continuation…")
                choose(window, "Accepted")
                window.wait(lambda: window.find_text("Continue → accepted"), "changed continuation")
                capture(output, "-continuation")
                key("s", control=True)
                window.wait(
                    lambda: "-> accepted" in source.read_text().split(":: alternative", 1)[1],
                    "continuation saved",
                )
                results["unconditional_continuation_uses_shared_destination_edit"] = True
                window.close_clean()
                assert process.wait(timeout=5) == 0
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
        if process is not None and process.poll() is None:
            capture(output, "-error")
            results["visible_names"] = [name for _, name in window.nodes()]
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
    output.write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results))
    return "error" not in results
