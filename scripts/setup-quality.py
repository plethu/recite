#!/usr/bin/env python3
"""Install the quality profile without provisioning unrelated build/test tools."""

import json
import os
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parent.parent
configs = json.loads(
    subprocess.check_output(
        ["mise", "-E", "quality", "config", "ls", "--json"],
        cwd=root,
        text=True,
    )
)
profiles = [
    config
    for config in configs
    if Path(config["path"]).resolve() == root / ".config/mise/config.quality.toml"
]
if len(profiles) != 1 or not profiles[0]["tools"]:
    raise ValueError("mise did not discover the repository quality profile and its tools")
tools = profiles[0]["tools"]
subprocess.run(["mise", "-E", "quality", "install", *tools], cwd=root, check=True)
subprocess.run(
    ["mise", "-E", "quality", "run", "quality-powershell-setup"],
    cwd=root,
    env={**os.environ, "MISE_EXEC_AUTO_INSTALL": "false"},
    check=True,
)
