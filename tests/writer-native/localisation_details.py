"""Existing wordings, plural guidance and malformed-metadata save guards."""

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
from project_interactions import manuscript
from startup_interactions import field, fill
from writer_interactions import capture

SOURCE = """:: start default
> hello@11111111111111111111
  Hello.
> letters@22222222222222222222 bind=(count:int=$letters)
  One letter
  | Many letters
-> END
"""
PO = """msgid ""
msgstr "Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n"

#. file: scene.recite
#. block: start
# translator note
msgctxt "22222222222222222222"
msgid "One letter"
msgid_plural "Many letters"
msgstr[0] "Une lettre"
msgstr[1] "Plusieurs lettres"

#. file: scene.recite
#. block: start
msgctxt "22222222222222222222&formal"
msgid "One letter"
msgid_plural "Many letters"
msgstr[0] "Un courrier"
msgstr[1] "Plusieurs courriers"
"""


def choose(window, name, expected):
    caption = "Wording: " + name
    reveal(window, caption)
    window.wait(
        lambda: window.find(caption) and window.enabled(caption), "wording navigation available"
    )
    window.act(caption)
    reveal_field(window, "Translation form 1")
    window.wait(
        lambda: window.text(field(window, "Translation form 1")) == expected, "wording fields"
    )


def inspect_formal(window, path):
    open_catalogue(window, path)
    search_entries(window, "formal")
    reveal_text(window, "1 matching entries", prefix=True)
    open_entry(window)


def exercise(window, project, output, results):
    original = manuscript(window)
    po = project / "fr.po"
    inspect_formal(window, po)
    window.wait(
        lambda: window.text(field(window, "Translation form 1")) == "Un courrier",
        "formal translation",
    )
    reveal(window, "Plural form 1 · 0, 1")
    capture(output, "-formal")
    action(window, "Plural rule")
    reveal_text(window, "nplurals=2;", prefix=True)
    capture(output, "-rule")
    results["target_language_plural_examples_and_rule"] = True
    choose(window, "Default wording", "Une lettre")
    fill(window, "Translation form 1", "Lettre conservée")
    choose(window, "formal", "Un courrier")
    reveal_field(window, "Translation form 2")
    fill(window, "Translation form 2", "Courriers formels")
    choose(window, "Default wording", "Lettre conservée")
    capture(output, "-default-draft")
    results["wordings_keep_separate_whole_entry_drafts"] = True
    action(window, "Save all")
    window.wait(
        lambda: "Lettre conservée" in po.read_text() and "Courriers formels" in po.read_text(),
        "both wordings saved",
    )
    assert "# translator note" in po.read_text()
    assert (project / "scene.recite").read_text() == SOURCE
    reveal(window, "Wording: formal")
    window.wait(
        lambda: window.find("Wording: formal") and window.enabled("Wording: formal"),
        "ordered save completion collected",
    )
    results["save_all_keeps_plural_siblings_and_po_metadata"] = True
    action(window, "Return to translation queue")
    invalid = project / "invalid.po"
    malformed = PO.replace("nplurals=2", "nplurals=3").replace('msgctxt "', '#, fuzzy\nmsgctxt "')
    invalid.write_text(malformed)
    action(window, "Catalogue files")
    inspect_formal(window, invalid)
    reveal_text(window, "This entry does not match a valid Plural-Forms rule.", prefix=True)
    assert not any(
        name in ("Plural rule", "Hide plural rule")
        and window.call(node, "Accessible", "GetRole")[0] == int(window.api.Atspi.Role.PUSH_BUTTON)
        for node, name in window.nodes()
    ), "invalid rules stay visible without an ineffective disclosure action"
    capture(output, "-invalid-rule")
    reveal_field(window, "Translation form 1")
    fill(window, "Translation form 1", "Draft retained for metadata repair")
    reveal(window, "Save translation")
    assert not window.enabled("Save translation")
    assert invalid.read_text() == malformed
    results["invalid_plural_metadata_retains_draft_and_disables_save"] = True
    action(window, "Return to writing")
    reveal_field(window, "Manuscript")
    assert manuscript(window) == original
    results["writing_widget_retained"] = True


def run(api, bus, registry, binary, output, scale=100):
    results = {"backend": "owned Wayland", "scenario": "localisation-details", "ui_scale": scale}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-po-details-", dir="/tmp") as temporary:
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
