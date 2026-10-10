#!/usr/bin/env python3
"""Select CI lanes from a complete Git diff; unknown paths receive full coverage."""

import argparse
import json
import os
import subprocess
from pathlib import Path

from ci_scope_config import (
    DISTRIBUTION,
    ENGINE,
    ENGINE_COMPANION_PREFIXES,
    JS,
    JUST,
    LANES,
    PACKAGING_PREFIXES,
    RUST,
    RUST_BUILD,
    WRITER_DISTRIBUTION,
    shared_config_lanes,
)
from ci_scope_config import JUST_QUALITY as JUST_QUALITY


def lanes_for_path(path, *, base=None, head=None):
    """Keep narrow, known surfaces explicit; new build inputs fail toward more CI."""
    name = Path(path).name
    if path.startswith("nix/") or path in {"flake.nix", "flake.lock"}:
        return frozenset({"nix-packages", "maintainability", "docs"})
    if path.startswith(("apps/writer/packaging/flatpak/", "tests/writer-flatpak/")) or path in {
        "scripts/check-writer-flatpak.py",
        "scripts/package-writer-flatpak.sh",
        "apps/writer/packaging/icons/recite-writer.svg",
    }:
        return frozenset({"flatpak-packages", "maintainability", "docs"})
    if path in {
        "dist-workspace.toml",
        "mise.release.toml",
        "release.just",
        ".github/workflows/cli-packages.yml",
    }:
        return frozenset({"cli-packages", "rust", "release-plan", "maintainability", "docs"})
    if path in {
        "release.toml",
        ".github/workflows/release.yml",
        ".github/workflows/publish-release.yml",
    } or path.startswith("tools/recite-release/"):
        return frozenset({"rust", "release-plan", "maintainability", "docs"})
    if path == ".github/workflows/rust-checks.yml":
        return frozenset({"rust", "hosts", "writer", "editor-native", "maintainability"})
    if path == ".github/workflows/writer-packages.yml":
        return WRITER_DISTRIBUTION | {"maintainability", "docs"}
    if path in {"LICENSE", "LICENSE-MIT", "LICENSE-APACHE"}:
        return DISTRIBUTION | {"maintainability", "docs"}
    if path == "apps/writer/packaging/icons/recite-writer.png":
        return WRITER_DISTRIBUTION | {"writer", "maintainability", "docs"}
    if path == ".github/workflows/lsp-sessions.yml" or path.startswith(
        (
            "scripts/lsp_session",
            "scripts/lsp-session-",
            "scripts/measure-lsp-endurance",
            "scripts/check-lsp-session-",
            "scripts/summarize-lsp-native-trace",
            "scripts/measure-lsp-channel-handoff",
            "scripts/measure-lsp-stdio-transport",
            "scripts/measure-lsp-driver-accounting",
            "scripts/measure-lsp-stopped-responses",
            "scripts/summarize-lsp-server-comparison",
        )
    ):
        return frozenset({"lsp-sessions", "maintainability"})
    if path in {
        "scripts/measure-lsp-latency.py",
        "scripts/lsp_measurement.py",
        "scripts/lsp_fanout.py",
    }:
        return frozenset({"lsp-sessions", "benchmark-smoke", "maintainability"})
    if name in {"Cargo.toml", "Cargo.lock"} or path.startswith(".cargo/"):
        return RUST_BUILD
    if path in {
        "perf.just",
        "mise.lsp.toml",
        "pyproject.toml",
        "uv.lock",
        "scripts/lsp.py",
    } or path.startswith("scripts/lsp_tools/"):
        return frozenset({"benchmark-smoke", "lsp-sessions", "maintainability"})
    if path in {"mise.maintainability.toml", "mise.godot.toml"}:
        return RUST
    if path in {
        "scripts/ci-scope.py",
        "scripts/ci_scope_config.py",
        "scripts/check-ci-results.py",
    } or path.startswith("tests/ci/"):
        # The unconditional git-policy job runs these contracts on every PR.
        return frozenset({"maintainability"})
    if path in {".mise.toml", "justfile", ".github/workflows/ci.yml"}:
        return (
            shared_config_lanes(path, base, head)
            if base and head
            else (JUST if path == "justfile" else LANES)
        )
    if path == ".gitignore":
        # Mutation sandboxes use ignore rules when copying sources and tests.
        return frozenset({"rust"})
    if path == "apps/writer/justfile":
        return frozenset({"writer", "maintainability"})
    if path == "editors/justfile":
        return frozenset({"rust", "editor-native", "maintainability"})
    if path == "editors/zed/justfile":
        return frozenset({"editor-native", "maintainability"})
    if path == "engines.just":
        return frozenset({"hosts", "maintainability"})
    if path == "stress.just":
        return JUST
    if path in {"scripts/check-project-gates.sh", "scripts/check-ffi-header.sh"}:
        return RUST
    if path == "scripts/check-zed.sh":
        return frozenset({"editor-native", "maintainability"})
    if path == "scripts/maintainability/exceptions.toml":
        return frozenset({"docs", "maintainability"})
    if path.startswith(ENGINE_COMPANION_PREFIXES):
        return ENGINE | ({"docs"} if path.endswith(".md") else set())
    if path.startswith("docs-site/"):
        if path == "docs-site/README.md":
            return frozenset({"docs"})
        lanes = {"docs", "site"}
        if path.endswith((".js", ".mjs", ".cjs", ".py", ".sh")):
            lanes.add("maintainability")
        return frozenset(lanes)
    if path.startswith("docs/") or (
        path.endswith(".md") and not path.startswith(("fixtures/", "tests/"))
    ):
        return frozenset({"docs"})
    if path in {
        "assets/identity/recite-wordmark.svg",
        "assets/identity/recite-wordmark-reversed.svg",
    }:
        return WRITER_DISTRIBUTION | {"writer", "maintainability", "docs", "site"}
    if path.startswith(PACKAGING_PREFIXES) or path in {
        "mise.packaging.toml",
        "scripts/check-writer-desktop-links.py",
    }:
        return frozenset({"packages", "maintainability", "docs"})
    if path.startswith(
        tuple(
            f"crates/recite-{crate}/"
            for crate in (
                "lsp",
                "compiler",
                "parser",
                "core",
                "config",
                "schema",
                "ui",
                "fixturegen",
            )
        )
    ):
        return RUST | {"lsp-sessions", "docs", "site"}
    if path.startswith("crates/"):
        if path.startswith(("crates/recite-playground/", "crates/recite-runtime/")):
            return RUST | {"docs", "site"}
        return RUST
    if path.startswith("apps/writer/") or path in {
        "scripts/check-writer-colors.py",
        "scripts/check-writer-native-accessibility.py",
    }:
        return frozenset({"writer", "maintainability"})
    if path.startswith(("editors/vscode/", "tests/editor-hosts/vscode/")):
        return frozenset({"editor", "lsp-sessions", "maintainability"})
    if path.startswith(("editors/helix/", "tests/editor-hosts/helix/")):
        return frozenset({"editor", "maintainability"})
    if path.startswith("editors/"):
        return frozenset({"editor-native", "editor", "maintainability"})
    if path.startswith(("fixtures/", "schemas/", "tests/editor-parity/")):
        return LANES - DISTRIBUTION
    if path.startswith(("examples/", "include/", "Packages/")):
        return RUST
    if path in {"package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml"}:
        return JS
    if path == "scripts/install-js-dependencies.sh":
        return JS | {"maintainability"}
    if path in {
        "scripts/check-docs.sh",
        "scripts/check-schema-manifest.mjs",
        "scripts/check-site-links.py",
    }:
        return frozenset({"docs", "site", "maintainability"})
    if path in {"scripts/check-vscode.sh", "scripts/check-helix.sh"}:
        return frozenset({"editor", "maintainability"})
    if path == "scripts/benchmark-smoke.sh" or path.startswith(
        (
            "scripts/check-lsp-performance.",
            "scripts/measure-lsp-",
            "scripts/lsp_",
            "scripts/lsp-performance-",
        )
    ):
        return frozenset({"benchmark-smoke", "maintainability"})
    if path in {"_typos.toml", "tombi.toml", "clippy.toml", "deny.toml"}:
        return frozenset({"rust", "maintainability"})
    return LANES


