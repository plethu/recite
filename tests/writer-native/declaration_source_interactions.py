"""Explicit IDE repair of standalone declarations preserves the live manuscript."""

import json
import os
import subprocess
import tempfile
import traceback
from pathlib import Path

from native_window import attach, key, type_text
from picker_interactions import focused
from project_interactions import manuscript
from project_tools_interactions import command
from startup_interactions import fill
from writer_interactions import SOURCE, capture

DECLARATIONS = "schema_version=1\n[producer]\nid='dialogue'\n[speakers.mara]\ndisplay_name='Mara'\n"


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "declaration-source"}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-declaration-source-") as temporary:
            root = Path(temporary)
            project = root / "Project [owned] $literal"
            project.mkdir()
            (project / "recite.project.toml").write_text(
                "format_version=1\n[project]\nschema='schema.json'\n"
            )
            (project / "scene.recite").write_text(SOURCE)
            source = project / "schema.toml"
            source.write_text(DECLARATIONS)
            (project / "schema.json").write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "producer": {"kind": "standalone", "id": "dialogue"},
                        "speakers": {"mara": {"display_name": "Mara"}},
                    }
                )
            )
            recovered = "# Recovered author draft\nunfinished TOML ["
            source.with_name(source.name + ".recite-editor-recovery.json").write_text(
                json.dumps({"version": 1, "baseline": DECLARATIONS, "draft": recovered})
            )
            commands = root / "bin"
            commands.mkdir()
            receipt, release = root / "argv.json", root / "release"
            launcher = commands / "xdg-open"
            launcher.write_text(
                "#!/usr/bin/python3\nimport json,sys,time\nfrom pathlib import Path\n"
                f"Path({str(receipt)!r}).write_text(json.dumps(sys.argv[1:]))\n"
                f"while not Path({str(release)!r}).exists(): time.sleep(0.02)\n"
                f"Path(sys.argv[1]).write_text({DECLARATIONS.replace('Mara', 'Repaired by IDE')!r})\n"
            )
            launcher.chmod(0o755)
            environment = dict(os.environ, PATH=str(commands) + os.pathsep + os.environ["PATH"])
            with output.with_suffix(".writer.log").open("w") as log:
                process = subprocess.Popen(
                    [str(binary), "--project", str(project)],
                    env=environment,
                    stdout=log,
                    stderr=subprocess.STDOUT,
                )
                window = attach(api, bus, registry, process)
                node = manuscript(window)
                window.focus(node)
                key("End", control=True)
                type_text("x")
                command(window, "Declarations")
                window.act("Declaration source")
                fill(window, "Declaration source path", str(source))
                window.act("Bind standalone source")
                window.wait(
                    lambda: window.find_text("Source needs repair.", contains=True),
                    "recovered invalid TOML",
                )
                capture(output, "-recovered")
                window.act("Export declaration draft")
                window.wait(
                    lambda: list(project.glob(".recite-schema-repair-*.toml")), "editable artifact"
                )
                (artifact,) = project.glob(".recite-schema-repair-*.toml")
                assert artifact.read_text() == recovered
                assert source.read_text() == DECLARATIONS
                window.reveal_start("Open exported declaration draft in editor")
                window.wait(
                    lambda: window.enabled("Open exported declaration draft in editor"),
                    "exported artifact action",
                )
                window.act("Open exported declaration draft in editor")
                window.wait(receipt.exists, "owned IDE handoff")
                assert json.loads(receipt.read_text()) == [str(artifact)]
                release.touch()
                window.wait(
                    lambda: "Repaired by IDE" in artifact.read_text(), "IDE artifact repair"
                )
                assert source.read_text() == DECLARATIONS
                assert "Repaired by IDE" not in (project / "schema.json").read_text()
                results["recovered_invalid_draft_export_and_literal_ide_handoff"] = True
                window.reveal_end("Import repaired declaration draft")
                window.act("Import repaired declaration draft")
                window.reveal_start("Save and generate declarations")
                window.wait(
                    lambda: window.find_text(
                        "Save and generate to update the schema.", contains=True
                    ),
                    "explicit import validated",
                )
                assert source.read_text() == DECLARATIONS
                capture(output, "-imported")
                window.act("Save and generate declarations")
                window.wait(
                    lambda: "Repaired by IDE" in (project / "schema.json").read_text(),
                    "checked declaration publication",
                    timeout=30,
                )
                window.wait(
                    lambda: window.find_text("generated schema is current.", contains=True),
                    "refreshed source status",
                )
                assert "Repaired by IDE" in source.read_text()
                assert (project / "scene.recite").read_text() == SOURCE
                results["explicit_import_then_save_updates_source_and_output_only"] = True
                capture(output, "-ready")
                window.reveal_start("Return to writing")
                window.act("Return to writing")
                window.wait(lambda: focused(window, node), "retained manuscript focus")
                assert manuscript(window) == node
                assert window.text(node) == "The last tram waits.x"
                key("z", control=True)
                window.wait(
                    lambda: window.text(node) == "The last tram waits.", "retained manuscript undo"
                )
                results["declaration_work_preserves_native_manuscript_and_undo"] = True
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
    return "error" not in results
