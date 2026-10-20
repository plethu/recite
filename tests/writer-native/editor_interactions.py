"""Owned preferred-application and explicit editor handoffs preserve live manuscript history."""
import json
import os
import subprocess
import tempfile
import time
import traceback
from pathlib import Path

from native_window import attach, key, type_text
from project_interactions import manuscript
from writer_interactions import SOURCE, capture


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "editor"}
    process, window = None, None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-editor-") as temporary:
            root = Path(temporary)
            project = root / "Project [owned] $literal"
            project.mkdir()
            (project / "recite.project.toml").write_text('format_version = 1\n')
            source = project / "scene.recite"
            source.write_text(SOURCE)
            commands = root / "bin"
            commands.mkdir()
            receipt, release = root / "argv.json", root / "release"
            launcher = commands / "xdg-open"
            launcher.write_text(
                "#!/usr/bin/python3\nimport json,sys,time\nfrom pathlib import Path\n"
                f"Path({str(receipt)!r}).write_text(json.dumps(sys.argv[1:]))\n"
                f"while not Path({str(release)!r}).exists(): time.sleep(0.02)\n"
            )
            launcher.chmod(0o755)
            environment = dict(os.environ, PATH=str(commands) + os.pathsep + os.environ["PATH"])
            arguments = [str(binary), "--project", str(project)]
            with output.with_suffix(".writer.log").open("w") as log:
                process = subprocess.Popen(arguments, env=environment, stdout=log, stderr=subprocess.STDOUT)
                window = attach(api, bus, registry, process)
                node = manuscript(window)
                window.focus(node)
                key("End", control=True)
                type_text(" unfinished")
                window.wait(lambda: window.text(node) == "The last tram waits. unfinished", "field draft before handoff")
                window.wait(lambda: window.enabled("Open in editor"), "preferred application action")
                window.act("Open in editor")
                window.wait(receipt.exists, "owned OS preferred-application request")
                assert json.loads(receipt.read_text()) == [str(source)]
                assert source.read_text() == SOURCE
                assert window.text(node) == "The last tram waits. unfinished"
                results["preferred_application_receives_literal_path_without_applying_field_draft"] = True
                window.wait(lambda: not window.enabled("Open in editor"), "one active editor launcher")
                window.focus(node)
                type_text("x")
                window.wait(lambda: window.text(node).endswith(" unfinishedx"), "typing while editor stays open")
                key("z", control=True)
                window.wait(lambda: window.text(node).endswith(" unfinished"), "undo while editor stays open")
                window.wait(lambda: window.find_text("Editor request active for scene.recite.", contains=True), "persistent editor request feedback after typing")
                capture(output, "-active")
                results["independent_editor_does_not_block_native_typing_or_undo"] = True
                release.touch()
                window.wait(lambda: window.enabled("Open in editor"), "launcher completion restores action")
                external = SOURCE + "\n# changed externally\n"
                source.write_text(external)
                before = receipt.stat().st_mtime_ns
                window.act("Open in editor")
                window.wait(lambda: window.find_text("changed on disk", contains=True), "checked-save conflict before editor launch")
                assert receipt.stat().st_mtime_ns == before
                assert source.read_text() == external
                assert window.text(node).endswith(" unfinished")
                capture(output, "-conflict")
                results["disk_conflict_prevents_launch_and_retains_native_draft"] = True
                window.act("Close")
                window.wait(lambda: window.find("Close and keep recovery"), "close with retained field draft")
                window.act("Close and keep recovery")
                assert process.wait(timeout=5) == 0
                source.write_text(SOURCE)
                missing = root / "missing editor"
                process = subprocess.Popen(arguments + ["--editor", str(missing)], env=environment, stdout=log, stderr=subprocess.STDOUT)
                window = attach(api, bus, registry, process)
                node = manuscript(window)
                window.act("Open in editor")
                window.wait(lambda: window.find_text("Could not start the configured editor", contains=True), "failed launcher feedback")
                window.wait(lambda: window.enabled("Open in editor"), "failed launcher restores retry")
                assert window.text(node).endswith(" unfinished")
                feedback = window.find_text("Could not start the configured editor", contains=True)
                results["failure_feedback_text"] = window.text(feedback)
                time.sleep(0.15)  # Allow the compositor to paint the already-published failure.
                capture(output, "-failure")
                window.act("Settings")
                window.wait(lambda: window.find("Increase UI scale"), "personal scale controls")
                for percent in range(110, 201, 10):
                    window.act("Increase UI scale")
                    window.wait(lambda: window.find(f"UI scale: {percent}%"), "larger interface scale")
                window.act("Done with settings")
                window.wait(lambda: window.find_text("Could not start the configured editor", contains=True), "retained error at 200 percent")
                time.sleep(0.15)
                capture(output, "-failure200")
                window.act("Settings")
                for percent in range(190, 99, -10):
                    window.act("Decrease UI scale")
                    window.wait(lambda: window.find(f"UI scale: {percent}%"), "restored interface scale")
                window.act("Done with settings")
                window.wait(lambda: window.enabled("Use system default"), "available fallback after scaling")
                before = receipt.stat().st_mtime_ns
                window.act("Use system default")
                window.wait(lambda: receipt.stat().st_mtime_ns != before, "fallback from failed explicit editor")
                window.wait(lambda: window.enabled("Open in editor"), "preferred launcher completion")
                assert window.text(node).endswith(" unfinished")
                results["failed_override_can_use_system_default_without_restarting_or_losing_draft"] = True
                results["failed_explicit_editor_restores_controls_and_preserves_recovered_draft"] = True
                window.act("Close")
                window.wait(lambda: window.find("Close and keep recovery"), "last recovery close")
                window.act("Close and keep recovery")
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