def select_lanes(paths, *, base=None, head=None):
    # Every contribution shares the same source formatting and lint baseline.
    selected = {"maintainability"}
    for path in paths:
        selected.update(lanes_for_path(path, base=base, head=head))
    return {lane: lane in selected for lane in sorted(LANES)}


def changed_paths(base, head, *, pull_request):
    # Disable rename detection: both the old and new paths must select coverage.
    # NUL delimiters preserve spaces/newlines; no API pagination or path-filter cap.
    revision = f"{base}...{head}" if pull_request else f"{base}..{head}"
    result = subprocess.run(
        ["git", "diff", "--name-only", "--no-renames", "-z", revision, "--"],
        check=True,
        stdout=subprocess.PIPE,
    )
    return [os.fsdecode(path) for path in result.stdout.split(b"\0") if path]


def event_scope(event_name, event):
    if event_name == "workflow_dispatch" and event.get("inputs", {}).get("lsp_sessions_only") in (
        True,
        "true",
    ):
        return {lane: lane == "lsp-sessions" for lane in sorted(LANES)}
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
    return select_lanes(
        changed_paths(base, head, pull_request=event_name == "pull_request"), base=base, head=head
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", help="inspect a local PR diff instead of the Actions event")
    parser.add_argument("--head", default="HEAD")
    args = parser.parse_args()
    if args.base:
        scope = select_lanes(
            changed_paths(args.base, args.head, pull_request=True), base=args.base, head=args.head
        )
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
