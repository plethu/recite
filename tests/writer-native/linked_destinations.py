"""Cold and forwarded links use real catalogue destinations and the existing IDE launcher."""

import json
import os
import subprocess
import tempfile
import time
import traceback
from pathlib import Path
from urllib.parse import urlencode

from localisation_interactions import reveal, reveal_field, reveal_text
from native_window import attach, key
from startup_interactions import field
from writer_interactions import capture

SOURCE = ":: start default\n> line@11111111111111111111\n  The last tram waits.\n-> END\n"
PO = 'msgctxt "99999999999999999999"\nmsgid "Other"\nmsgstr "Autre"\n\n#. file: scene.recite\n#. block: start\nmsgctxt "11111111111111111111"\nmsgid "The last tram waits."\nmsgstr "Le dernier tram attend."\n'


def route(screen, **query):
    return "recite://writer/" + screen + "?" + urlencode({"scene": "scene.recite", **query})


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "linked-destinations"}
    process, window = None, None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-linked-", dir="/tmp") as temporary:
            root = Path(temporary)
            for scale in [100, 200]:
                project = root / str(scale)
                project.mkdir()
                (project / "recite.project.toml").write_text("format_version=1\n")
                source = project / "scene.recite"
                source.write_text(SOURCE)
                (project / "fr.po").write_text(PO)
                config = project / "config.toml"
                config.write_text(f"config_version=1\n[writer.presentation]\nui_scale={scale}\n")
                environment = dict(os.environ, RECITE_CONFIG=str(config))
                receipt, release = project / "editor.json", project / "release"
                editor = project / "editor.py"
                editor.write_text(
                    "import json,sys,time\nfrom pathlib import Path\n"
                    f"Path({str(receipt)!r}).write_text(json.dumps(sys.argv[1:]))\n"
                    f"while not Path({str(release)!r}).exists(): time.sleep(0.02)\n"
                )
                base = [str(binary), "--project", str(project)]
                launcher = ["--editor", "/usr/bin/python3", "--editor-arg", str(editor)]
                with output.with_suffix(f".{scale}.writer.log").open("w") as log:
                    process = subprocess.Popen(
                        base
                        + [
                            "--route",
                            route("localise", catalogue="fr.po", passage="11111111111111111111"),
                        ],
                        env=environment,
                        stdout=log,
                        stderr=subprocess.STDOUT,
                    )
                    window = attach(api, bus, registry, process)
                    reveal_text(window, "Selected source passage")
                    window.wait(
                        lambda: window.find_text("The last tram waits."), "linked source context"
                    )
                    reveal(window, "Translate this passage")
                    capture(output, f"-passage-{scale}")
                    window.act("Translate this passage")
                    reveal_field(window, "Translation form 1")
                    assert (
                        window.text(field(window, "Translation form 1"))
                        == "Le dernier tram attend."
                    )
                    results[f"cold_passage_context_and_matching_entry_{scale}"] = True
                    forwarded = subprocess.run(
                        base
                        + [
                            "--route",
                            route("translation", catalogue="fr.po", entry="11111111111111111111"),
                        ],
                        env=environment,
                        stdout=log,
                        stderr=subprocess.STDOUT,
                        timeout=10,
                    )
                    assert forwarded.returncode == 0
                    reveal_field(window, "Translation form 1")
                    assert (
                        window.text(field(window, "Translation form 1"))
                        == "Le dernier tram attend."
                    )
                    capture(output, f"-entry-{scale}")
                    results[f"forwarded_stable_entry_context_{scale}"] = True
                    forwarded = subprocess.run(
                        base + launcher + ["--route", route("write", view="source")],
                        env=environment,
                        stdout=log,
                        stderr=subprocess.STDOUT,
                        timeout=10,
                    )
                    assert forwarded.returncode == 0
                    window.wait(receipt.exists, "source link starts configured editor")
                    assert json.loads(receipt.read_text()) == [str(source)]
                    assert not release.exists(), "activation completed while editor remains open"
                    assert source.read_text() == SOURCE
                    results[f"forwarded_source_link_completes_at_process_start_{scale}"] = True
                    release.touch()
                    window.reveal_start("Close")
                    window.close_clean()
                    assert process.wait(timeout=5) == 0
                    long_source = "À la gare, le dernier tram attend. " * 64
                    source.write_text(SOURCE.replace("The last tram waits.", long_source))
                    process = subprocess.Popen(
                        base
                        + [
                            "--route",
                            route("localise", catalogue="fr.po", passage="11111111111111111111"),
                        ],
                        env=environment,
                        stdout=log,
                        stderr=subprocess.STDOUT,
                    )
                    window = attach(api, bus, registry, process)
                    window.reveal_start("Selected source passage")
                    capture(output, f"-long-source-{scale}")
                    reveal(window, "More source passage")
                    window.wait(
                        lambda: window.find_text("Source part 1 of", prefix=True),
                        "bounded reading part",
                    )
                    window.reveal_end("Open translation queue")
                    window.focus(window._outer_scrollbar())
                    key("End")
                    time.sleep(0.15)  # Paint the native scroll before capturing lower actions.
                    capture(output, f"-long-source-actions-{scale}")
                    reveal(window, "More source passage")
                    window.act("More source passage")
                    window.wait(
                        lambda: window.find_text("Source part 2 of", prefix=True),
                        "next source reading part",
                    )
                    results[f"bounded_long_source_reading_and_next_part_{scale}"] = True
                    window.reveal_start("Close")
                    window.close_clean()
                    assert process.wait(timeout=5) == 0
                    receipt.unlink()
                    release.unlink()
                    process = subprocess.Popen(
                        base + launcher + ["--route", route("write", view="source")],
                        env=environment,
                        stdout=log,
                        stderr=subprocess.STDOUT,
                    )
                    window = attach(api, bus, registry, process)
                    window.wait(receipt.exists, "cold source link launches without another input")
                    assert json.loads(receipt.read_text()) == [str(source)]
                    window.wait(
                        lambda: window.find_text("Editor launch requested", contains=True),
                        "cold editor launch receipt",
                    )
                    capture(output, f"-source-{scale}")
                    results[f"cold_source_link_resumes_on_worker_readiness_{scale}"] = True
                    release.touch()
                    window.reveal_start("Close")
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
    output.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(results, ensure_ascii=False))
    return "error" not in results
