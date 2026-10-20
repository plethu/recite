"""Producer execution and output publication remain owned after leaving its screen."""

import json
import tempfile
import traceback
from pathlib import Path

from native_window import key, type_text
from picker_interactions import focused
from project_interactions import manuscript
from project_tools_interactions import command
from writer_interactions import SOURCE, capture, launch


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "generation"}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-generation-") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text(
                "format_version=1\n[project]\nschema='schema.json'\n"
            )
            (project / "scene.recite").write_text(SOURCE)
            schema = {
                "schema_version": 1,
                "producer": {"kind": "standalone", "id": "dialogue"},
                "speakers": {"mara": {"display_name": "Mara"}},
            }
            (project / "schema.json").write_text(json.dumps(schema))
            schema["speakers"]["mara"]["display_name"] = "Updated Mara"
            (project / "candidate.json").write_text(json.dumps(schema))
            # The fixture explicitly admits completion so assertions do not race a fast producer.
            (project / "producer.sh").write_text(
                'while [ ! -f release ]; do sleep 0.02; done\ncp candidate.json "$1"\n'
            )
            (project / "recite.producer.toml").write_text(
                'version=1\n[producer]\nkind="standalone"\nid="dialogue"\n'
                '[generate]\nprogram="/bin/sh"\nargs=["producer.sh", "{output}"]\n'
            )
            with output.with_suffix(".writer.log").open("w") as log:
                process, window, node = launch(
                    api, bus, registry, [str(binary), "--project", str(project)], log
                )
                window.focus(node)
                key("End", control=True)
                type_text("x")
                command(window, "Declarations")
                window.act("Producer")
                window.wait(lambda: window.find("Regenerate schema"), "reviewed producer command")
                capture(output, "-producer-ready")
                window.act("Regenerate schema")
                window.wait(lambda: window.find("Cancel schema generation"), "active generation")
                window.act("Return to writing")
                window.wait(lambda: focused(window, node), "writing focus after producer")
                assert manuscript(window) == node
                type_text("y")
                (project / "release").touch()
                window.wait(
                    lambda: window.find_text("Schema published; project analysis refreshed."),
                    "hidden publication and schema refresh",
                    timeout=30,
                )
                assert "Updated Mara" in (project / "schema.json").read_text()
                assert window.text(node) == "The last tram waits.xy"
                key("z", control=True)
                window.wait(lambda: window.text(node) == "The last tram waits.x", "retained undo")
                results["hidden_publication_preserves_native_typing_and_undo"] = True
                command(window, "Declarations")
                window.act("Producer")
                (project / "release").unlink()
                window.act("Regenerate schema")
                window.wait(lambda: window.find("Cancel schema generation"), "cancel action")
                capture(output, "-producer-running")
                window.act("Cancel schema generation")
                window.wait(
                    lambda: not window.find("Cancel schema generation"),
                    "terminal cancellation",
                    timeout=30,
                )
                assert "Updated Mara" in (project / "schema.json").read_text()
                results["cancel_retirement_retains_published_output"] = True
                window.act("Return to writing")
                window.act("Close")
                window.wait(
                    lambda: window.find("Close and keep recovery"), "close after cancellation"
                )
                window.act("Close and keep recovery")
                assert process.wait(timeout=5) == 0
                results["cancel_retirement_allows_recovery_close"] = True
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
        if process is not None and process.poll() is None:
            capture(output, "-error")
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
    output.write_text(json.dumps(results, indent=2) + "\n")
    return "error" not in results
