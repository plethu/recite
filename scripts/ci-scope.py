#!/usr/bin/env python3
"""Select affected PR checks; complete events also build native distributions."""

import argparse
import json
import os
import sys
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
    WORKING_TREE,
    WRITER_DISTRIBUTION,
    shared_config_lanes,
)
from ci_scope_config import JUST_QUALITY as JUST_QUALITY
from ci_scope_inputs import Change as Change
from ci_scope_inputs import changed_files, config_change_lanes, tooling_owners


def lanes_for_path(path, *, base=None, head=None):
    """Keep narrow, known surfaces explicit; new build inputs fail toward more CI."""
    name = Path(path).name
    if path in {
        ".github/renovate.json",
        "turbo.json",
        "scripts/run-checks.py",
        ".github/workflows/cache-cleanup.yml",
    }:
        return frozenset({"maintainability"})
    if (owners := tooling_owners(path)) is not None:
        return owners
    if path == "scripts/check-workflow-performance.sh" or path.startswith("tools/recite-perf/"):
        return frozenset({"workflow-performance", "maintainability"})
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
    if path == "clippy.toml":
        return frozenset({"rust", "maintainability"})
    if path == "scripts/check-dependencies.sh":
        return frozenset({"dependencies", "maintainability"})
    if name in {"Cargo.toml", "Cargo.lock"} or path.startswith(".cargo/"):
        return RUST_BUILD
    if path in {
        "pyproject.toml",
        "uv.lock",
        "scripts/lsp.py",
    } or path.startswith("scripts/lsp_tools/"):
        return frozenset({"benchmark-smoke", "lsp-sessions", "maintainability"})
    if path in {
        "scripts/ci-scope.py",
        "scripts/ci_scope_config.py",
        "scripts/ci_scope_inputs.py",
        "scripts/check-ci-results.py",
        "scripts/setup-quality.py",
    } or path.startswith("tests/ci/"):
        # The unconditional git-policy job runs these contracts on every PR.
        return frozenset({"maintainability"})
    if path in {".config/mise/config.toml", "justfile", ".github/workflows/ci.yml"}:
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
    if path in {"scripts/check-project-gates.sh", "scripts/check-ffi-header.sh"}:
        return RUST
    if path == "scripts/check-zed.sh":
        return frozenset({"editor-native", "maintainability"})
    if path == "scripts/maintainability/exceptions.toml":
        return frozenset({"docs", "maintainability"})
    if path.startswith(ENGINE_COMPANION_PREFIXES):
        return ENGINE | ({"docs"} if path.endswith(".md") else set())
    if path == "docs-site/src/content/docs/getting-started/first-scene.md" or path.startswith(
        "docs-site/src/content/docs/migration/"
    ):
        return frozenset({"docs", "site", "rust"})
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
        lanes = RUST | {"lsp-sessions", "docs", "site"}
        if not path.startswith("crates/recite-config/"):
            lanes |= {"workflow-performance"}
        return lanes
    if path.startswith("crates/"):
        if path.startswith("crates/recite-adapter/"):
            return RUST | {"workflow-performance"}
        if path.startswith("crates/recite-runtime/"):
            return RUST | {"docs", "site", "workflow-performance"}
        if path.startswith("crates/recite-playground/"):
            return RUST | {"docs", "site"}
        if path.startswith("crates/recite-benchmarks/"):
            return RUST | {"workflow-performance"}
        return RUST
    if path.startswith("apps/writer/") or path in {
        "tools/ast-grep/rules/rust-writer-palette.yml",
        "tools/ast-grep/rules/rust-writer-cold-projections.yml",
        "tools/ast-grep/tests/rust-writer-cold-projections-test.yml",
        "tools/ast-grep/tests/rust-writer-palette-test.yml",
        "scripts/check-writer-native-accessibility.py",
        "scripts/writer_accessibility_host.py",
    }:
        lanes = {"writer", "maintainability"}
        if path.startswith("apps/writer/crates/authoring/"):
            lanes.add("workflow-performance")
        return frozenset(lanes)
    if path.startswith(
        (
            "scripts/maintainability/",
            "scripts/quality/",
            "tests/maintainability/",
            "tests/ast-grep/",
            "tests/lint-suppressions/",
            "tools/ast-grep/",
        )
    ) or path in {
        "scripts/check-source-policy.sh",
        "scripts/check-maintainability.sh",
        "scripts/check-lint-suppressions.sh",
        "scripts/check-test-organization.sh",
    }:
        return JUST
    if path.startswith(("editors/vscode/", "tests/editor-hosts/vscode/")):
        return frozenset({"editor", "lsp-sessions", "maintainability"})
    if path.startswith(("editors/helix/", "tests/editor-hosts/helix/")):
        return frozenset({"editor", "maintainability"})
    if path.startswith("editors/recite-tree-sitter/"):
        return frozenset({"writer", "editor-native", "editor", "maintainability"})
    if path.startswith("editors/"):
        return frozenset({"editor-native", "editor", "maintainability"})
    if path.startswith(("fixtures/", "schemas/", "tests/editor-parity/")):
        return LANES - DISTRIBUTION
    if path.startswith(("examples/", "include/", "Packages/")):
        return RUST
    if path in {
        "package.json",
        "bun.lock",
        "bunfig.toml",
        "oxfmt.config.mts",
        "oxlint.config.mts",
        "tsconfig.base.json",
        "tsconfig.node.json",
        "tsconfig.tools.json",
    }:
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
    return LANES


