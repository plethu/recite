"""Create a catalogue, inspect source changes, reject stale review and publish only PO."""

import json
import os
import subprocess
import tempfile
import traceback
from pathlib import Path

from localisation_interactions import edit, open_entry, reveal, reveal_field, reveal_text
from native_window import attach, key, type_text
from project_interactions import manuscript
from project_tools_interactions import command
from startup_interactions import fill
from writer_interactions import SOURCE, capture


def action(window, caption):
    reveal(window, caption)
    window.act(caption)


def apply_source(window, text):
    window.reveal_start("Return to writing")
    window.act("Return to writing")
    reveal_field(window, "Manuscript")
    native = manuscript(window)
    window.focus(native)
    key("a", control=True)
    type_text(text)
    window.wait(lambda: window.text(native) == text, "source field text")
    reveal(window, "Apply draft")
    window.wait(lambda: window.enabled("Apply draft"), "source Apply available")
    action(window, "Apply draft")
    reveal_text(window, "Applied edit updated.")
    command(window, "Localise")


def run(api, bus, registry, binary, output, scale=100):
    results = {"backend": "owned Wayland", "scenario": "localisation-sources", "ui_scale": scale}
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-po-sources-", dir="/tmp") as temporary:
            root = Path(temporary)
            (root / "recite.project.toml").write_text("format_version=1\n")
            source = root / "scene.recite"
            source.write_text(SOURCE)
            config = root / "ui.toml"
            config.write_text(f"config_version=1\n[writer.presentation]\nui_scale={scale}\n")
            with output.with_suffix(".writer.log").open("w") as log:
                process = subprocess.Popen(
                    [str(binary), "--project", str(root)],
                    env=dict(os.environ, RECITE_CONFIG=str(config)),
                    stdout=log,
                    stderr=subprocess.STDOUT,
                )
                window = attach(api, bus, registry, process)
                original = manuscript(window)
                command(window, "Localise")
                action(window, "Add language")
                reveal(window, "Language name or tag")
                fill(window, "Language name or tag", "fr-CA")
                action(window, "Search languages")
                reveal(window, "Choose French (Canada)", prefix=True)
                choice = next(
                    name
                    for _, name in window.nodes()
                    if name.startswith("Choose ") and name.endswith("· fr-CA")
                )
                window.act(choice)
                reveal_text(window, "Selected language:", prefix=True)
                reveal(window, "Create catalogue")
                window.wait(
                    lambda: window.find("Create catalogue") and window.enabled("Create catalogue"),
                    "selected language creation available",
                )
                window.wait(
                    lambda: window.find_text("Selected language:", prefix=True),
                    "selected language confirmation",
                )
                capture(output, "-language")
                window.act("Create catalogue")
                po = root / "locale/fr-CA.po"
                window.wait(po.is_file, "new PO catalogue")
                reveal(window, "Catalogue files")
                assert "Language: fr-CA" in po.read_text()
                assert 'msgid "The last tram waits."' in po.read_text()
                assert 'msgstr ""' in po.read_text()
                assert source.read_text() == SOURCE
                results["language_search_creates_explicit_blank_catalogue"] = True
                reveal(window, "Search translations")
                fill(window, "Search translations", "The last tram waits.")
                reveal(window, "Open translation · ", prefix=True)
                search = window.wait(
                    lambda: next(
                        (
                            node
                            for node, name in window.nodes()
                            if name == "Search translations"
                            and window.call(node, "Accessible", "GetRole")[0]
                            == int(api.Atspi.Role.PUSH_BUTTON)
                        ),
                        None,
                    ),
                    "queue search action",
                )
                window.call(search, "Action", "DoAction", api.GLib.Variant("(i)", (0,)))
                reveal(window, "Open translation · ", prefix=True)
                window.wait(
                    lambda: window.find_text("1 matching entries", prefix=True),
                    "matching source entry",
                )
                open_entry(window)
                edit(window, "Le dernier tram attend.")
                action(window, "Save translation")
                window.wait(
                    lambda: "Le dernier tram attend." in po.read_text(), "translation saved"
                )
                apply_source(window, "The last tram leaves.")
                reveal(window, "Return to translation queue")
                window.act("Return to translation queue")
                action(window, "Review source updates")
                reveal(window, "Apply source updates to PO")
                capture(output, "-review")
                assert 'msgid "The last tram waits."' in po.read_text()
                assert source.read_text() == SOURCE
                results["source_review_is_observational"] = True
                apply_source(window, "The last tram crosses the harbour.")
                capture(output, "-stale-review-before-apply")
                action(window, "Apply source updates to PO")
                window.reveal_start("Return to writing")
                window.wait(
                    lambda: window.find_text("source or translation drafts changed", contains=True),
                    "stale source review rejected",
                )
                assert 'msgid "The last tram waits."' in po.read_text()
                results["newer_source_invalidates_review_without_publishing"] = True
                action(window, "Recheck source updates")
                window.reveal_start("Return to writing")
                window.wait(
                    lambda: window.find_text("Source update review ready.", prefix=True),
                    "current source review accepted",
                )
                reveal(window, "Apply source updates to PO")
                capture(output, "-rechecked")
                window.act("Apply source updates to PO")
                window.wait(
                    lambda: 'msgid "The last tram crosses the harbour."' in po.read_text(),
                    "reviewed source refresh published",
                )
                reveal(window, "Catalogue files")
                assert "Le dernier tram attend." in po.read_text()
                assert "fuzzy" in po.read_text()
                assert source.read_text() == SOURCE
                results[
                    "source_refresh_preserves_translation_and_marks_review_without_saving_source"
                ] = True
                window.reveal_start("Return to writing")
                window.act("Return to writing")
                reveal_field(window, "Manuscript")
                current = manuscript(window)
                assert tuple(current) == tuple(original)
                assert window.text(current) == "The last tram crosses the harbour."
                results["both_checkouts_preserve_the_mounted_manuscript"] = True
                command(window, "Localise")
                window.reveal_start("Close")
                window.act("Close")
                reveal(window, "Close and keep recovery")
                window.act("Close and keep recovery")
                assert process.wait(timeout=10) == 0
    except Exception as error:
        results["error"] = repr(error)
        results["error_traceback"] = traceback.format_exc()
        if process is not None and process.poll() is None:
            capture(output, "-error")
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
    output.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(results, ensure_ascii=False))
    return "error" not in results


def run_scale(api, bus, registry, binary, output):
    return run(api, bus, registry, binary, output, scale=200)
