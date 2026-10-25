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
from preview_controls import diagnostic_focus, focused_in_content, within_content
from preview_controls import start as start_preview
from project_interactions import manuscript
from startup_interactions import field, fill
from writer_interactions import capture, capture_settled

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
    start_preview(window)
    reveal_text(window, expected)


def capture_end(window, output, suffix):
    scrollbar = window._outer_scrollbar()
    if scrollbar:
        window.focus(scrollbar)
        key("End")
    # The bus exposes the new layout before the compositor presents its frame.
    time.sleep(0.15)
    capture(output, suffix)


def configuration_focus_and_tab(window):
    """Observe the automatic input focus and native forward/backward Tab moves."""

    def visible_and_focused(caption):
        node = (
            field(window, caption) if caption == "Find preview language" else window.find(caption)
        )
        return focused_in_content(window, node)

    window.wait(
        lambda: visible_and_focused("Find preview language"),
        "configuration focuses the visible preview language input",
    )
    key("Tab")
    window.wait(
        lambda: visible_and_focused("Search preview languages"),
        "Tab reaches the visible language search action",
    )
    key("Tab", shift=True)
    window.wait(
        lambda: visible_and_focused("Find preview language"),
        "Shift+Tab returns to the visible preview language input",
    )


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
    configuration_focus_and_tab(window)
    capture(output, "-language-tab")
    results["native_tab_reveals_preview_input"] = True
    reveal_field(window, "Find preview language")
    fill(window, "Find preview language", "fr-CA")
    action(window, "Search preview languages")
    action(window, "Preview in French (Canada) · fr-CA")
    reveal_text(window, "Preview language:", prefix=True)
    action(window, "Hide preview inputs")
    action(window, "Configure preview")
    configuration_focus_and_tab(window)
    results["selected_locale_configuration_focuses_the_language_query"] = True
    capture(output, "-language")
    reveal_field(window, "Preview wording")
    fill(window, "Preview wording", "formal")
    value(window, "person", "Ada")
    value(window, "tickets", "2")
    capture_end(window, output, "-values")
    restart(window, "Bonjour Ada.")
    capture_end(window, output, "-saved")
    action(window, "Hide preview inputs")
    scrollbar = window.wait(
        lambda: (window.section_scrollbar("Conversation preview"),), "retained conversation layout"
    )[0]
    if scrollbar is not None:
        window.focus(scrollbar)
        key("Home")
    window.wait(lambda: window.find_text("fr-CA", prefix=True), "captured preview context")
    capture_settled(window, output, "-current-context")
    action(window, "Continue preview")
    reveal_text(window, "2 billets")
    results["saved_localised_plural_and_locale_variant_fallback"] = True
    action(window, "Configure preview")
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
    invalid_field = field(window, "Preview value: tickets")
    start_preview(window)
    window.wait(
        lambda: window.find("Preview failed. Existing conversation kept."),
        "fixed restart failure feedback",
    )
    reveal_text(window, "tickets:", prefix=True)
    reveal_text(window, "1 billet")
    capture_end(window, output, "-invalid")
    results["invalid_input_preserves_previous_trial"] = True
    window.act("Review current inputs")
    window.wait(
        lambda: focused_in_content(window, field(window, "Preview value: tickets")),
        "review reveals and focuses the invalid value",
    )
    window.wait(
        lambda: (
            within_content(window, window.find("tickets · whole number"))
            and within_content(window, window.find("Enter a whole number."))
        ),
        "review reveals the value label, input and validation hint together",
    )
    assert field(window, "Preview value: tickets") == invalid_field
    key("z", control=True)
    window.wait(
        lambda: window.text(invalid_field) == "invali",
        "native keystroke undo after reviewing failed restart",
    )
    key("y", control=True)
    window.wait(
        lambda: window.text(invalid_field) == "invalid",
        "typing redo after reviewing failed restart",
    )
    window.focus(window.find("Review current inputs"))
    capture_settled(window, output, "-review-invalid")
    results["fixed_failure_feedback_and_review_preserve_field_identity_and_undo"] = True
    value(window, "tickets", "2")
    window.act("Review current inputs")
    diagnostic = window.wait(
        lambda: window.find("Preview preparation diagnostic"),
        "retained failure diagnostic after correcting current values",
    )
    coordinates = window.api.GLib.Variant("(u)", (int(window.api.Atspi.CoordType.WINDOW),))
    frame = window.call(window.application, "Accessible", "GetChildren")[0][0]
    bounds = window.call(diagnostic, "Component", "GetExtents", coordinates)[0]
    viewport = window.call(frame, "Component", "GetExtents", coordinates)[0]
    assert bounds[1] >= viewport[1] and bounds[1] + bounds[3] <= viewport[1] + viewport[3]
    diagnostic_focus(window)
    capture_settled(window, output, "-error-details")
    results["corrected_current_inputs_reveal_the_failed_attempt_diagnostic"] = True
    window.act("Hide preview error")
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