def select_lanes(changes, *, base=None, head=None, reasons=None):
    # Every contribution shares the same source formatting and lint baseline.
    selected = {"maintainability"}
    if reasons is not None:
        reasons["maintainability"] = {"count": 1, "inputs": ["shared formatting and source policy"]}
    for before, after in changes:
        focused = config_change_lanes(before, after, base, head) if base and head else None
        lanes = set()
        if focused is not None:
            lanes.update(focused)
        else:
            for path in {before, after} - {None}:
                lanes.update(lanes_for_path(path, base=base, head=head))
        # Ordinary contributions validate plans; complete events prove installability.
        if lanes & DISTRIBUTION:
            lanes.add("package-config")
        lanes.difference_update(DISTRIBUTION)
        selected.update(lanes)
        if reasons is not None:
            label = (
                f"{before} -> {after}" if before and after and before != after else before or after
            )
            for lane in lanes:
                reason = reasons.setdefault(lane, {"count": 0, "inputs": []})
                reason["count"] += 1
                if len(reason["inputs"]) < 8:
                    reason["inputs"].append(label[:240])
    return {lane: lane in selected for lane in sorted(LANES)}


def event_scope(event_name, event, *, reasons=None):
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
        changed_files(base, head, pull_request=event_name == "pull_request"),
        base=base,
        head=head,
        reasons=reasons,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("base_ref", nargs="?", help="local PR base (defaults to origin/main)")
    parser.add_argument("head_ref", nargs="?", help="local PR head (defaults to HEAD)")
    parser.add_argument("--base", help="inspect a local PR diff instead of the Actions event")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument(
        "--working-tree",
        action="store_true",
        help="include staged, unstaged and untracked source in the local plan",
    )
    parser.add_argument("--explain", action="store_true", help="explain selected lanes on stderr")
    args = parser.parse_args()
    args.base = args.base_ref or args.base
    args.head = args.head_ref or args.head
    if args.working_tree:
        args.base = args.base or "origin/main"
        if args.head != "HEAD":
            parser.error("--working-tree requires --head HEAD")
    reasons = {}
    if args.base:
        scope = select_lanes(
            changed_files(args.base, args.head, pull_request=True, working_tree=args.working_tree),
            base=args.base,
            head=WORKING_TREE if args.working_tree else args.head,
            reasons=reasons,
        )
    else:
        event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
        scope = event_scope(os.environ["GITHUB_EVENT_NAME"], event, reasons=reasons)
    if not reasons:
        reasons = {
            lane: {"count": 1, "inputs": ["explicit complete or diagnostic event"]}
            for lane, run in scope.items()
            if run
        }
    print(json.dumps(scope, indent=2))
    explanation = json.dumps(reasons, indent=2, sort_keys=True)
    if args.explain:
        print(explanation, file=sys.stderr)
    if output := os.environ.get("GITHUB_OUTPUT"):
        with open(output, "a") as stream:
            for lane, selected in scope.items():
                stream.write(f"{lane}={str(selected).lower()}\n")
    if summary := os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(summary, "a") as stream:
            stream.write("## Selected CI coverage\n\n")
            for lane, selected in scope.items():
                stream.write(f"- {lane}: {'run' if selected else 'not affected'}\n")
            stream.write(f"\n## Why these checks?\n\n```json\n{explanation}\n```\n")


if __name__ == "__main__":
    main()
