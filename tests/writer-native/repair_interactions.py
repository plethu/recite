"""Recovered Source drafts use an owned IDE artifact and checked import before Save."""

import json
import subprocess
import tempfile
import traceback
from pathlib import Path

from native_window import attach
from startup_interactions import field, fill
from writer_interactions import SOURCE, capture


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "repair"}
    process, window = None, None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-source-repair-") as temporary:
            project = Path(temporary) / "Project [owned] $literal"
            project.mkdir()
            (project / "recite.project.toml").write_text("format_version = 1\n")
            source = project / "scene.recite"
            source.write_text(SOURCE)
            invalid = ":: opening default\n? invalid café 💬"
            recovery = Path(str(source) + ".recite-editor-recovery.json")
            recovery.write_text(
                json.dumps(
                    {
                        "version": 1,
                        "baseline": SOURCE,
                        "draft": {
                            "source": SOURCE,
                            "view": "Source",
                            "draft": invalid,
                            "source_validation": "NoNewErrors",
                        },
                    }
                )
            )
            receipt = Path(temporary) / "editor-argv.json"
            editor = Path(temporary) / "Preferred IDE"
            editor.write_text(
                "#!/usr/bin/python3\nimport json,sys\nfrom pathlib import Path\n"
                f"Path({str(receipt)!r}).write_text(json.dumps(sys.argv[1:]))\n"
            )
            editor.chmod(0o755)
            arguments = [str(binary), "--project", str(project), "--editor", str(editor)]
            with output.with_suffix(".writer.log").open("w") as log:
                process = subprocess.Popen(arguments, stdout=log, stderr=subprocess.STDOUT)
                window = attach(api, bus, registry, process)
                window.wait(
                    lambda: window.find("Export draft and open editor"), "recovered source repair"
                )
                assert not window.enabled("Save file")
                assert not window.enabled("Save all")
                assert source.read_text() == SOURCE
                capture(output, "-recovered")
                window.act("Export draft and open editor")
                window.wait(lambda: receipt.is_file(), "owned IDE launch")
                exported = Path(json.loads(receipt.read_text())[0])
                path = field(window, "Repaired source file")
                window.wait(lambda: window.text(path) == str(exported), "exported artifact path")
                assert exported.read_text() == invalid
                assert exported.suffix == ".txt"
                assert source.read_text() == SOURCE
                backups = list(project.glob(".recite-recovered-*.json"))
                assert len(backups) == 1
                assert json.loads(backups[0].read_text())["draft"]["draft"] == invalid
                results["export_preserves_staged_source_backup_and_literal_editor_path"] = True
                window.act("Import repaired source")
                window.wait(
                    lambda: window.find_text("Cannot apply this source change", contains=True),
                    "invalid import refusal",
                )
                assert source.read_text() == SOURCE
                assert not window.enabled("Save file")
                capture(output, "-invalid")
                window.document_action("Close")
                window.wait(lambda: window.find("Close and keep recovery"), "close recovered draft")
                window.act("Close and keep recovery")
                assert process.wait(timeout=5) == 0
                assert json.loads(recovery.read_text())["draft"]["draft"] == invalid
                assert exported.read_text() == invalid
                results["invalid_import_and_close_keep_original_recovery_and_artifact"] = True

                process = subprocess.Popen(arguments, stdout=log, stderr=subprocess.STDOUT)
                window = attach(api, bus, registry, process)
                window.wait(
                    lambda: window.find("Export draft and open editor"), "restarted source repair"
                )
                repaired = SOURCE.replace("The last tram waits.", "The repaired tram waits.")
                exported.write_text(repaired)
                fill(window, "Repaired source file", str(exported))
                window.act("Import repaired source")
                window.wait(
                    lambda: window.find_text("Repaired source accepted.", contains=True),
                    "checked repaired source acceptance",
                )
                assert source.read_text() == SOURCE
                assert window.enabled("Save file")
                capture(output, "-accepted")
                window.act("Save file")
                window.wait(lambda: source.read_text() == repaired, "explicit repaired-source save")
                assert backups[0].is_file() and exported.read_text() == repaired
                results["restart_import_validates_and_requires_explicit_main_file_save"] = True
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
