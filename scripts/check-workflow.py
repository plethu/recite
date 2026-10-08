#!/usr/bin/env python3
"""Exercise the documented workflow in a fresh copy of the maintained project."""

import argparse
import json
import queue
import shutil
import subprocess
import tempfile
import threading
import time
from pathlib import Path


def command(recite, root, *args, success=True):
    result = subprocess.run(
        [str(recite), *args], cwd=root, text=True, capture_output=True, timeout=30
    )
    if (result.returncode == 0) != success:
        raise RuntimeError(f"{args}: exit {result.returncode}\n{result.stdout}\n{result.stderr}")
    return result.stdout


def wait_build(records, status):
    deadline = time.monotonic() + 30
    observed = []
    while time.monotonic() < deadline:
        try:
            line = records.get(timeout=max(0.01, deadline - time.monotonic()))
        except queue.Empty as error:
            raise RuntimeError(f"watch did not report {status}: {observed}") from error
        if line is None:
            raise RuntimeError(f"watch ended before {status}: {observed}")
        record = json.loads(line)
        observed.append(record)
        if record["event"] == "watch.build.completed":
            if record["data"]["status"] == status:
                return record
            raise RuntimeError(f"expected {status}, got {record}")
    raise RuntimeError(f"watch did not report {status}: {observed}")


def watch_edits(recite, root):
    records = queue.Queue()
    with tempfile.TemporaryFile(mode="w+") as errors:
        process = subprocess.Popen(
            [str(recite), "watch", ".", "--output-format", "structured"],
            cwd=root,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=errors,
            text=True,
        )

        def collect():
            for line in process.stdout:
                records.put(line)
            records.put(None)

        reader = threading.Thread(target=collect, daemon=True)
        reader.start()
        try:
            wait_build(records, "succeeded")
            command(recite, root, "validate-project", ".")
            asset = root / "build/dialogue.recitec"
            before = asset.read_bytes()
            source = root / "src/arrival.recite"
            original = source.read_text()
            source.write_text(
                original.replace("-> src/effects.recite::effects", "-> missing_block")
            )
            failed = wait_build(records, "failed")
            assert failed["data"]["diagnostics"], "failed build must explain the source error"
            assert asset.read_bytes() == before, "failed build changed last valid asset"
            source.write_text(original.replace("The case narrows", "The case turns"))
            wait_build(records, "succeeded")
            assert asset.read_bytes() != before, "corrected edit did not publish"
            command(recite, root, "check-fresh", ".")
        finally:
            if process.poll() is None:
                process.stdin.write('{"version":1,"command":"watch","action":"cancel"}\n')
                process.stdin.flush()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
            reader.join(timeout=5)
            process.stdin.close()
            process.stdout.close()
            if process.returncode != 0:
                errors.seek(0)
                raise RuntimeError(f"watch exit {process.returncode}: {errors.read()}")


def check(recite, root):
    schema = "schema/realistic.schema.json"
    (root / "build").mkdir()
    command(recite, root, "validate", "src")
    command(recite, root, "check-ids", "src")
    command(recite, root, "check-metadata", "src", "--schema", schema)
    command(recite, root, "check-markup", "src", "--schema", schema)
    command(recite, root, "compile", "src", "--schema", schema, "-o", "build/dialogue.recitec")
    command(recite, root, "extract", "src", "--schema", schema, "-o", "dialogue.pot")
    assert "11111111111111111111" in (root / "dialogue.pot").read_text()
    runtime = ["build/dialogue.recitec", "--block", "arrival", "--fixture", "runtime-fixture.toml"]
    output = command(recite, root, "run", *runtime)
    assert "LACUNA" in output
    first = json.loads(command(recite, root, "trace", *runtime))
    second = json.loads(command(recite, root, "trace", *runtime))
    assert first == second, "identical fixture produced different traces"
    watch_edits(recite, root)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--recite", type=Path, required=True)
    args = parser.parse_args()
    recite = args.recite.resolve(strict=True)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix="recite-workflow-") as scratch:
        root = Path(scratch) / "project"
        shutil.copytree(repo / "fixtures/realistic/v1-pack", root)
        check(recite, root)
    print("Copied-project validation, extraction, deterministic trace and watch recovery passed.")


if __name__ == "__main__":
    main()
