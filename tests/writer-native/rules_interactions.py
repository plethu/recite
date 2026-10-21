"""Reply controls share checked drafts and preserve mounted native field history."""

import json
import tempfile
import traceback
from pathlib import Path

from native_window import key, type_text
from picker_interactions import focused
from startup_interactions import field, fill
from writer_interactions import capture, launch

SOURCE = """# Preserve comments and IDs.
:: start default
> first@11111111111111111111
  A gate stands open.
? reply@22222222222222222222 requires=(standing(3))
  Let me through.
  -> accepted
:: accepted
! immediate play_sfx(gate_latch)
# Keep this effect comment.
-> END
"""


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "rules"}
    process, window = None, None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-rules-") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text(
                'format_version = 1\n[project]\ncontent_set = "rules"\n'
                'version = "1"\nschema = "schema.json"\n'
            )
            (project / "schema.json").write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "registries": {"sounds": {"values": ["bell", "gate_latch"]}},
                        "conditions": {
                            "standing": {"params": [{"name": "minimum", "type": "int"}]},
                            "ready": {"params": []},
                        },
                        "effects": {
                            "play_sfx": {
                                "modes": ["immediate"],
                                "params": [{"name": "sound", "type": "registry:sounds"}],
                            }
                        },
                    }
                )
            )
            source = project / "scene.recite"
            source.write_text(SOURCE)
            with output.with_suffix(".writer.log").open("w") as log:
                process, window, _ = launch(
                    api, bus, registry, [str(binary), "--project", str(project)], log
                )
                window.act("Read beat")
                window.act("Reply rules…")
                minimum = "standing · minimum · Whole number"
                node = field(window, minimum)
                capture(output, "-controls")
                window.focus(node)
                key("End", control=True)
                type_text("0")
                window.wait(lambda: window.text(node) == "30", "staged numeric argument")
                window.act("Add condition…")
                query = field(window, "Search query")
                window.wait(lambda: focused(window, query), "rule choice query focus")
                key("Escape")
                assert field(window, minimum) == node
                window.focus(node)
                key("z", control=True)
                window.wait(lambda: window.text(node) == "3", "native undo after picker cancel")
                results["picker_cancel_preserves_mounted_field_and_native_undo"] = True
                fill(window, minimum, "7")
                key("Return", control=True)
                window.wait(lambda: window.find_text("Applied edit updated."), "refreshed controls")
                assert field(window, minimum) == node
                assert source.read_text() == SOURCE
                window.focus(node)
                key("End", control=True)
                type_text("x")
                key("z", control=True)
                window.wait(lambda: window.text(node) == "7", "same-field native undo after Apply")
                results["apply_reprojects_without_replacing_argument_field_or_saving"] = True
                fill(window, minimum, "-")
                key("s", control=True)
                window.wait(
                    lambda: window.find_text("needs a valid whole number value.", contains=True),
                    "invalid argument feedback",
                )
                capture(output, "-invalid")
                assert source.read_text() == SOURCE
                fill(window, minimum, "9")
                window.reveal_end("Choose value…")
                capture(output, "-lower-controls")
                window.act("Choose value…")
                query = field(window, "Search query")
                window.wait(lambda: focused(window, query), "registry query focus")
                type_text("bell")
                capture(output, "-registry")
                key("Return")
                sound = field(window, "play_sfx · sound · sounds")
                window.wait(lambda: window.text(sound) == "bell", "staged registry selection")
                key("s", control=True)
                window.wait(
                    lambda: (
                        "standing(9)" in source.read_text()
                        and "play_sfx(bell)" in source.read_text()
                    ),
                    "checked reply save",
                )
                assert "# Keep this effect comment." in source.read_text()
                assert "reply@22222222222222222222" in source.read_text()
                results[
                    "invalid_save_preserves_source_then_declared_choice_saves_with_comments_and_ids"
                ] = True
                window.act("Edit reply text")
                window.wait(lambda: window.find("Manuscript"), "reply text editor")
                results["rules_returns_to_editable_reply_text"] = True
                capture(output, "-reply")
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
