#!/usr/bin/env python3
"""Select CI lanes from a complete Git diff; unknown paths receive full coverage."""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib


LANES = frozenset({
    "rust", "windows-publisher", "docs", "site", "editor", "benchmark-smoke",
    "maintainability", "packages", "lsp-sessions",
})
RUST = frozenset({"rust", "windows-publisher", "benchmark-smoke", "editor", "maintainability"})
JS = frozenset({"docs", "site", "editor", "lsp-sessions"})
RUST_BUILD = RUST | {"packages", "lsp-sessions"}
JUST = frozenset({"maintainability"})
JUST_QUALITY = LANES - {"windows-publisher", "packages"}
ENGINE = frozenset({"rust", "maintainability"})
PACKAGING_PREFIXES = (
    "apps/writer/packaging/", "assets/identity/", "nix/",
    "scripts/package-writer", "scripts/check-writer-package",
    "scripts/check-writer-flatpak", "tests/writer-packaging/", "tests/writer-flatpak/",
)
ENGINE_COMPANION_PREFIXES = (
    "addons/", "Packages/com.recite.dialogue/", "examples/godot/",
    "tests/godot-host/", "tests/unity-project/", "scripts/unity/",
    "scripts/check-godot-host.sh", "scripts/package-godot-addon.sh",
    "scripts/check-unity-adapter.sh", "scripts/check-bevy-package.sh",
)


def file_at(revision, path):
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return result.stdout if result.returncode == 0 else None


def changed_sections(before, after, pattern):
    """Compare named, top-level blocks in the CI workflow."""
    old_preamble, previous = sections(before, pattern)
    new_preamble, current = sections(after, pattern)
    changed = {key for key in previous.keys() | current.keys()
               if previous.get(key) != current.get(key)}
    if old_preamble != new_preamble:
        changed.add("__preamble__")
    return changed


def sections(source, pattern):
    matches = list(re.finditer(pattern, source, re.M))
    preamble = source[:matches[0].start()] if matches else source
    blocks = {match.group(1): source[match.start():
              matches[index + 1].start() if index + 1 < len(matches) else len(source)]
              for index, match in enumerate(matches)}
    return preamble, blocks


def lane_wiring_changes(before, after, pattern, fixed):
    """Recognize only exact lane output/needs entries; other edits run every lane."""
    regex = re.compile(pattern)

    def split(source):
        lines = source.splitlines(keepends=True)
        entries = [match.group(1) for line in lines if (match := regex.fullmatch(line.rstrip("\n")))]
        remainder = "".join(line for line in lines if not regex.fullmatch(line.rstrip("\n")))
        return entries, remainder

    old_entries, old_remainder = split(before)
    new_entries, new_remainder = split(after)
    if (old_remainder != new_remainder or len(old_entries) != len(set(old_entries))
            or len(new_entries) != len(set(new_entries))):
        return None
    changed = set(old_entries) ^ set(new_entries)
    if changed & fixed or changed - LANES:
        return None
    return frozenset(changed)


def justfile_lanes(before, after):
    pattern = r"^([a-z][a-z0-9-]*)(?: [^:\n]*)?:(?!=)[^\n]*$"

    def without_comments(source):
        return "\n".join(line for line in source.splitlines() if line.strip()
                         and not line.lstrip().startswith("#")) + "\n"

    before, after = without_comments(before), without_comments(after)
    old_preamble, _ = sections(before, pattern)
    new_preamble, _ = sections(after, pattern)

    def shared_preamble(source):
        return [line for line in source.splitlines() if line.strip()
                and not line.startswith("mod ")]

    if shared_preamble(old_preamble) != shared_preamble(new_preamble):
        return JUST_QUALITY
    changed = changed_sections(before, after, pattern) - {"__preamble__"}
    if changed & {"check", "verify"}:
        return JUST_QUALITY
    lanes = set(JUST)
    if changed & {"test", "test-doc", "clippy"}:
        lanes.add("rust")
    if "setup" in changed:
        lanes.update(JS)
    return frozenset(lanes)


