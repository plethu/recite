#!/usr/bin/env python3
"""Inspect a Flatpak export and smoke its headless Writer CLI."""

import argparse
import json
from pathlib import Path
import subprocess
import sys
import tomllib
import xml.etree.ElementTree as ET


APP_ID = "io.github.plethu.recite"
ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "apps/writer/packaging/flatpak" / f"{APP_ID}.yml"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def check_builder_manifest() -> None:
    manifest = json.loads(
        subprocess.check_output(
            ["flatpak-builder", "--show-manifest", str(MANIFEST)],
            text=True,
            timeout=20,
        )
    )
    require(manifest["id"] == APP_ID, "wrong application ID")
    require(manifest["command"] == "recite-writer", "wrong command")
    require(manifest["runtime"] == "org.gnome.Platform", "wrong runtime")
    require(manifest["runtime-version"] == "50", "wrong runtime version")
    require(
        {
            "org.freedesktop.Sdk.Extension.rust-stable",
            "org.freedesktop.Sdk.Extension.llvm22",
        } <= set(manifest["sdk-extensions"]),
        "Rust and LLVM SDK extensions are required",
    )
    finish_args = set(manifest["finish-args"])
    require({
        "--socket=wayland",
        "--socket=fallback-x11",
        "--device=dri",
        "--filesystem=host",
        "--talk-name=org.freedesktop.Flatpak",
    } <= finish_args, "missing sandbox permissions")
    writer = manifest["modules"][-1]
    require(writer["name"] == "recite-writer", "wrong writer module")
    env = writer["build-options"]["env"]
    require(env["CARGO_NET_OFFLINE"] == "true", "Cargo must build offline")
    require(env["SKIA_SOURCE_DIR"] == "/run/build/recite-writer/skia", "Skia must build from pinned source")
    skia = next(
        source
        for source in writer["sources"]
        if isinstance(source, dict) and source.get("dest") == "skia"
    )
    require(skia["commit"] == "41382841f36aa208e7433f3c46d14237a1b15054", "wrong Skia revision")
    externals = {
        source.get("dest"): source.get("commit")
        for source in writer["sources"]
        if isinstance(source, dict)
    }
    require(
        {
            "skia/third_party/externals/wuffs": "e3f919ccfe3ef542cfc983a82146070258fb57f8",
            "skia/third_party/externals/vulkanmemoryallocator": "eb744ea7a2b17040121b4bbb4d6f9e8a77e3cae7",
            "skia/third_party/externals/spirv-cross": "b8fcf307f1f347089e3c46eb4451d27f32ebc8d3",
        }.items() <= externals.items(),
        "missing Skia DEPS revisions",
    )


def check_sources_and_metadata() -> None:
    sources = json.loads((MANIFEST.parent / "cargo-sources.json").read_text())
    cargo_lock = tomllib.loads((ROOT / "apps/writer/Cargo.lock").read_text())
    expected = {
        (
            f"https://static.crates.io/crates/{package['name']}/"
            f"{package['name']}-{package['version']}.crate",
            package["checksum"],
        )
        for package in cargo_lock["package"]
        if package.get("source", "").startswith("registry+")
    }
    actual = {
        (source["url"], source["sha256"])
        for source in sources
        if source["type"] == "archive"
    }
    require(actual == expected, "Cargo sources must match the writer lockfile")

    desktop = (MANIFEST.parent / f"{APP_ID}.desktop").read_text()
    require("Exec=recite-writer %u" in desktop, "desktop entry must accept links")
    require("MimeType=x-scheme-handler/recite;" in desktop, "desktop entry must register recite links")
    metadata = ET.parse(MANIFEST.parent / f"{APP_ID}.metainfo.xml")
    require(metadata.findtext("./id") == APP_ID, "wrong AppStream ID")
    require(metadata.findtext("./launchable") == f"{APP_ID}.desktop", "wrong AppStream launchable")
    cargo_manifest = tomllib.loads((ROOT / "apps/writer/Cargo.toml").read_text())
    release = metadata.find("./releases/release")
    require(
        release is not None
        and release.get("version") == cargo_manifest["workspace"]["package"]["version"],
        "AppStream release must match the Writer workspace version",
    )


def check_export(output: Path) -> None:
    build = output / "build"
    files = build / "files"
    bundle = output / f"{APP_ID}.flatpak"
    require(bundle.is_file() and bundle.stat().st_size > 0, "bundle missing or empty")
    checksum = (output / "SHA256SUMS").read_text().strip()
    require(checksum.endswith(f"  {APP_ID}.flatpak"), "SHA256SUMS must name the relative bundle")
    subprocess.run(
        ["sha256sum", "--check", "SHA256SUMS"],
        cwd=output,
        check=True,
        stdout=subprocess.DEVNULL,
        timeout=30,
    )
    require((files / "bin/recite-writer").is_file(), "writer executable missing")
    for relative in (
        f"share/applications/{APP_ID}.desktop",
        f"share/metainfo/{APP_ID}.metainfo.xml",
        f"share/icons/hicolor/scalable/apps/{APP_ID}.svg",
        f"share/icons/hicolor/512x512/apps/{APP_ID}.png",
        f"share/licenses/{APP_ID}/LICENSE",
        f"share/licenses/{APP_ID}/LICENSE-MIT",
        f"share/licenses/{APP_ID}/LICENSE-APACHE",
    ):
        require((files / relative).is_file(), f"missing {relative}")
    metadata = (build / "metadata").read_text()
    require("org.freedesktop.Flatpak" in metadata, "host spawn permission missing")
    require("x-scheme-handler/recite" in (
        files / f"share/applications/{APP_ID}.desktop"
    ).read_text(), "packaged desktop entry lost link registration")
    for flag in ("--help", "--version"):
        subprocess.run(
            [
                "flatpak",
                "build",
                "--runtime",
                "--readonly",
                "--die-with-parent",
                str(build),
                "/app/bin/recite-writer",
                flag,
            ],
            check=True,
            stdout=subprocess.DEVNULL,
            timeout=20,
        )
    literal = "Recite host argv: spaces; $(literal)"
    result = subprocess.check_output(
        [
            "dbus-run-session",
            "--",
            "flatpak",
            "build",
            "--runtime",
            "--readonly",
            "--die-with-parent",
            "--talk-name=org.freedesktop.Flatpak",
            str(build),
            "/usr/bin/flatpak-spawn",
            "--host",
            "--watch-bus",
            "--directory=/tmp",
            "--",
            "/usr/bin/printf",
            "%s",
            literal,
        ],
        text=True,
        timeout=20,
    )
    require(result == literal, "host bridge altered literal argv")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", nargs="?", type=Path)
    parser.add_argument("--builder", action="store_true", help="also parse with flatpak-builder")
    args = parser.parse_args()
    check_sources_and_metadata()
    if args.builder or args.output is not None:
        check_builder_manifest()
    if args.output is not None:
        check_export(args.output)
    print("Writer Flatpak checks passed")


if __name__ == "__main__":
    try:
        main()
    except (
        ValueError,
        KeyError,
        StopIteration,
        OSError,
        subprocess.CalledProcessError,
        subprocess.TimeoutExpired,
        ET.ParseError,
    ) as exc:
        print(f"Writer Flatpak check failed: {exc}", file=sys.stderr)
        sys.exit(1)
