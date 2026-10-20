#!/usr/bin/env python3
"""Supply check inputs to Turborepo; Turborepo alone owns scheduling and cached results."""

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GROUPS = {
    "fmt": {"rust", "quality"},
    "fmt-rust": {"rust"},
    "fmt-zed": {"rust"},
    "fmt-other": {"quality"},
    "lint-other": {"quality"},
    "spelling": {"spelling"},
    "workflows": {"workflows"},
    "ci-contracts": {"contracts"},
    "quality": {"rust", "quality", "spelling", "workflows"},
}
TOOLS = {
    "rust": {"rust", "just"},
    "quality": {"rust", "node", "bun", "dotnet", "python", "just", "aqua:BurntSushi/ripgrep"},
    "spelling": {"typos"},
    "workflows": {"aqua:rhysd/actionlint", "aqua:zizmorcore/zizmor"},
    "contracts": {"python", "uv", "just", "typos", "aqua:rhysd/actionlint", "aqua:jqlang/jq"},
}


BINARIES = {
    "rust": "rustc",
    "aqua:BurntSushi/ripgrep": "rg",
    "powershell": "pwsh",
    "pipx:gdtoolkit": "gdlint",
    "cargo:https://github.com/kamadorueda/alejandra": "alejandra",
    "github:vercel/turborepo": "turbo",
}


def tool_bound(tool, entry):
    command = BINARIES.get(tool, tool.rsplit(":", 1)[-1].rsplit("/", 1)[-1])
    installation = Path(entry["install_path"]).resolve()
    paths = [
        path
        for path in os.environ.get("PATH", "").split(os.pathsep)
        if Path(path).resolve().is_relative_to(installation)
    ]
    executable = shutil.which(command, path=os.pathsep.join(paths))
    return (
        executable
        if executable and Path(executable).resolve().is_relative_to(installation)
        else None
    )


def output(*command):
    return subprocess.check_output(command, cwd=ROOT).decode().strip()


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def contexts(groups):
    installed = json.loads(output("mise", "ls", "--json", "--current"))
    profile = tomllib.loads((ROOT / ".config/mise/config.quality.toml").read_text())
    common = {
        "platform": [platform.system(), platform.machine(), platform.release()],
        "image": [os.getenv("ImageOS"), os.getenv("ImageVersion")],
        "python": sys.version,
        "mise": output("mise", "--version"),
        "git": output("git", "--version"),
        # Hash values without printing environment contents or absolute paths.
        "environment": {
            key: value
            for key, value in os.environ.items()
            if key.startswith(("RUST", "CARGO_", "DOTNET_", "BUN_", "RUFF_", "LC_"))
            or key in {"LANG", "TZ", "NODE_OPTIONS", "PYTHONPATH", "RECITE_PSSA_VERSION"}
        },
    }
    result = {}
    bindings = {}
    for group in sorted(groups):
        tools = (
            TOOLS[group]
            | {"bun", "github:vercel/turborepo"}
            | set(profile["tools"] if group == "quality" else ())
        )
        versions = {}
        for tool in sorted(tools):
            active = [entry for entry in installed.get(tool, []) if entry["active"]]
            if not active or any(
                not entry["installed"]
                or entry["version"] == "system"
                or not tool_bound(tool, entry)
                for entry in active
            ):
                raise ValueError(
                    f"Install the pinned check tool before using cached results: {tool}"
                )
            versions[tool] = [entry["version"] for entry in active]
            bindings[tool] = tool_bound(tool, active[0])
        context = {**common, "tools": versions}
        if group == "rust":
            context["rustfmt"] = output(
                str(Path(bindings["rust"]).with_name("rustfmt")), "--version"
            )
            context["compiler"] = output(bindings["rust"], "-vV")
        if group == "quality":
            context["discovery"] = output(bindings["aqua:BurntSushi/ripgrep"], "--version")
            context["tracked"] = output("git", "ls-files", "-z", "*justfile", "*.just", "*.gd")
            context["analyzer"] = output(
                bindings["powershell"],
                "-NoProfile",
                "-Command",
                "$ErrorActionPreference='Stop'; Import-Module "
                "./target/quality/powershell/PSScriptAnalyzer/"
                "$env:RECITE_PSSA_VERSION/PSScriptAnalyzer.psd1; "
                "(Get-Module PSScriptAnalyzer).Version.ToString()",
            )
        result[f"RECITE_CHECK_{group.upper()}"] = digest(context)
    directories = dict.fromkeys(str(Path(binary).parent) for binary in bindings.values())
    result["PATH"] = os.pathsep.join([*directories, os.environ.get("PATH", "")])
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", choices=GROUPS)
    parser.add_argument(
        "--force", action="store_true", help="execute checks despite matching results"
    )
    args = parser.parse_args()
    try:
        environment = {**os.environ, **contexts(GROUPS[args.target])}
    except (ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"{error}\n")
    targets = {
        "fmt": ["fmt-rust", "fmt-zed", "fmt-other"],
        "quality": ["fmt-rust", "fmt-zed", "fmt-other", "lint-other", "spelling", "workflows"],
    }.get(args.target, [args.target])
    command = [
        "turbo",
        "run",
        *[f"check:{target}" for target in targets],
        "--filter=//",
        "--cache=local:w" if args.force else "--cache=local:rw",
        "--framework-inference=false",
        "--env-mode=strict",
    ]
    environment["TURBO_TELEMETRY_DISABLED"] = "1"
    environment["TURBO_GLOBAL_WARNING_DISABLED"] = "1"
    os.chdir(ROOT)
    os.execvpe(command[0], command, environment)


if __name__ == "__main__":
    main()