def shared_config_lanes(path, base, head):
    before, after = file_at(base, path), file_at(head, path)
    if before is None or after is None:
        return LANES
    if path == ".mise.toml":
        old, new = tomllib.loads(before.decode()), tomllib.loads(after.decode())
        old_tools, new_tools = old.get("tools"), new.get("tools")
        if not isinstance(old_tools, dict) or not isinstance(new_tools, dict):
            return LANES
        changed = {key for key in old_tools.keys() | new_tools.keys()
                   if old_tools.get(key) != new_tools.get(key)}
        if {key: value for key, value in old.items() if key != "tools"} != {
            key: value for key, value in new.items() if key != "tools"
        }:
            return LANES
        if changed <= {"node", "pnpm"}:
            return JS
        if changed <= {"rust", "cargo:cbindgen", "cargo:cargo-deny",
                       "cargo:cargo-machete", "cargo:cargo-nextest", "taplo"}:
            return RUST_BUILD
        return LANES
    if path == "justfile":
        return justfile_lanes(before.decode(), after.decode())
    if path == ".github/workflows/ci.yml":
        old, new = before.decode(), after.decode()
        prefix = lambda source: source.split("\njobs:\n", 1)[0]
        if prefix(old) != prefix(new) or "\njobs:\n" not in old or "\njobs:\n" not in new:
            return LANES
        pattern = r"^  ([a-z][a-z0-9-]*):\s*$"
        old_jobs = sections(old.split("\njobs:\n", 1)[1], pattern)[1]
        new_jobs = sections(new.split("\njobs:\n", 1)[1], pattern)[1]
        changed = changed_sections(old.split("\njobs:\n", 1)[1],
                                   new.split("\njobs:\n", 1)[1], pattern)
        wiring = set()
        if "changes" in changed:
            lanes = lane_wiring_changes(
                old_jobs.get("changes", ""), new_jobs.get("changes", ""),
                r"^      ([a-z][a-z0-9-]*): \$\{\{ steps\.scope\.outputs\.\1 \}\}$", set(),
            )
            if lanes is None:
                return LANES
            wiring.update(lanes)
            changed.remove("changes")
        if "required-check" in changed:
            lanes = lane_wiring_changes(
                old_jobs.get("required-check", ""), new_jobs.get("required-check", ""),
                r"^      - ([a-z][a-z0-9-]*)$", {"changes", "git-policy"},
            )
            if lanes is None:
                return LANES
            wiring.update(lanes)
            changed.remove("required-check")
        if "git-policy" in changed:
            return LANES
        job_lanes = {"rust": {"rust"}, "windows-publisher": {"windows-publisher"},
                     "docs": {"docs", "site"}, "site": {"site"},
                     "editor": {"editor"}, "benchmark-smoke": {"benchmark-smoke"},
                     "maintainability": {"maintainability"}, "packages": {"packages"},
                     "lsp-sessions": {"lsp-sessions"}}
        if changed - job_lanes.keys():
            return LANES
        return frozenset(wiring).union(*(job_lanes[job] for job in changed))
    return LANES


