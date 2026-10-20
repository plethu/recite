#!/usr/bin/env python3
"""Exercise native scanner findings on bounded inputs without network or package installs."""

import json
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def run(command, directory):
    start = time.monotonic()
    result = subprocess.run(command, cwd=directory, capture_output=True, text=True, timeout=10)
    print(f"{command[0]}: {time.monotonic() - start:.3f}s; exit {result.returncode}")
    assert result.returncode != 0, result.stdout + result.stderr
    return result.stdout


with tempfile.TemporaryDirectory() as temporary:
    root = Path(temporary)
    workflow = root / "unsafe.yml"
    # Fixed input: hash pinning and untrusted expression interpolation must both fail.
    workflow.write_text(
        "on: pull_request\npermissions: {}\njobs:\n  probe:\n    runs-on: ubuntu-latest\n"
        "    steps:\n      - uses: actions/checkout@v7\n"
        '      - run: echo "${{ github.event.pull_request.title }}"\n'
    )
    findings = json.loads(
        run(
            [
                "zizmor",
                "--offline",
                "--no-config",
                "--no-progress",
                "--format=json",
                "--min-severity=medium",
                str(workflow),
            ],
            root,
        )
    )
    assert {"template-injection", "unpinned-uses"} <= {item["ident"] for item in findings}

    # Fixed input: retain import reachability instead of declaring every source an entry.
    (root / "package.json").write_text(
        json.dumps(
            {
                "name": "unused-probe",
                "private": True,
                "knip": {"entry": ["index.js"], "project": ["*.js"]},
                "devDependencies": {"left-pad": "1.3.0"},
            }
        )
    )
    (root / "index.js").write_text('import { used } from "./used.js"; console.log(used);\n')
    (root / "used.js").write_text("export const used = 1;\n")
    (root / "unused.js").write_text("export const unused = 1;\n")
    findings = json.loads(
        run(
            [
                shutil.which("bun"),
                str(ROOT / "node_modules/knip/bin/knip.js"),
                "--reporter=json",
                "--no-progress",
            ],
            root,
        )
    )
    issues = findings["issues"]
    assert any(item["file"] == "unused.js" and item["files"] for item in issues), findings
    assert any(dep["name"] == "left-pad" for item in issues for dep in item["devDependencies"]), (
        findings
    )
    assert not any(item["file"] == "used.js" and item["files"] for item in issues), findings
