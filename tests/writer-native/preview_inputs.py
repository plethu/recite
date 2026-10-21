"""Configured runtime inputs through a real owned Wayland window."""

import json
import os
import subprocess
import tempfile
import time
import traceback
from pathlib import Path

from localisation_interactions import (
    edit,
    open_catalogue,
    open_entry,
    reveal_field,
    reveal_text,
    search_entries,
)
from localisation_sources import action
from native_window import attach, key
from project_interactions import manuscript
from startup_interactions import fill
from writer_interactions import capture

SOURCE = """:: start default
> hello@11111111111111111111 bind=(name:string=$person)
  Hello {name}.
> tickets@22222222222222222222 bind=(count:int=$tickets)
  {count} ticket
  | {count} tickets
-> END
"""
PO = """msgid ""
msgstr "Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n"

msgctxt "11111111111111111111"
msgid "Hello {name}."
msgstr "Bonjour {name}."

msgctxt "22222222222222222222"
msgid "{count} ticket"
msgid_plural "{count} tickets"
msgstr[0] "{count} billet"
msgstr[1] "{count} billets"
"""


def value(window, name, content):
    caption = "Preview value: " + name
    reveal_field(window, caption)
    fill(window, caption, content)


def restart(window, expected):
    action(window, "Start / restart preview")
    reveal_text(window, expected)


def capture_end(window, output, suffix):
    scrollbar = window._outer_scrollbar()
    if scrollbar:
        window.focus(scrollbar)
        key("End")
    # The bus exposes the new layout before the compositor presents its frame.
    time.sleep(0.15)
    capture(output, suffix)


def exercise(window, po, output, results):
    original = manuscript(window)
    open_catalogue(window, po)
    search_entries(window, "Hello")
    reveal_text(window, "1 matching entries", prefix=True)
    open_entry(window)
    edit(window, "Salut {name}.")
    action(window, "Return to writing")
    action(window, "Preview inputs")
    reveal_field(window, "Find preview language")
    fill(window, "Find preview language", "fr-CA")
    action(window, "Search preview languages")
    action(window, "Preview in French (Canada) · fr-CA")
    reveal_text(window, "Preview language:", prefix=True)
    capture(output, "-language")
    reveal_field(window, "Preview wording")
    fill(window, "Preview wording", "formal")
    value(window, "person", "Ada")
    value(window, "tickets", "2")
    capture_end(window, output, "-values")
    restart(window, "Bonjour Ada.")
    capture_end(window, output, "-saved")
    action(window, "Continue preview")
    reveal_text(window, "2 billets")
    results["saved_localised_plural_and_locale_variant_fallback"] = True
    value(window, "tickets", "1")
    reveal_text(window, "2 billets")
    results["staging_leaves_running_snapshot_unchanged"] = True
    action(window, "Include unsaved translations")
    restart(window, "Salut Ada.")
    action(window, "Continue preview")
    reveal_text(window, "1 billet")
    capture_end(window, output, "-drafts")
    assert po.read_text() == PO
    results["draft_trial_does_not_save_po"] = True
    value(window, "tickets", "invalid")
    action(window, "Start / restart preview")
    reveal_text(window, "tickets:", prefix=True)
    reveal_text(window, "1 billet")
    capture_end(window, output, "-invalid")
    results["invalid_input_preserves_previous_trial"] = True
    value(window, "tickets", "2")
    action(window, "Use source language only")
    restart(window, "Hello Ada.")
    action(window, "Hide preview inputs")
    reveal_field(window, "Manuscript")
    assert manuscript(window) == original
    results["source_only_restart_and_manuscript_identity"] = True


def run(api, bus, registry, binary, output, scale=100):
    results = {"backend": "owned Wayland", "scenario": "preview-inputs", "ui_scale": scale}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-trial-inputs-", dir="/tmp") as temporary:
            project = Path(temporary)
            (project / "recite.project.toml").write_text("format_version=1\n")
            (project / "scene.recite").write_text(SOURCE)
            po = project / "fr.po"
            po.write_text(PO)
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
                    exercise(window, po, output, results)
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
