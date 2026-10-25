"""Native project-tool inspection and saved/save-all build lifecycles."""

import json
import tempfile
import traceback
from pathlib import Path

from native_window import key, type_text
from picker_interactions import focused
from project_interactions import manuscript
from startup_interactions import field
from writer_interactions import SOURCE, capture, launch


def command(window, query):
    key("p", control=True, shift=True)
    node = field(window, "Search query")
    window.wait(lambda: focused(window, node), "command query focus")
    type_text(query)
    key("Return")
    window.wait(lambda: not window.find("Search query"), "project command accepted")


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "project-tools"}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-project-tools-") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text(
                "format_version=1\n[project]\ncontent_set='trial'\nversion='1'\nschema='schema.json'\n"
                "[[scenes]]\nid='opening'\nasset='build/scene.recitec'\nblock='opening'\nparticipants=['mara']\n"
            )
            speakers = {"mara": {"display_name": "Mara"}}
            speakers.update(
                {
                    f"speaker_{index:04}": {"display_name": f"Announcer {index}"}
                    for index in range(1026)
                }
            )
            (project / "schema.json").write_text(
                json.dumps({"schema_version": 1, "speakers": speakers})
            )
            source = project / "scene.recite"
            original = SOURCE + "".join(
                f":: spare_{index}\n> spare_{index}@{index + 1000:020x}\n  Another line.\n-> END\n"
                for index in range(8192)
            )
            source.write_text(original)
            with output.with_suffix(".writer.log").open("w") as log:
                process, window, node = launch(
                    api, bus, registry, [str(binary), "--project", str(project)], log
                )
                window.focus(node)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(node) == "The last tram waits.x", "retained draft")
                command(window, "Declarations")
                query = field(window, "Search declarations")
                window.wait(
                    lambda: focused(window, query), "declaration query receives native focus"
                )
                type_text("speaker_0555")
                window.wait(
                    lambda: window.find_text("Showing 1–1 of 1 declaration"),
                    "filtered declaration",
                )
                window.wait(
                    lambda: window.find_text("Announcer 555"), "selected declaration detail"
                )
                assert not window.find("Previous declarations") and not window.find(
                    "Next declarations"
                )
                assert not window.find("Previous declaration detail") and not window.find(
                    "More declaration detail"
                )
                results["single_page_declaration_has_no_inactive_pagers"] = True
                capture(output, "-declarations")
                results["declaration_query_focus_and_selected_detail"] = True
                window.act("Return to writing")
                window.wait(lambda: focused(window, node), "writing focus restored")
                assert manuscript(window) == node
                key("z", control=True)
                window.wait(
                    lambda: window.text(node) == "The last tram waits.",
                    "native undo after project tools",
                )
                key("y", control=True)
                window.wait(
                    lambda: window.text(node) == "The last tram waits.x",
                    "native redo after project tools",
                )
                results["project_tools_preserve_native_field_identity_and_undo"] = True
                command(window, "Build scenes")
                window.wait(lambda: window.find("Build saved"), "build workspace")
                window.wait(
                    lambda: window.find_text(
                        "recite.project.toml · 1 source file · 1 output target"
                    ),
                    "singular saved build input and target summary",
                )
                capture(output, "-build")
                window.act("Build saved")
                window.act("Return to writing")
                asset = project / "build/scene.recitec"
                window.wait(lambda: asset.is_file(), "saved-input output", timeout=30)
                window.wait(
                    lambda: not window.find("Cancel build"),
                    "hidden build terminal result",
                    timeout=30,
                )
                assert source.read_text() == original
                before = asset.read_bytes()
                assert window.text(manuscript(window)) == "The last tram waits.x"
                results["build_saved_preserves_unapplied_draft_and_completes_after_return"] = True
                command(window, "Build scenes")
                window.wait(lambda: window.find("Save all and build"), "save-build action")
                window.act("Save all and build")
                window.wait(
                    lambda: "The last tram waits.x" in source.read_text(),
                    "ordered save before build",
                    timeout=30,
                )
                window.wait(
                    lambda: asset.read_bytes() != before, "updated build output", timeout=30
                )
                window.wait(
                    lambda: not window.find("Cancel build"),
                    "save-build terminal result",
                    timeout=30,
                )
                results["save_all_and_build_publishes_saved_changes"] = True
                assert not window.find("Previous build report") and not window.find(
                    "More build report"
                )
                results["single_page_build_result_has_no_inactive_pager"] = True
                capture(output, "-built")
                before_cancel = asset.read_bytes()
                source.write_text(
                    source.read_text().replace("The last tram waits.x", "Fresh build inputs.")
                )
                window.act("Build saved")

                def cancel_or_complete():
                    cancel = window.find_action("Cancel build")
                    if cancel is not None:
                        return window.call(
                            cancel, "Action", "DoAction", api.GLib.Variant("(i)", (0,))
                        )[0]
                    return asset.read_bytes() != before_cancel

                # A terminal result may retire the control between observation and Click.
                # wait() retries stale AT-SPI nodes; completion is a distinct outcome below.
                window.wait(cancel_or_complete, "cancel request or completed build")
                window.wait(
                    lambda: not window.find("Cancel build"),
                    "cancelled build retirement",
                    timeout=30,
                )
                cancelled = window.find_text("Build cancelled. Previous outputs remain available.")
                results["native_cancellation_observed"] = cancelled is not None
                if cancelled is not None:
                    assert asset.read_bytes() == before_cancel
                else:
                    assert asset.read_bytes() != before_cancel
                results["build_terminal_result_harvested_before_close"] = True
                window.close_clean()
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
