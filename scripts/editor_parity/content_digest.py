"""Content inputs used to invalidate selected Cargo test targets."""

import hashlib
import os
import stat
import subprocess
from dataclasses import dataclass
from pathlib import Path

from .model import Context
from .paths import require_no_symlink_components


def selected_target_digest(ctx: Context, package: str) -> str:
    """Hash accepted source inputs visible to the selected Cargo workspace.

    Cargo remains responsible for parsing Rust and discovering tests. The digest
    is only an explicit rustc argument, ensuring a restored source mtime cannot
    make Cargo reuse a stale selected test executable. ``package`` remains part
    of the interface because the digest is attached to one selected target.

    Input enumeration is intentionally Git-aware: tracked files and nonignored
    untracked files are accepted, while ignored untracked files are not an
    accepted compiler-input surface. Projects that need Cargo to consume an
    ignored file must force-add it (or remove the ignore rule); otherwise there
    is no reliable way to discover it before compilation and the digest does
    not claim to cover it.
    """
    del package
    inputs = _digest_inputs(ctx)

    digest = hashlib.sha256()
    for input_file in inputs:
        path = input_file.path
        try:
            # Use the host filesystem encoding rather than UTF-8 text encoding
            # so valid non-UTF-8 Git paths retain a stable digest identity.
            relative = os.fsencode(path.relative_to(ctx.repo_root).as_posix())
            content = path.read_bytes()
            filesystem_mode = stat.S_IMODE(path.stat().st_mode)
        except OSError as error:
            ctx.require(False, f"unable to read evidence digest input {path}: {error}")
            continue
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update((input_file.git_mode or 0).to_bytes(4, "big"))
        digest.update(filesystem_mode.to_bytes(4, "big"))
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def workspace_files(ctx: Context) -> list[Path]:
    """Return Git-tracked and nonignored untracked files in stable byte order.

    A filesystem walk sees ignored build products and editor caches that cannot
    affect Cargo's source graph. Git already has the authoritative answer for
    tracked and nonignored untracked paths, and ``-z`` preserves spaces and
    arbitrary filename bytes without quoting. The path checks below remain
    necessary because Git can record symlinks and because this function is also
    responsible for the repository-boundary safety contract.
    """
    return [input_file.path for input_file in _digest_inputs(ctx)]


@dataclass(frozen=True)
class _DigestInput:
    path: Path
    git_mode: int | None


def _digest_inputs(ctx: Context) -> list[_DigestInput]:
    inputs: list[_DigestInput] = []
    for raw_path, tracked, git_mode in _git_files(ctx):
        try:
            relative = Path(os.fsdecode(raw_path))
        except (TypeError, ValueError) as error:
            ctx.require(False, f"unable to decode Git digest input: {error}")
            continue
        if relative.is_absolute() or any(component == ".." for component in relative.parts):
            ctx.require(False, f"workspace digest input escapes the repository: {relative}")
            continue
        path = ctx.repo_root / relative
        if path.is_symlink():
            require_no_symlink_components(ctx, path, "workspace digest input")
            continue
        if path.is_dir():
            ctx.require(False, f"workspace digest input must not be a directory: {relative}")
            continue
        if not tracked and _ignored(ctx, relative):
            continue
        if path.is_file() and _safe_input(ctx, path, "workspace digest input"):
            inputs.append(_DigestInput(path, git_mode))
    return sorted(
        inputs,
        key=lambda input_file: os.fsencode(input_file.path.relative_to(ctx.repo_root).as_posix()),
    )


def _git_files(ctx: Context) -> list[tuple[bytes, bool, int | None]]:
    """List Git paths with tracking state and index mode, without shell parsing."""
    all_paths = _run_git_files(
        ctx,
        ["ls-files", "--cached", "--others", "--exclude-standard", "-z"],
    )
    staged_entries = _run_git_files(ctx, ["ls-files", "--cached", "--stage", "-z"])
    if all_paths is None or staged_entries is None:
        return []

    tracked_modes: dict[bytes, int] = {}
    for entry in staged_entries.split(b"\0"):
        if not entry:
            continue
        try:
            metadata, raw_path = entry.split(b"\t", 1)
            fields = metadata.split()
            if len(fields) != 3:
                raise ValueError("expected mode, object ID, and stage")
            mode = int(fields[0], 8)
            stage = int(fields[2])
        except (ValueError, IndexError) as error:
            ctx.require(False, f"unable to parse Git digest input metadata: {error}")
            continue
        if stage != 0:
            ctx.require(False, f"workspace digest input has an unmerged Git index entry: {os.fsdecode(raw_path)}")
        if mode not in {0o100644, 0o100755, 0o120000, 0o160000}:
            ctx.require(False, f"workspace digest input has unsupported Git mode {mode:o}: {os.fsdecode(raw_path)}")
        if mode == 0o160000:
            ctx.require(False, f"workspace digest input must not be a gitlink: {os.fsdecode(raw_path)}")
        existing_mode = tracked_modes.get(raw_path)
        if existing_mode is not None and existing_mode != mode:
            ctx.require(False, f"workspace digest input has conflicting Git modes: {os.fsdecode(raw_path)}")
        tracked_modes[raw_path] = mode
    return [
        (raw_path, raw_path in tracked_modes, tracked_modes.get(raw_path))
        for raw_path in all_paths.split(b"\0")
        if raw_path
    ]


def _run_git_files(ctx: Context, arguments: list[str]) -> bytes | None:
    """Run a path-only Git query while retaining raw filename bytes."""
    try:
        result = subprocess.run(
            ["git", "-C", os.fsencode(ctx.repo_root), *arguments],
            capture_output=True,
            check=False,
        )
    except OSError as error:
        ctx.require(False, f"unable to enumerate Git digest inputs: {error}")
        return None
    if result.returncode != 0:
        detail = os.fsdecode(result.stderr).strip() or "git ls-files failed"
        ctx.require(False, f"unable to enumerate Git digest inputs: {detail}")
        return None
    return result.stdout


def _ignored(ctx: Context, relative_path: Path) -> bool:
    # Agent instructions are repository metadata, not compiler-visible inputs.
    if relative_path == Path("CLAUDE.md") or relative_path.parts[:1] == (".claude",):
        return True
    if any(part in {".git", "node_modules", "__pycache__"} for part in relative_path.parts):
        return True
    if relative_path.suffix in {".pyc", ".pyo"}:
        return True
    target_relative = _target_relative(ctx)
    return bool(target_relative and (relative_path == target_relative or target_relative in relative_path.parents))


def _target_relative(ctx: Context) -> Path | None:
    try:
        return ctx.cargo_target_dir.resolve().relative_to(ctx.repo_root)
    except ValueError:
        return None


def _safe_input(ctx: Context, path: Path, label: str) -> bool:
    require_no_symlink_components(ctx, path, label)
    return not path.is_symlink() and ctx.repo_root in path.resolve().parents
