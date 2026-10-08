"""CI lane routing for shared configuration and workflow changes."""

import re
import subprocess
import tomllib

LANES = frozenset(
    {
        "rust",
        "hosts",
        "writer",
        "editor-native",
        "release-plan",
        "windows-publisher",
        "docs",
        "site",
        "editor",
        "benchmark-smoke",
        "maintainability",
        "packages",
        "cli-packages",
        "nix-packages",
        "flatpak-packages",
        "lsp-sessions",
    }
)
WRITER_DISTRIBUTION = frozenset({"packages", "nix-packages", "flatpak-packages"})
DISTRIBUTION = WRITER_DISTRIBUTION | {"cli-packages"}
RUST = frozenset(
    {
        "rust",
        "hosts",
        "writer",
        "editor-native",
        "windows-publisher",
        "benchmark-smoke",
        "editor",
        "maintainability",
        "release-plan",
    }
)
JS = frozenset({"docs", "site", "editor", "lsp-sessions"})
RUST_BUILD = RUST | {"lsp-sessions", "docs", "site"}
JUST = frozenset({"maintainability"})
JUST_QUALITY = LANES - DISTRIBUTION - {"windows-publisher"}
ENGINE = frozenset({"rust", "hosts", "maintainability"})
PACKAGING_PREFIXES = (
    "apps/writer/packaging/",
    "assets/identity/",
    "scripts/package-writer",
    "scripts/check-writer-package",
    "tests/writer-packaging/",
)
ENGINE_COMPANION_PREFIXES = (
    "addons/",
    "Packages/com.recite.dialogue/",
    "examples/godot/",
    "tests/godot-host/",
    "tests/unity-project/",
    "scripts/unity/",
    "scripts/check-godot-host.sh",
    "scripts/package-godot-addon.sh",
    "scripts/check-unity-adapter.sh",
    "scripts/check-bevy-package.sh",
)


def file_at(revision, path):
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return result.stdout if result.returncode == 0 else None


def changed_sections(before, after, pattern):
    """Compare named, top-level blocks in the CI workflow."""
    old_preamble, previous = sections(before, pattern)
    new_preamble, current = sections(after, pattern)
    changed = {
        key for key in previous.keys() | current.keys() if previous.get(key) != current.get(key)
    }
    if old_preamble != new_preamble:
        changed.add("__preamble__")
    return changed


def sections(source, pattern):
    matches = list(re.finditer(pattern, source, re.M))
    preamble = source[: matches[0].start()] if matches else source
    blocks = {
        match.group(1): source[
            match.start() : matches[index + 1].start() if index + 1 < len(matches) else len(source)
        ]
        for index, match in enumerate(matches)
    }
    return preamble, blocks


def lane_wiring_changes(before, after, pattern, fixed):
    """Recognize only exact lane output/needs entries; other edits run every lane."""
    regex = re.compile(pattern)

    def split(source):
        lines = source.splitlines(keepends=True)
        entries = [
            match.group(1) for line in lines if (match := regex.fullmatch(line.rstrip("\n")))
        ]
        remainder = "".join(line for line in lines if not regex.fullmatch(line.rstrip("\n")))
        return entries, remainder

    old_entries, old_remainder = split(before)
    new_entries, new_remainder = split(after)
    if (
        old_remainder != new_remainder
        or len(old_entries) != len(set(old_entries))
        or len(new_entries) != len(set(new_entries))
    ):
        return None
    changed = set(old_entries) ^ set(new_entries)
    if changed & fixed or changed - LANES:
        return None
    return frozenset(changed)


