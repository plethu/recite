"""The standalone production-component specimen never loads or saves user preferences."""

import json
import os
import subprocess
import tempfile
import time
import traceback
from pathlib import Path

from localisation_interactions import reveal_field
from native_window import attach, key, type_text
from startup_interactions import field, fill
from writer_interactions import capture


def scroll(window, direction):
    window.focus(window._outer_scrollbar())
    key(direction)
    time.sleep(0.15)


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "specimen"}
    process, window = None, None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-specimen-", dir="/tmp") as temporary:
            config = Path(temporary) / "config.toml"
            original = 'config_version=1\n[writer]\ntheme="dark"\nmonochrome=true\n'
            config.write_text(original)
            with output.with_suffix(".writer.log").open("w") as log:
                process = subprocess.Popen([str(binary), "--design-system"], env=dict(os.environ, RECITE_CONFIG=str(config)), stdout=log, stderr=subprocess.STDOUT)
                window = attach(api, bus, registry, process)
                window.wait(lambda: window.find("Dark appearance"), "session-only default appearance")
                window.act("Try scene")
                window.wait(lambda: window.find_text("Primary action activated."), "production button callback")
                assert not window.enabled("Save changes")
                capture(output, "-light100")
                reveal_field(window, "Find a destination")
                fill(window, "Find a destination", "Station")
                window.wait(lambda: window.find("Station History"), "filtered destination")
                window.act("Station History")
                window.wait(lambda: window.find_text("Destination: Station History"), "selected destination")
                reveal_field(window, "Specimen manuscript")
                manuscript = field(window, "Specimen manuscript")
                before = window.text(manuscript)
                window.focus(manuscript)
                key("End", control=True)
                type_text("x")
                window.wait(lambda: window.text(manuscript) == before + "x", "native specimen typing")
                key("z", control=True)
                window.wait(lambda: window.text(manuscript) == before, "native specimen undo")
                capture(output, "-fields100")
                results["production_actions_search_and_manuscript_native_undo"] = True
                scroll(window, "Home")
                window.act("Dark appearance")
                window.wait(lambda: window.find("Light appearance"), "dark appearance rendered")
                time.sleep(0.15)
                capture(output, "-dark100")
                window.act("Monochrome")
                window.act("200% UI scale")
                window.wait(lambda: window.find("100% UI scale"), "session scale200")
                time.sleep(0.15)
                capture(output, "-mono200")
                window.act("Monochrome")
                window.act("Light appearance")
                window.wait(lambda: window.find("Dark appearance"), "light appearance rendered")
                window.act("Reduced motion")
                time.sleep(0.15)
                capture(output, "-light200")
                reveal_field(window, "Find a destination")
                window.act("Map")
                time.sleep(0.15)
                capture(output, "-fields200")
                results["scaled_native_checkbox_and_radio_actions"] = True
                scroll(window, "End")
                window.wait(lambda: window.find_text("Selected passage"), "production palette samples")
                capture(output, "-samples200")
                results["appearance_monochrome_and_scale_are_session_only"] = True
                assert config.read_text() == original
                scroll(window, "Home")
                window.act("Close specimen")
                assert process.wait(timeout=5) == 0
                assert config.read_text() == original
                results["no_user_preferences_or_project_owner"] = True
    except Exception as error:
        results["error"], results["error_traceback"] = repr(error), traceback.format_exc()
        if process is not None and process.poll() is None:
            capture(output, "-error")
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
    output.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(results, ensure_ascii=False))
    return "error" not in results
