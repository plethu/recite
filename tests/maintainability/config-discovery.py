#!/usr/bin/env python3
"""Check native Mise profile discovery without installing their build tools."""

import json
import os
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[2]
paths = (
    subprocess.check_output(
        ["git", "ls-files", "-z", "--", ".config/mise/config*.toml"],
        cwd=root,
    )
    .decode()
    .split("\0")
)
for path in filter(None, paths):
    profile = Path(path).stem.removeprefix("config").removeprefix(".")
    args = ["mise", "-E", profile] if profile else ["mise"]
    configs = json.loads(
        subprocess.check_output(
            [*args, "config", "ls", "--json"],
            cwd=root,
            text=True,
            env={**os.environ, "MISE_EXEC_AUTO_INSTALL": "false"},
        )
    )
    if root / path not in {Path(config["path"]).resolve() for config in configs}:
        raise ValueError(f"mise did not discover {path}")
print(f"Native Mise discovery passed for {len(list(filter(None, paths)))} configurations.")
