"""Recognized source and installed-host evidence commands."""

import re
import shlex

from .cargo_evidence import discovered_test_paths, exact_test_selection
from .model import Context
from .paths import require_no_symlink_components, require_repo_file

HOST_RUNNER_PATTERN = re.compile(r"scripts/check-[a-z0-9][a-z0-9-]*-host\.sh")
SHARED_HOST_CLIENTS = {"vscode": frozenset({"vscode", "vscodium"})}


def is_host_runner(command: object) -> bool:
    return isinstance(command, str) and HOST_RUNNER_PATTERN.fullmatch(command) is not None


def host_runner_client(command: str) -> str | None:
    if not is_host_runner(command):
        return None
    return command[len("scripts/check-") : -len("-host.sh")]


def host_runner_clients(command: str) -> frozenset[str]:
    slug = host_runner_client(command)
    if slug is None:
        return frozenset()
    return SHARED_HOST_CLIENTS.get(slug, frozenset({slug}))


def validate_command(ctx: Context, capability_id: str, command: str) -> None:
    if command in {
        "scripts/check-tree-sitter.sh",
        "scripts/check-neovim.sh",
        "scripts/check-vscode.sh",
        "scripts/check-zed.sh",
    }:
        script, _ = require_repo_file(ctx, command, f"capability {capability_id} evidence script")
        if script.is_file():
            ctx.require(
                script.stat().st_mode & 0o111,
                f"capability {capability_id} evidence script is not executable: {command}",
            )
        return
    if is_host_runner(command):
        script, _ = require_repo_file(
            ctx, command, f"capability {capability_id} installed-host evidence runner"
        )
        if script.is_file():
            ctx.require(
                script.stat().st_mode & 0o111,
                f"capability {capability_id} installed-host evidence runner is not executable: {command}",
            )
        return
    try:
        parts = shlex.split(command)
    except ValueError as error:
        parts = []
        ctx.require(False, f"capability {capability_id} evidence command is malformed: {error}")
    valid_shape = (
        len(parts) == 8
        and parts[:2] == ["cargo", "test"]
        and parts[2:4] == ["--locked", "-p"]
        and parts[5] == "--test"
        and "--" not in parts
    )
    ctx.require(
        valid_shape,
        f"capability {capability_id} evidence command must name a cargo integration test and filter",
    )
    if not valid_shape:
        return
    package, target, test_filter = parts[4], parts[6], parts[7]
    test_file = ctx.repo_root / "crates" / package / "tests" / f"{target}.rs"
    require_no_symlink_components(ctx, test_file, f"capability {capability_id} evidence target")
    resolved_test_file = test_file.resolve()
    ctx.require(
        ctx.repo_root in resolved_test_file.parents,
        f"capability {capability_id} evidence target escapes the repository: {test_file}",
    )
    ctx.require(
        test_file.is_file() and not test_file.is_symlink(),
        f"capability {capability_id} evidence target does not exist: {test_file.relative_to(ctx.repo_root) if test_file.is_relative_to(ctx.repo_root) else test_file}",
    )
    if test_file.is_file() and not test_file.is_symlink():
        registered = discovered_test_paths(ctx, package, target)
        if registered is None:
            return
        ctx.require(
            test_filter in registered,
            f"capability {capability_id} evidence command does not name an existing runnable test discovered by Cargo: {test_filter}",
        )
        if test_filter not in registered:
            return
        selected = exact_test_selection(ctx, package, target, test_filter)
        if selected is None:
            return
        ctx.require(
            selected == {test_filter},
            f"capability {capability_id} evidence command does not select exactly one Cargo test: {test_filter}",
        )
