#!/usr/bin/env python3
"""Inspect built preview bundles without installing them on this machine."""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import plistlib
import platform
import re
import shutil
import subprocess
import sys
import tempfile


PACKAGER_SPEC = importlib.util.spec_from_file_location(
    "package_writer", Path(__file__).with_name("package-writer.py")
)
package_writer = importlib.util.module_from_spec(PACKAGER_SPEC)
PACKAGER_SPEC.loader.exec_module(package_writer)


def one(directory, suffix):
    found = list(directory.rglob(f"*{suffix}"))
    if len(found) != 1:
        raise ValueError(f"expected one {suffix} in {directory}, found {len(found)}")
    return found[0]


def launch_cli(binary):
    for flag, marker in (("--help", "--project"), ("--version", "0.0.0")):
        result = subprocess.run(
            [str(binary), flag], text=True, capture_output=True, timeout=20
        )
        if result.returncode != 0 or marker not in result.stdout:
            raise ValueError(f"{binary} {flag} failed: {result.stderr}")


def check_linux(directory):
    package = one(directory, ".deb")
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        if shutil.which("dpkg-deb"):
            fields = subprocess.check_output(
                ["dpkg-deb", "--field", str(package)], text=True
            )
            subprocess.run(["dpkg-deb", "--extract", str(package), str(root)], check=True)
        else:
            members = subprocess.check_output(["ar", "t", str(package)], text=True).splitlines()
            control = next(member for member in members if member.startswith("control.tar."))
            data = next(member for member in members if member.startswith("data.tar."))
            for member, destination in ((control, root / "control"), (data, root)):
                destination.mkdir(parents=True, exist_ok=True)
                archive = root / member
                archive.write_bytes(subprocess.check_output(["ar", "p", str(package), member]))
                subprocess.run(["bsdtar", "-xf", str(archive), "-C", str(destination)], check=True)
                archive.unlink()
            fields = (root / "control/control").read_text(encoding="utf-8")
        metadata = dict(
            line.split(": ", 1)
            for line in fields.splitlines()
            if not line.startswith(" ") and ": " in line
        )
        if metadata.get("Package") != "recite-writer" or metadata.get("Version") != "0.0.0":
            raise ValueError(f"unexpected Debian metadata: {fields}")
        binary = one(root / "usr/bin", "recite-writer")
        debian_for_library = {
            "libc.so.6": "libc6",
            "libm.so.6": "libc6",
            "ld-linux-x86-64.so.2": "libc6",
            "libgcc_s.so.1": "libgcc-s1",
            "libstdc++.so.6": "libstdc++6",
            "libfreetype.so.6": "libfreetype6",
            "libfontconfig.so.1": "libfontconfig1",
            "libEGL.so.1": "libegl1",
            "libGL.so.1": "libgl1",
        }
        dynamic = subprocess.check_output(["readelf", "-d", str(binary)], text=True)
        needed = set(re.findall(r"Shared library: \[(.+?)\]", dynamic))
        unknown = needed - debian_for_library.keys()
        if unknown:
            raise ValueError(f"review unmapped writer runtime libraries: {sorted(unknown)}")
        declared = {part.strip().split(" (")[0] for part in metadata.get("Depends", "").split(",")}
        # Winit loads this at runtime, so ELF DT_NEEDED cannot discover it.
        runtime_dependencies = {"libxkbcommon-x11-0"}
        missing = ({debian_for_library[library] for library in needed} | runtime_dependencies) - declared
        if missing:
            raise ValueError(f"Debian package lacks runtime dependencies: {sorted(missing)}")
        libc_dependency = next((part.strip() for part in metadata.get("Depends", "").split(",") if part.strip().startswith("libc6")), "")
        minimum = re.fullmatch(r"libc6 \(>= ([0-9]+(?:\.[0-9]+)+)\)", libc_dependency)
        if minimum is None:
            raise ValueError("Debian package lacks a versioned libc6 dependency")
        abi = package_writer.linux_abi(binary)
        if package_writer.version_parts(minimum.group(1)) < package_writer.version_parts(abi["minimumGlibc"]):
            raise ValueError("Debian libc6 minimum is below the packaged ELF requirement")
        recorded = json.loads((directory / "runtime-abi.json").read_text(encoding="utf-8"))
        if recorded != abi:
            raise ValueError("runtime-abi.json does not match the packaged ELF")
        desktop = one(root / "usr/share/applications", ".desktop")
        entry = desktop.read_text(encoding="utf-8")
        if "Exec=" not in entry or "%u" not in entry:
            raise ValueError("desktop entry does not hand over one URL")
        if "MimeType=x-scheme-handler/recite;" not in entry:
            raise ValueError("desktop entry lacks recite:// registration")
        for name in ("LICENSE", "LICENSE-MIT", "LICENSE-APACHE"):
            if not list(root.rglob(name)):
                raise ValueError(f"Debian package lacks {name}")
        launch_cli(binary)


