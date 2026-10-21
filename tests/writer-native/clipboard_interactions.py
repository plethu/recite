"""Native link copying uses the existing clipboard without accepting manuscript drafts."""

import json
import os
import subprocess
import tempfile
import traceback
from pathlib import Path
from urllib.parse import parse_qs, urlencode, urlsplit

from localisation_interactions import reveal, reveal_field
from native_window import attach
from startup_interactions import field, fill
from writer_interactions import SOURCE, capture


def clipboard(previous=None):
    result = subprocess.run(["wl-paste", "--no-newline"], capture_output=True, text=True, timeout=5)
    return result.stdout if result.returncode == 0 and result.stdout != previous else None


def run(api, bus, registry, binary, output):
    results = {"backend": "owned Wayland", "scenario": "clipboard"}
    process, window = None, None
    try:
        with tempfile.TemporaryDirectory(prefix="recite-clipboard-", dir="/tmp") as temporary:
            for scale in [100, 200]:
                project = Path(temporary) / f"Écrivain & notes {scale}"
                project.mkdir()
                (project / "recite.project.toml").write_text("format_version=1\n")
                source = project / "scène & notes.recite"
                source.write_text(SOURCE)
                (project / "fr.po").write_text(
                    'msgctxt "11111111111111111111"\nmsgid "The last tram waits."\nmsgstr "Le dernier tram attend."\n'
                )
                config = project / "config.toml"
                config.write_text(f"config_version=1\n[writer.presentation]\nui_scale={scale}\n")
                environment = dict(os.environ, RECITE_CONFIG=str(config))
                arguments = [str(binary), "--project", str(project), "--document", source.name]
                with output.with_suffix(f".{scale}.writer.log").open("w") as log:
                    process = subprocess.Popen(
                        arguments, env=environment, stdout=log, stderr=subprocess.STDOUT
                    )
                    window = attach(api, bus, registry, process)
                    fill(window, "Manuscript", "Keep this private draft.")
                    window.reveal_start("Copy link to this view")
                    window.act("Copy link to this view")
                    link = window.wait(lambda: clipboard(), "native link clipboard")
                    query = parse_qs(urlsplit(link).query)
                    assert query["scene"] == [source.name]
                    assert query["project"] == [str(project)]
                    assert "private" not in link
                    assert source.read_text() == SOURCE
                    capture(output, f"-writing-{scale}")
                    reveal_field(window, "Manuscript")
                    assert window.text(field(window, "Manuscript")) == "Keep this private draft."
                    results[f"escaped_native_link_preserves_unapplied_prose_{scale}"] = True
                    route = "recite://writer/translations?" + urlencode(
                        {"scene": source.name, "catalogue": "fr.po", "q": "clé & café?"}
                    )
                    forwarded = subprocess.run(
                        [str(binary), "--project", str(project), "--route", route],
                        env=environment,
                        stdout=log,
                        stderr=subprocess.STDOUT,
                        timeout=10,
                    )
                    assert forwarded.returncode == 0
                    reveal(window, "Copy link to this view")
                    window.act("Copy link to this view")
                    copied = window.wait(lambda: clipboard(link), "catalogue location clipboard")
                    location = urlsplit(copied)
                    query = parse_qs(location.query)
                    assert location.path == "/translations"
                    assert query["q"] == ["clé & café?"]
                    assert query["catalogue"] == ["fr.po"]
                    capture(output, f"-catalogue-{scale}")
                    results[f"catalogue_query_and_destination_roundtrip_{scale}"] = True
                    window.reveal_start("Close")
                    window.act("Close")
                    window.wait(
                        lambda: window.find("Close and keep recovery"),
                        "keep accepted unsaved source",
                    )
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
    output.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(results, ensure_ascii=False))
    return "error" not in results
