"""Explicit project-settings IDE review preserves manuscript identity and history."""

import json
import os
import subprocess
import tempfile
import traceback
from pathlib import Path

from native_window import attach, key, type_text
from picker_interactions import focused
from project_interactions import manuscript
from writer_interactions import SOURCE, capture


def settings(window):
    window.workspace_action("Settings")
    window.act("Project settings…")
    window.wait(lambda: window.find_text("Project settings"), "project settings workspace")


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "project-settings"}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-project-settings-") as temporary:
            root = Path(temporary)
            project = root / "Project [owned] $literal"
            project.mkdir()
            manifest = project / "recite.project.toml"
            original = "format_version=1\n[project]\ncontent_set='original'\n"
            manifest.write_text(original)
            source = project / "scene.recite"
            source.write_text(SOURCE)
            commands = root / "bin"
            commands.mkdir()
            receipt = root / "argv.json"
            launcher = commands / "xdg-open"
            launcher.write_text(
                "#!/usr/bin/python3\nimport json,sys\nfrom pathlib import Path\n"
                f"Path({str(receipt)!r}).write_text(json.dumps(sys.argv[1:]))\n"
                "path=Path(sys.argv[1])\n"
                "path.write_text(path.read_text().replace('original','IDE reviewed'))\n"
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
                settings(window)
                window.act("Export settings draft")
                window.wait(
                    lambda: list(project.glob(".recite-project-settings-*.toml")),
                    "settings artifact export",
                )
                (artifact,) = project.glob(".recite-project-settings-*.toml")
                assert artifact.read_text() == original
                window.act("Open settings draft in editor")
                window.wait(receipt.exists, "owned editor handoff")
                assert json.loads(receipt.read_text()) == [str(artifact)]
                window.wait(lambda: "IDE reviewed" in artifact.read_text(), "IDE edit")
                window.wait(
                    lambda: window.find_text("Choose Import settings draft", contains=True),
                    "explicit import guidance",
                )
                assert manifest.read_text() == original
                results["literal_ide_artifact_handoff_requires_explicit_import"] = True
                window.reveal_end("Import settings draft")
                window.act("Import settings draft")
                window.wait(
                    lambda: window.find_text("Settings draft imported. Ready to Apply."),
                    "validated settings review",
                )
                assert manifest.read_text() == original
                window.reveal_start("Apply reviewed settings")
                capture(output, "-review")
                window.act("Apply reviewed settings")
                window.wait(
                    lambda: window.find_text(
                        "Project settings saved; project analysis refreshed.", contains=True
                    ),
                    "published settings and refreshed context",
                )
                assert manifest.read_text() == artifact.read_text()
                assert source.read_text() == SOURCE
                capture(output, "-ready")
                results[
                    "explicit_apply_updates_manifest_and_refreshes_without_saving_manuscript"
                ] = True
                window.reveal_start("Return to writing")
                window.act("Return to writing")
                window.wait(lambda: focused(window, node), "retained manuscript focus")
                assert manuscript(window) == node
                assert window.text(node) == "The last tram waits.x"
                key("z", control=True)
                window.wait(
                    lambda: window.text(node) == "The last tram waits.", "retained native undo"
                )
                results["same_native_manuscript_and_undo_after_settings"] = True
                settings(window)
                artifact.write_text(manifest.read_text().replace("IDE reviewed", "candidate"))
                window.reveal_end("Import settings draft")
                window.act("Import settings draft")
                window.wait(
                    lambda: window.find_text("Settings draft imported. Ready to Apply."),
                    "second settings review",
                )
                external = manifest.read_text().replace("IDE reviewed", "external")
                manifest.write_text(external)
                window.reveal_start("Apply reviewed settings")
                window.act("Apply reviewed settings")
                window.wait(
                    lambda: window.find_text("Project settings changed on disk.", contains=True),
                    "original baseline conflict",
                )
                assert manifest.read_text() == external
                assert "candidate" in artifact.read_text()
                capture(output, "-conflict")
                results["external_conflict_preserves_manifest_and_review_artifact"] = True
                window.act("Reload saved settings")
                window.wait(
                    lambda: window.find_text(
                        "Project settings reloaded; project analysis refreshed.", contains=True
                    ),
                    "explicit baseline reload and refresh",
                )
                artifact.write_text(external + "\n[discovery]\nexcludes=['scene.recite']\n")
                window.reveal_end("Import settings draft")
                window.act("Import settings draft")
                window.wait(
                    lambda: window.find_text("remove an open document", contains=True),
                    "open document membership protection",
                )
                assert manifest.read_text() == external
                results["settings_cannot_exclude_open_manuscript"] = True
                window.close_clean()
                assert process.wait(timeout=5) == 0
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
    output.write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results, indent=2))
    return "error" not in results
