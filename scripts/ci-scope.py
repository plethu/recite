#!/usr/bin/env python3
"""Select CI lanes from a complete Git diff; unknown paths receive full coverage."""

import argparse
import json
import os
from pathlib import Path
import subprocess


LANES = frozenset({
    "rust", "windows-publisher", "docs", "editor", "benchmark-smoke",
    "maintainability", "packages",
})
RUST = frozenset({"rust", "windows-publisher", "benchmark-smoke", "editor", "maintainability"})
PACKAGING_PREFIXES = (
    "apps/writer/packaging/", "assets/identity/", "nix/",
    "scripts/package-writer", "scripts/check-writer-package",
    "scripts/check-writer-flatpak", "tests/writer-packaging/", "tests/writer-flatpak/",
)


def lanes_for_path(path):
    """Keep narrow, known surfaces explicit; new build inputs fail toward more CI."""
    name = Path(path).name
    if name in {"Cargo.toml", "Cargo.lock"} or path in {
        ".mise.toml", "mise.maintainability.toml", "mise.godot.toml", "justfile",
        ".github/workflows/ci.yml", "scripts/ci-scope.py", "scripts/check-ci-results.py",
    } or path.startswith((".cargo/", "tests/ci/")):
        return LANES
    if path == "docs/maintainability-baseline.md":
        return frozenset({"docs", "maintainability"})
    if path.startswith("docs/") or (
        path.endswith(".md") and not path.startswith(("fixtures/", "tests/"))
    ):
        return frozenset({"docs"})
    if path.startswith(PACKAGING_PREFIXES) or name.startswith("LICENSE") or path in {
        "flake.nix", "flake.lock", ".github/workflows/writer-packages.yml",
        "scripts/check-writer-desktop-links.py",
    }:
        return frozenset({"packages", "maintainability", "docs"})
    if path.startswith("docs-site/"):
        return frozenset({"docs", "maintainability"})
    if path.startswith("crates/"):
        return RUST
    if path.startswith("apps/writer/") or path in {
        "scripts/check-writer-colors.py", "scripts/check-writer-native-accessibility.py",
    }:
        return frozenset({"rust", "maintainability"})
    if path.startswith(("editors/vscode/", "editors/helix/", "tests/editor-hosts/helix/")):
        return frozenset({"editor", "maintainability"})
    if path.startswith("editors/"):
        return frozenset({"rust", "editor", "maintainability"})
    if path.startswith(("fixtures/", "schemas/", "tests/editor-parity/")):
        return LANES - {"packages"}
    if path.startswith(("examples/", "include/", "Packages/")):
        return RUST
    if path in {"package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml",
                "scripts/install-js-dependencies.sh"}:
        return frozenset({"rust", "docs", "editor", "maintainability"})
    if path in {"scripts/check-docs.sh", "scripts/check-schema-manifest.mjs"}:
        return frozenset({"docs", "maintainability"})
    if path in {"scripts/check-vscode.sh", "scripts/check-helix.sh"}:
        return frozenset({"editor", "maintainability"})
    if path == "scripts/benchmark-smoke.sh":
        return frozenset({"benchmark-smoke", "maintainability"})
    if path in {"_typos.toml", "taplo.toml", "clippy.toml", "deny.toml"}:
        return frozenset({"rust", "maintainability"})
    return LANES


def select_lanes(paths):
    selected = set()
    for path in paths:
        selected.update(lanes_for_path(path))
    return {lane: lane in selected for lane in sorted(LANES)}


def changed_paths(base, head, *, pull_request):
    # Disable rename detection: both the old and new paths must select coverage.
    # NUL delimiters preserve spaces/newlines; no API pagination or path-filter cap.
    revision = f"{base}...{head}" if pull_request else f"{base}..{head}"
    result = subprocess.run(
        ["git", "diff", "--name-only", "--no-renames", "-z", revision, "--"],
        check=True, stdout=subprocess.PIPE,
    )
    return [os.fsdecode(path) for path in result.stdout.split(b"\0") if path]


def event_scope(event_name, event):
    if event_name in {"workflow_dispatch", "schedule"}:
        return {lane: True for lane in sorted(LANES)}
    if event_name == "pull_request":
        base = event["pull_request"]["base"]["sha"]
        head = event["pull_request"]["head"]["sha"]
    elif event_name == "push":
        base, head = event["before"], event["after"]
        if base == "0" * 40:
            return {lane: True for lane in sorted(LANES)}
    else:
        raise ValueError(f"unsupported CI event: {event_name}")
    if not base or not head:
        raise ValueError("CI diff requires both commit references")
    return select_lanes(changed_paths(base, head, pull_request=event_name == "pull_request"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", help="inspect a local PR diff instead of the Actions event")
    parser.add_argument("--head", default="HEAD")
    args = parser.parse_args()
    if args.base:
        scope = select_lanes(changed_paths(args.base, args.head, pull_request=True))
    else:
        event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
        scope = event_scope(os.environ["GITHUB_EVENT_NAME"], event)
    print(json.dumps(scope, indent=2))
    if output := os.environ.get("GITHUB_OUTPUT"):
        with open(output, "a") as stream:
            for lane, selected in scope.items():
                stream.write(f"{lane}={str(selected).lower()}\n")
    if summary := os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(summary, "a") as stream:
            stream.write("## Selected CI coverage\n\n")
            for lane, selected in scope.items():
                stream.write(f"- {lane}: {'run' if selected else 'not affected'}\n")


if __name__ == "__main__":
    main()
