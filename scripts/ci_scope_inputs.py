"""Semantic routing for tool configuration, manifests and command modules."""

import ast
import os
import posixpath
import re
import subprocess
import tomllib
from pathlib import PurePosixPath
from typing import NamedTuple

from ci_scope_config import (
    JUST,
    LANES,
    RUST_BUILD,
    file_at,
    justfile_lanes,
    mise_lanes,
)

MISE_PROFILES = {
    "quality": JUST,
    "maintainability": JUST,
    "godot": frozenset({"hosts", "maintainability"}),
    "lsp": frozenset({"benchmark-smoke", "lsp-sessions", "maintainability"}),
    "perf": frozenset({"workflow-performance", "maintainability"}),
    "site": frozenset({"docs", "site", "maintainability"}),
    "release": frozenset({"rust", "release-plan", "cli-packages", "maintainability", "docs"}),
    "packaging": frozenset({"packages", "maintainability", "docs"}),
}
JUST_MODULES = {
    "quality.just": JUST,
    "engines.just": frozenset({"hosts", "maintainability"}),
    "stress.just": frozenset({"workflow-performance", "maintainability"}),
    "perf.just": frozenset(
        {"benchmark-smoke", "lsp-sessions", "workflow-performance", "maintainability"}
    ),
    "release.just": frozenset({"rust", "release-plan", "cli-packages", "maintainability", "docs"}),
}
LOCAL_MISE = {
    ".mise.local.toml",
    "mise.local.toml",
    "/.config/mise/config.local.toml",
    "/.config/mise/config.*.local.toml",
}


def config_kind(path):
    if path is None:
        return None
    item = PurePosixPath(path)
    if path in {".mise.toml", "mise.toml", ".config/mise/config.toml"}:
        return "mise", ""
    if item.parent == PurePosixPath("."):
        match = re.fullmatch(r"mise\.([a-z-]+)\.toml", item.name)
    elif item.parent == PurePosixPath(".config/mise"):
        match = re.fullmatch(r"config\.([a-z-]+)\.toml", item.name)
    else:
        match = None
    if match:
        return "mise", match[1]
    if path in {"justfile", "apps/writer/justfile", "editors/justfile", "editors/zed/justfile"}:
        return "just", path
    if (
        item.parent in {PurePosixPath("."), PurePosixPath(".config/just")}
        and item.name in JUST_MODULES
    ):
        return "just", item.name
    return None


def tooling_owners(path):
    kind = config_kind(path)
    if kind:
        category, name = kind
        if category == "mise":
            return MISE_PROFILES.get(name, LANES) if name else None
        return JUST_MODULES.get(name)
    item = PurePosixPath(path)
    if item.parent in {PurePosixPath("."), PurePosixPath(".config")} and item.name in {
        "dprint.json",
        "tombi.toml",
        ".rumdl.toml",
        "rumdl.toml",
        "_typos.toml",
        ".editorconfig",
        ".clang-format",
        "stylua.toml",
        ".luarc.json",
        "gdlintrc",
    }:
        return JUST
    if path in {"deny.toml", ".cargo/deny.toml"}:
        return frozenset({"dependencies", "maintainability"})
    return None


def just_source(path, content):
    """Compare the effective directory, rather than its spelling after a move."""
    source = content.decode()
    pattern = r"^set working-directory := (.+)$"
    matches = re.findall(pattern, source, re.M)
    if len(matches) > 1:
        return None
    try:
        directory = ast.literal_eval(matches[0]) if matches else "."
    except (SyntaxError, ValueError):
        return None
    if not isinstance(directory, str):
        return None
    directory = posixpath.normpath(str(PurePosixPath(path).parent / directory))
    source = re.sub(pattern + r"\n?", "", source, flags=re.M)
    return f"set working-directory := {directory!r}\n{source}"


def cargo_inputs(source):
    manifest = tomllib.loads(source.decode())
    workspace = manifest.get("workspace", {})
    metadata = workspace.get("metadata", {})
    metadata.pop("typos", None)
    if not metadata:
        workspace.pop("metadata", None)
    return manifest


def config_change_lanes(before_path, after_path, base, head):
    if before_path is None or after_path is None:
        return None
    kind = config_kind(before_path)
    same_path = before_path == after_path
    cargo = same_path and PurePosixPath(before_path).name == "Cargo.toml"
    ignore = same_path and before_path == ".gitignore"
    if not (cargo or ignore or (kind is not None and kind == config_kind(after_path))):
        return None
    before, after = file_at(base, before_path), file_at(head, after_path)
    if before is None or after is None:
        return None
    if ignore:

        def source_rules(content):
            return [
                raw
                for raw in content.decode().splitlines()
                if raw.strip() and not raw.startswith("#") and raw not in LOCAL_MISE
            ]

        return JUST if source_rules(before) == source_rules(after) else frozenset({"rust"})
    if cargo:
        return JUST if cargo_inputs(before) == cargo_inputs(after) else RUST_BUILD
    category, name = kind
    if category == "mise":
        old, new = tomllib.loads(before.decode()), tomllib.loads(after.decode())
        return (
            mise_lanes(old, new)
            if not name
            else (JUST if old == new else MISE_PROFILES.get(name, LANES))
        )
    old, new = just_source(before_path, before), just_source(after_path, after)
    if old is None or new is None:
        return LANES
    owners = JUST_MODULES.get(
        name,
        {
            "justfile": JUST,
            "apps/writer/justfile": frozenset({"writer", "maintainability"}),
            "editors/justfile": frozenset({"rust", "editor-native", "maintainability"}),
            "editors/zed/justfile": frozenset({"editor-native", "maintainability"}),
        }.get(name, LANES),
    )
    return justfile_lanes(old, new, default_lanes=owners, root=name == "justfile")


class Change(NamedTuple):
    before: str | None
    after: str | None


def changed_files(base, head, *, pull_request):
    # Retain both sides of renames. Known configuration moves can compare their
    # actual settings; all other moves still select both consumers.
    revision = f"{base}...{head}" if pull_request else f"{base}..{head}"
    result = subprocess.run(
        ["git", "diff", "--name-status", "--find-renames", "-z", revision, "--"],
        check=True,
        stdout=subprocess.PIPE,
    )
    fields = iter(os.fsdecode(field) for field in result.stdout.split(b"\0")[:-1])
    changes = []
    for status in fields:
        path = next(fields)
        if status.startswith(("R", "C")):
            changes.append(Change(path, next(fields)))
        else:
            changes.append(Change(None if status == "A" else path, None if status == "D" else path))
    return changes
