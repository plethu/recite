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
from native_window import attach, key, type_text
from project_interactions import manuscript
from startup_interactions import field, fill
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


def tab_to_language(window):
    """Observe a real Tab move and its viewport bounds without probe scrolling."""
    key("Tab")
    focused = 1 << int(window.api.Atspi.StateType.FOCUSED)
    coordinates = window.api.GLib.Variant("(u)", (int(window.api.Atspi.CoordType.WINDOW),))

    def visible():
        node = field(window, "Find preview language")
        if not window.call(node, "Accessible", "GetState")[0][0] & focused:
            return False
        viewport = window.section_scrollbar("Conversation preview")
        if viewport is None:
            return False
        bounds = window.call(node, "Component", "GetExtents", coordinates)[0]
        clip = window.call(viewport, "Component", "GetExtents", coordinates)[0]
        return bounds[1] >= clip[1] and bounds[1] + bounds[3] <= clip[1] + clip[3]

    window.wait(visible, "Tab reveals the preview language input within its viewport")


def exercise(window, po, output, results):
    original = manuscript(window)
    original_text = window.text(original)
    window.focus(original)
    key("End", control=True)
    type_text("x")
    window.wait(lambda: window.text(original) == original_text + "x", "retained history seed")
    key("z", control=True)
    window.wait(lambda: window.text(original) == original_text, "clean source before preview")
    open_catalogue(window, po)
    search_entries(window, "Hello")
    reveal_text(window, "1 matching entries", prefix=True)
    open_entry(window)
    edit(window, "Salut {name}.")
    action(window, "Return to writing")
    window.document_action("Configure preview")
    tab_to_language(window)
    capture(output, "-language-tab")
    results["native_tab_reveals_preview_input"] = True
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
    window.act("Return to writing")

    def returned():
        current = window.find("Manuscript")
        if current and window.text(current) == original_text:
            focused = 1 << int(window.api.Atspi.StateType.FOCUSED)
            return (
                current if window.call(current, "Accessible", "GetState")[0][0] & focused else None
            )
        return None

    current = window.wait(returned, "automatically restored writing focus")
    key("y", control=True)
    window.wait(
        lambda: window.text(current) == original_text + "x", "redo after preview configuration"
    )
    key("z", control=True)
    window.wait(lambda: window.text(current) == original_text, "undo after preview configuration")
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
