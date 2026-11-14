"""Representative branching-beat reading, map, editing and reference captures at both UI scales."""

import json
import os
import shutil
import subprocess
import tempfile
import traceback
from pathlib import Path

from native_window import attach, key, type_text
from project_interactions import manuscript
from startup_interactions import fill
from writer_interactions import capture, capture_settled

ROOT = Path(__file__).resolve().parents[2]


def run(api, bus, registry, binary, output):
    result = {
        "scope": "current Relay Hub reading, map, inline editing and split reference",
        "frames": [],
    }
    process = None
    window = None
    try:
        for scale in [100, 200]:
            window = None
            with tempfile.TemporaryDirectory(
                prefix="recite-fresh-workbench-", dir="/tmp"
            ) as temporary:
                project = Path(temporary)
                (project / "src").mkdir()
                (project / "schema").mkdir()
                (project / "recite.project.toml").write_text(
                    'format_version=1\n[project]\ncontent_set="relay-hub"\nschema="schema/writer_examples.json"\n[discovery]\nsource_roots=["src"]\n'
                )
                shutil.copyfile(
                    ROOT / "fixtures/schema/valid/writer_examples.json",
                    project / "schema/writer_examples.json",
                )
                source = (
                    ROOT / "fixtures/recite/valid/writer_examples/relay_hub.recite"
                ).read_text()
                document = project / "src/relay_hub.recite"
                document.write_text(source)
                config = project / "config.toml"
                config.write_text(
                    f"config_version=1\n[writer]\nconfirm_exit=false\n[writer.presentation]\nui_scale={scale}\nsplit={'true' if scale == 200 else 'false'}\n"
                )
                with output.with_name(output.stem + f"-{scale}.writer.log").open("w") as log:
                    process = subprocess.Popen(
                        [
                            str(binary),
                            "--project",
                            str(project),
                            "--document",
                            "src/relay_hub.recite",
                            "--route",
                            "recite://writer/write?scene=src/relay_hub.recite&view=script",
                        ],
                        env=dict(os.environ, RECITE_CONFIG=str(config), RUST_LOG="warn"),
                        stdout=log,
                        stderr=subprocess.STDOUT,
                    )
                    window = attach(api, bus, registry, process)
                    window.wait(
                        lambda: window.find_text("If you're here", prefix=True),
                        "Relay Desk reading",
                    )
                    if scale == 200:
                        window.wait(
                            lambda: window.find_text("Split needs a wider window", prefix=True),
                            "direct constrained Split recovery",
                        )
                        capture_settled(window, output, "-200-split-fallback")
                        result["frames"].append("200-split-fallback")
                        window.act("Map")
                        window.wait(
                            lambda: window.find_action("Open script"), "map from constrained Split"
                        )
                        window.act("Script")
                        window.wait(
                            lambda: window.find_text("If you're here", prefix=True),
                            "Script after constrained Split",
                        )
                    window.focus(
                        window.wait(lambda: window.find_action("Script"), "current Script selector")
                    )
                    capture_settled(window, output, f"-{scale}-script")
                    result["frames"].append(f"{scale}-script")
                    window.act("Map")
                    window.wait(lambda: window.find_action("Open script"), "Relay Hub map toolbar")
                    window.focus(window.find_action("Map"))
                    capture_settled(window, output, f"-{scale}-map")
                    result["frames"].append(f"{scale}-map")
                    window.act("Script")
                    ready = window.wait(
                        lambda: window.find("Manuscript") or window.find("Edit passage 1"),
                        "live Script editor after map transition",
                    )
                    if api.property_value(bus, ready, api.ACCESSIBLE, "Name") != "Manuscript":
                        window.edit_passage(1)
                    node = manuscript(window)
                    window.focus(node)
                    original = window.text(node)
                    key("End", control=True)
                    type_text("x")
                    window.wait(
                        lambda: window.text(node) == original + "x", "inline Relay Desk draft"
                    )
                    capture(output, f"-{scale}-inline-edit")
                    result["frames"].append(f"{scale}-inline-edit")
                    key("z", control=True)
                    window.wait(lambda: window.text(node) == original, "native inline Undo")
                    pin = window.wait(
                        lambda: window.find_action("Pin reference"), "pin the current whole beat"
                    )
                    window.focus(pin)
                    window.act("Pin reference")
                    key("p", control=True)
                    fill(window, "Search query", "missing courier")
                    window.wait(
                        lambda: any(
                            name.startswith("Selected · ")
                            and "missing courier" in name.lower().replace("_", " ")
                            for _, name in window.nodes()
                        ),
                        "selected Missing Courier result",
                    )
                    key("Return")
                    window.wait(
                        lambda: window.find_text("Our courier", prefix=True),
                        "Missing Courier reading",
                    )
                    split = "Split unavailable" if scale == 200 else "Split"
                    if scale == 200:
                        assert not window.enabled(split)
                    else:
                        window.act(split)
                    window.wait(lambda: window.find("Unpin reference"), "retained pinned reference")
                    window.focus(
                        window.wait(
                            lambda: window.find_action("Script" if scale == 200 else split),
                            "live Split selector after map preparation",
                        )
                    )
                    capture_settled(window, output, f"-{scale}-split-reference")
                    result["frames"].append(f"{scale}-split-reference")
                    assert document.read_text() == source
                    result[f"inline_undo_and_source_preserved_{scale}"] = True
                    window.close_clean()
                    assert process.wait(timeout=5) == 0
                    process = None
        result["passed"] = True
    except Exception:
        result["passed"] = False
        result["error"] = traceback.format_exc()
        if window is not None:
            try:
                result["exposure_at_failure"] = window.exposure_diagnostic()
            except Exception as error:
                result["exposure_error"] = str(error)
        if window is not None and process is not None and process.poll() is None:
            capture(output, "-error")
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
        output.write_text(json.dumps(result, indent=2) + "\n")
    return result["passed"]
