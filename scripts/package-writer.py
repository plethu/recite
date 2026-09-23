#!/usr/bin/env python3
"""Build a native preview installer with the pinned cargo-packager CLI."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib


ROOT = Path(__file__).resolve().parent.parent
CONFIG = ROOT / "apps/writer/packaging"
PACKAGER_VERSION = "0.11.8"
ABI_SYMBOL = re.compile(r"Name: (GLIBC|GLIBCXX|CXXABI)_([0-9]+(?:\.[0-9]+)+)\b")


def host_platform():
    return {"Linux": "linux", "Darwin": "macos", "Windows": "windows"}.get(platform.system())


def version_parts(version):
    return tuple(int(part) for part in version.split("."))


def linux_abi(binary):
    output = subprocess.check_output(["readelf", "--version-info", str(binary)], text=True)
    required = {family: set() for family in ("GLIBC", "GLIBCXX", "CXXABI")}
    for family, version in ABI_SYMBOL.findall(output):
        required[family].add(version)
    if not required["GLIBC"]:
        raise ValueError(f"no GLIBC symbol requirements found in {binary}")
    with binary.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    return {
        "schemaVersion": 1,
        "binarySha256": digest,
        "minimumGlibc": max(required["GLIBC"], key=version_parts),
        "requiredSymbols": {
            family: [f"{family}_{version}" for version in sorted(versions, key=version_parts)]
            for family, versions in required.items()
        },
    }


def set_linux_runtime_dependency(config, abi):
    depends = config["deb"]["depends"]
    if depends.count("libc6") != 1:
        raise ValueError("Debian template must contain exactly one libc6 placeholder")
    depends[depends.index("libc6")] = f"libc6 (>= {abi['minimumGlibc']})"


def load_config(target, target_dir, output_dir):
    with (CONFIG / "common.json").open(encoding="utf-8") as source:
        config = json.load(source)
    with (CONFIG / f"{target}.json").open(encoding="utf-8") as source:
        config.update(json.load(source))

    with (ROOT / "apps/writer/Cargo.toml").open("rb") as source:
        manifest = tomllib.load(source)
    if config["version"] != manifest["workspace"]["package"]["version"]:
        raise ValueError("packaging preview version differs from writer Cargo.toml")
    if manifest["workspace"]["package"]["license"] != "MIT OR Apache-2.0":
        raise ValueError("writer license differs from package licenses")

    config["licenseFile"] = str(ROOT / config["licenseFile"])
    for resource in config["resources"]:
        resource["src"] = str(ROOT / resource["src"])
    config["icons"] = [str(ROOT / icon) for icon in config["icons"]]
    if "deb" in config:
        config["deb"]["desktopTemplate"] = str(
            ROOT / config["deb"]["desktopTemplate"]
        )
    config["binariesDir"] = str(target_dir / "release")
    config["outDir"] = str(output_dir)
    return config


def stage_smoke_binary(config, target_dir, target, source):
    """Keep local smoke inputs away from Cargo's fingerprinted release output."""
    smoke_dir = target_dir / "writer-package-smoke"
    staged = smoke_dir / ("recite-writer.exe" if target == "windows" else "recite-writer")
    smoke_dir.mkdir(parents=True, exist_ok=True)
    if source != staged:
        shutil.copy2(source, staged)
    config["binariesDir"] = str(smoke_dir)
    return staged


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", choices=("linux", "macos", "windows"))
    parser.add_argument("--target-dir", type=Path)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--binary", type=Path, help="stage an existing native binary for a local package smoke")
    parser.add_argument("--check-config", action="store_true")
    args = parser.parse_args()
    target = args.platform or host_platform()
    if target is None:
        parser.error("unsupported packaging host")
    if not args.check_config and target != host_platform():
        parser.error("native packaging requires the matching host OS")

    target_dir = (args.target_dir or ROOT / "apps/writer/target").resolve()
    output_dir = (args.output_dir or ROOT / "target/writer-packages" / target).resolve()
    config = load_config(target, target_dir, output_dir)
    for path in [config["licenseFile"], *config["icons"]]:
        if not Path(path).is_file():
            parser.error(f"missing packaging input: {path}")
    for resource in config["resources"]:
        if not Path(resource["src"]).is_file():
            parser.error(f"missing packaging input: {resource['src']}")
    if args.check_config:
        print(json.dumps(config, indent=2, sort_keys=True))
        return 0

    version = subprocess.check_output(["cargo", "packager", "--version"], text=True)
    if version.strip() != f"cargo-packager {PACKAGER_VERSION}":
        parser.error(f"cargo-packager {PACKAGER_VERSION} required; found {version.strip()}")
    if args.binary is not None:
        source = args.binary.resolve()
        if not source.is_file():
            parser.error(f"missing smoke binary: {source}")
        binary = stage_smoke_binary(config, target_dir, target, source)
        print(f"staged smoke binary {source}; this package is not a release build")
    else:
        subprocess.run(
            [
                "cargo", "build", "--locked", "--release", "--manifest-path",
                str(ROOT / "apps/writer/Cargo.toml"), "-p", "recite-writer",
                "--target-dir", str(target_dir),
            ],
            cwd=ROOT,
            check=True,
        )
        binary = target_dir / "release" / ("recite-writer.exe" if target == "windows" else "recite-writer")
    if not binary.is_file():
        parser.error(f"missing built writer: {binary}")
    output_dir.mkdir(parents=True, exist_ok=True)
    if target == "linux":
        abi = linux_abi(binary)
        set_linux_runtime_dependency(config, abi)
        (output_dir / "runtime-abi.json").write_text(
            json.dumps(abi, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    with tempfile.TemporaryDirectory(prefix="recite-packager-") as temporary:
        config_path = Path(temporary) / "packager.json"
        config_path.write_text(json.dumps(config), encoding="utf-8")
        environment = os.environ.copy()
        if target == "linux":
            # cargo-packager uses platform cache/config directories while
            # preparing a .deb; keep them beside this isolated build.
            environment["XDG_CACHE_HOME"] = str(target_dir / "packager-cache")
            environment["XDG_CONFIG_HOME"] = str(target_dir / "packager-config")
        subprocess.run(
            ["cargo", "packager", "--config", str(config_path)],
            cwd=ROOT,
            env=environment,
            check=True,
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