def lanes_for_path(path, *, base=None, head=None):
    """Keep narrow, known surfaces explicit; new build inputs fail toward more CI."""
    name = Path(path).name
    if path == ".github/workflows/lsp-sessions.yml" or path.startswith((
        "scripts/lsp_session", "scripts/lsp-session-", "scripts/measure-lsp-endurance",
        "scripts/check-lsp-session-", "scripts/summarize-lsp-native-trace",
        "scripts/measure-lsp-channel-handoff", "scripts/summarize-lsp-server-comparison",
    )):
        return frozenset({"lsp-sessions", "maintainability"})
    if path in {"scripts/measure-lsp-latency.py", "scripts/lsp_measurement.py", "scripts/lsp_fanout.py"}:
        return frozenset({"lsp-sessions", "benchmark-smoke", "maintainability"})
    if name in {"Cargo.toml", "Cargo.lock"} or path.startswith(".cargo/"):
        return RUST_BUILD
    if path in {"mise.maintainability.toml", "mise.godot.toml"}:
        return RUST
    if path in {"scripts/ci-scope.py", "scripts/check-ci-results.py"} or path.startswith("tests/ci/"):
        # The unconditional git-policy job runs these contracts on every PR.
        return frozenset({"maintainability"})
    if path in {".mise.toml", "justfile", ".github/workflows/ci.yml"}:
        return shared_config_lanes(path, base, head) if base and head else (
            JUST if path == "justfile" else LANES
        )
    if path == ".gitignore":
        return frozenset()
    if path in {"apps/writer/justfile", "editors/justfile", "editors/zed/justfile",
                "engines.just", "stress.just"}:
        return JUST
    if path in {"scripts/check-project-gates.sh", "scripts/check-ffi-header.sh"}:
        return frozenset({"rust", "maintainability"})
    if path == "scripts/check-zed.sh":
        return frozenset({"rust", "editor", "maintainability"})
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
    if path in {"assets/identity/recite-wordmark.svg",
                "assets/identity/recite-wordmark-reversed.svg"}:
        return frozenset({"packages", "maintainability", "docs", "site"})
    if path.startswith(PACKAGING_PREFIXES) or name.startswith("LICENSE") or path in {
        "flake.nix", "flake.lock", ".github/workflows/writer-packages.yml",
        "scripts/check-writer-desktop-links.py",
    }:
        return frozenset({"packages", "maintainability", "docs"})
    if path.startswith(tuple(f"crates/recite-{crate}/" for crate in (
        "lsp", "compiler", "parser", "core", "config", "schema", "ui", "fixturegen",
    ))):
        return RUST | {"lsp-sessions"}
    if path.startswith("crates/"):
        return RUST
    if path.startswith("apps/writer/") or path in {
        "scripts/check-writer-colors.py", "scripts/check-writer-native-accessibility.py",
    }:
        return frozenset({"rust", "maintainability"})
    if path.startswith(("editors/vscode/", "tests/editor-hosts/vscode/")):
        return frozenset({"editor", "lsp-sessions", "maintainability"})
    if path.startswith(("editors/helix/", "tests/editor-hosts/helix/")):
        return frozenset({"editor", "maintainability"})
    if path.startswith("editors/"):
        return frozenset({"rust", "editor", "maintainability"})
    if path.startswith(("fixtures/", "schemas/", "tests/editor-parity/")):
        return LANES - {"packages"}
    if path.startswith(("examples/", "include/", "Packages/")):
        return RUST
    if path in {"package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml"}:
        return JS
    if path == "scripts/install-js-dependencies.sh":
        return JS | {"maintainability"}
    if path in {"scripts/check-docs.sh", "scripts/check-schema-manifest.mjs",
                "scripts/check-site-links.py"}:
        return frozenset({"docs", "site", "maintainability"})
    if path in {"scripts/check-vscode.sh", "scripts/check-helix.sh"}:
        return frozenset({"editor", "maintainability"})
    if path == "scripts/benchmark-smoke.sh" or path.startswith((
        "scripts/check-lsp-performance.", "scripts/measure-lsp-", "scripts/lsp_",
        "scripts/lsp-performance-",
    )):
        return frozenset({"benchmark-smoke", "maintainability"})
    if path in {"_typos.toml", "taplo.toml", "clippy.toml", "deny.toml"}:
        return frozenset({"rust", "maintainability"})
    return LANES


def select_lanes(paths, *, base=None, head=None):
    selected = set()
    for path in paths:
        selected.update(lanes_for_path(path, base=base, head=head))
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
    if event_name == "workflow_dispatch" and event.get("inputs", {}).get("lsp_sessions_only") in (True, "true"):
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
    return select_lanes(changed_paths(base, head, pull_request=event_name == "pull_request"),
                        base=base, head=head)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", help="inspect a local PR diff instead of the Actions event")
    parser.add_argument("--head", default="HEAD")
    args = parser.parse_args()
    if args.base:
        scope = select_lanes(changed_paths(args.base, args.head, pull_request=True),
                             base=args.base, head=args.head)
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
