"""Inspect typed runtime evidence without changing or rerunning its captured inputs."""

import json
import os
import subprocess
import tempfile
import traceback
from pathlib import Path

from localisation_interactions import (
    open_catalogue,
    open_entry,
    reveal,
    reveal_field,
    reveal_text,
    search_entries,
)
from localisation_sources import action
from native_window import attach
from preview_inputs import value
from project_interactions import manuscript
from startup_interactions import fill
from writer_interactions import capture

SOURCE = """:: start default
:if accepts(actor, "spoken", 7, 2.5, true, false)
  > hello@11111111111111111111
    Hello.
> tickets@22222222222222222222 bind=(count:int=$tickets)
  {count} ticket
  | {count} tickets
-> END
"""
PO = """msgid ""
msgstr "Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n"

msgctxt "11111111111111111111&formal"
msgid "Hello."
msgstr "Bonjour formel."

msgctxt "22222222222222222222"
msgid "{count} ticket"
msgid_plural "{count} tickets"
msgstr[0] "{count} billet"
msgstr[1] "{count} billets"
"""


def exercise(window, project, output, results):
    manuscript(window)
    open_catalogue(window, project / "fr.po")
    search_entries(window, "formal")
    reveal_text(window, "1 matching entries", prefix=True)
    open_entry(window)
    action(window, "Preview this passage")
    action(window, "Configure preview")
    window.reveal_end("Preview value: tickets")
    value(window, "tickets", "2")
    action(window, "Hide preview inputs")
    action(window, "Start / restart preview")
    reveal(window, "Answer true")
    action(window, "Preview trace")
    action(window, "Open trace record 1 · Condition requested · accepts")
    reveal_text(window, "Argument 1 · Identifier", prefix=True)
    reveal_text(window, "actor")
    reveal_text(window, "Argument 2 · String", prefix=True)
    reveal_text(window, "spoken")
    capture(output, "-typed-argument")
    results["condition_arguments_preserve_identifier_and_string_types"] = True
    action(window, "Answer true")
    reveal_text(window, "Bonjour formel.")
    action(window, "Continue preview")
    reveal_text(window, "2 billets")
    action(window, "Trace: Translation lookups")
    action(window, "Open trace record 1 · Translation lookup · Hello.")
    reveal_text(window, "Resolved text", prefix=True)
    reveal_text(window, "Bonjour formel.")
    capture(output, "-translation")
    results["translation_details_come_from_the_running_lookup"] = True
    action(window, "Trace: Plural lookups")
    action(window, "Open trace record 1 · Plural lookup · {count} tickets")
    reveal_text(window, "Count", prefix=True)
    reveal_text(window, "Selected plural form", prefix=True)
    reveal_text(window, "2")
    capture(output, "-plural")
    results["plural_details_expose_count_and_selected_form"] = True
    action(window, "Trace: Events")
    action(window, "Open trace record 1 · Condition requested · accepts")
    action(window, "Return to writing")
    reveal_field(window, "Manuscript")
    fill(window, "Manuscript", "A later authoring draft.")
    window.document_action("Configure preview")
    action(window, "Hide preview inputs")
    reveal_text(window, "spoken")
    action(window, "Start / restart preview")
    reveal_text(window, "Apply or discard", prefix=True)
    reveal_text(window, "spoken")
    action(window, "Return to writing")
    action(window, "Apply draft")
    reveal_text(window, "Applied edit updated.")
    window.document_action("Configure preview")
    action(window, "Hide preview inputs")
    action(window, "Start / restart preview")
    window.reveal_end("Open trace record 1 · Condition requested · accepts")
    capture(output, "-restarted")
    assert (project / "scene.recite").read_text() == SOURCE
    assert (project / "fr.po").read_text() == PO
    results["typing_and_failed_restart_keep_trace_successful_restart_resets_cursor"] = True


def run(api, bus, registry, binary, output, scale=100):
    results = {"backend": "owned Wayland", "scenario": "preview-trace", "ui_scale": scale}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-preview-trace-", dir="/tmp") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text("format_version=1\n")
            (project / "scene.recite").write_text(SOURCE)
            (project / "fr.po").write_text(PO)
            config = project / "ui.toml"
            config.write_text(f"config_version=1\n[writer.presentation]\nui_scale={scale}\n")
            with output.with_suffix(".writer.log").open("w") as log:
                process = subprocess.Popen(
                    [str(binary), "--project", str(project)],
                    env=dict(os.environ, RECITE_CONFIG=str(config)),
                    stdout=log,
                    stderr=subprocess.STDOUT,
                )
                window = attach(api, bus, registry, process)
                try:
                    exercise(window, project, output, results)
                except Exception:
                    results["visible_at_failure"] = [name for _, name in window.nodes()]
                    capture(output, "-error")
                    raise
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
    finally:
        if process and process.poll() is None:
            process.terminate()
            process.wait(timeout=10)
    output.write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results))
    return "error" not in results


def run_scale(api, bus, registry, binary, output):
    return run(api, bus, registry, binary, output, 200)