def check_macos(directory, expected_arch):
    bundle = one(directory, ".app")
    dmg = one(directory, ".dmg")
    with tempfile.TemporaryDirectory() as temporary:
        mount = Path(temporary)
        subprocess.run(
            ["hdiutil", "attach", "-nobrowse", "-readonly", "-mountpoint", str(mount), str(dmg)],
            check=True,
            stdin=subprocess.DEVNULL,
        )
        try:
            one(mount, ".app")
        finally:
            subprocess.run(["hdiutil", "detach", str(mount)], check=True)
    plist = plistlib.loads((bundle / "Contents/Info.plist").read_bytes())
    if plist.get("CFBundleIdentifier") != "io.github.plethu.recite":
        raise ValueError("unexpected macOS bundle ID")
    if plist.get("CFBundleURLTypes"):
        raise ValueError("macOS bundle advertises an unimplemented URL-open handler")
    binary = bundle / "Contents/MacOS/recite-writer"
    architectures = subprocess.check_output(["lipo", "-archs", str(binary)], text=True).split()
    if architectures != [expected_arch]:
        raise ValueError(f"expected native {expected_arch} binary, found {architectures}")
    launch_cli(binary)
    for name in ("LICENSE", "LICENSE-MIT", "LICENSE-APACHE"):
        if not list(bundle.rglob(name)):
            raise ValueError(f"macOS bundle lacks {name}")
    subprocess.run(
        ["ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", str(bundle), str(directory / "recite-writer.app.zip")],
        check=True,
    )


def check_windows(directory):
    installer = one(directory, ".exe")
    listing = subprocess.check_output(["7z", "l", str(installer)], text=True)
    for name in ("recite-writer.exe", "LICENSE-MIT", "LICENSE-APACHE"):
        if name not in listing:
            raise ValueError(f"NSIS installer lacks {name}")
    # Disposable Windows CI checks install, same-version reinstall, uninstall,
    # and preservation of an unrelated recite:// association.


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", choices=("linux", "macos", "windows"))
    parser.add_argument("--arch", choices=("x86_64", "arm64"), help="expected native runner and package architecture")
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    host = {"Linux": "linux", "Darwin": "macos", "Windows": "windows"}.get(platform.system())
    target = args.platform or host
    if target is None:
        parser.error("unsupported platform")
    if target != host:
        parser.error("native package inspection requires the matching host OS")
    machine = platform.machine().lower()
    architecture = {"amd64": "x86_64", "aarch64": "arm64"}.get(machine, machine)
    if args.arch and args.arch != architecture:
        parser.error(f"expected {args.arch} runner, found {architecture}")
    directory = args.directory.resolve()
    if target == "macos":
        check_macos(directory, args.arch or architecture)
    else:
        {"linux": check_linux, "windows": check_windows}[target](directory)
    artifacts = [p for p in directory.rglob("*") if p.is_file() and (p.suffix in {".deb", ".dmg", ".exe", ".zip"} or p.name == "runtime-abi.json")]
    with (directory / "SHA256SUMS").open("w", encoding="ascii") as sums:
        for artifact in sorted(artifacts):
            with artifact.open("rb") as source:
                digest = hashlib.file_digest(source, "sha256").hexdigest()
            sums.write(f"{digest}  {artifact.relative_to(directory)}\n")
    print(f"inspected {target} preview package in {directory}")


if __name__ == "__main__":
    sys.exit(main())