def justfile_lanes(before, after):
    pattern = r"^([a-z_][a-z0-9_-]*)(?: [^:\n]*)?:(?!=)[^\n]*$"

    def without_comments(source):
        return (
            "\n".join(
                line
                for line in source.splitlines()
                if line.strip() and not line.lstrip().startswith("#")
            )
            + "\n"
        )

    before, after = without_comments(before), without_comments(after)
    old_preamble, _ = sections(before, pattern)
    new_preamble, _ = sections(after, pattern)

    def shared_preamble(source):
        return [
            line for line in source.splitlines() if line.strip() and not line.startswith("mod ")
        ]

    if shared_preamble(old_preamble) != shared_preamble(new_preamble):
        return JUST_QUALITY
    changed = changed_sections(before, after, pattern) - {"__preamble__"}
    if changed & {"check", "verify", "_verify"}:
        return JUST_QUALITY
    lanes = set(JUST)
    if changed & {"test", "test-doc", "clippy", "_clippy-rust", "core-check"}:
        lanes.add("rust")
    if "host-check" in changed:
        lanes.add("hosts")
    if "editor-native-check" in changed:
        lanes.add("editor-native")
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
        changed = {
            key
            for key in old_tools.keys() | new_tools.keys()
            if old_tools.get(key) != new_tools.get(key)
        }
        if {key: value for key, value in old.items() if key != "tools"} != {
            key: value for key, value in new.items() if key != "tools"
        }:
            return LANES
        if changed <= {"node", "pnpm"}:
            return JS
        if changed <= {
            "rust",
            "cargo:cbindgen",
            "cargo:cargo-deny",
            "cargo:cargo-machete",
            "cargo:cargo-nextest",
            "tombi",
        }:
            return RUST_BUILD
        return LANES
    if path == "justfile":
        return justfile_lanes(before.decode(), after.decode())
    if path == ".github/workflows/ci.yml":
        old, new = before.decode(), after.decode()

        def prefix(source):
            return source.split("\njobs:\n", 1)[0]

        if prefix(old) != prefix(new) or "\njobs:\n" not in old or "\njobs:\n" not in new:
            return LANES
        pattern = r"^  ([a-z][a-z0-9-]*):\s*$"
        old_jobs = sections(old.split("\njobs:\n", 1)[1], pattern)[1]
        new_jobs = sections(new.split("\njobs:\n", 1)[1], pattern)[1]
        changed = changed_sections(
            old.split("\njobs:\n", 1)[1], new.split("\njobs:\n", 1)[1], pattern
        )
        wiring = set()
        if "changes" in changed:
            lanes = lane_wiring_changes(
                old_jobs.get("changes", ""),
                new_jobs.get("changes", ""),
                r"^      ([a-z][a-z0-9-]*): \$\{\{ steps\.scope\.outputs\.\1 \}\}$",
                set(),
            )
            if lanes is None:
                return LANES
            wiring.update(lanes)
            changed.remove("changes")
        if "required-check" in changed:
            lanes = lane_wiring_changes(
                old_jobs.get("required-check", ""),
                new_jobs.get("required-check", ""),
                r"^      - ([a-z][a-z0-9-]*)$",
                {"changes", "git-policy"},
            )
            if lanes is None:
                return LANES
            wiring.update(lanes)
            changed.remove("required-check")
        if "git-policy" in changed:
            return LANES
        job_lanes = {
            "rust": {"rust"},
            "windows-publisher": {"windows-publisher"},
            "docs": {"docs", "site"},
            "site": {"site"},
            "editor": {"editor"},
            "benchmark-smoke": {"benchmark-smoke"},
            "maintainability": {"maintainability"},
            "packages": {"packages"},
            "cli-packages": {"cli-packages"},
            "nix-packages": {"nix-packages"},
            "flatpak-packages": {"flatpak-packages"},
            "hosts": {"hosts"},
            "writer": {"writer"},
            "editor-native": {"editor-native"},
            "release-plan": {"release-plan"},
            "lsp-sessions": {"lsp-sessions"},
        }
        if changed - job_lanes.keys():
            return LANES
        return frozenset(wiring).union(*(job_lanes[job] for job in changed))
    return LANES
