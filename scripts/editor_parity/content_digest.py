"""Content inputs used to invalidate selected Cargo test targets."""

import hashlib
import os
from pathlib import Path

from .model import Context
from .paths import require_no_symlink_components


def selected_target_digest(ctx: Context, package: str) -> str:
    """Hash the repository files visible to the selected Cargo workspace.

    Cargo remains responsible for parsing Rust and discovering tests. The digest
    is only an explicit rustc argument, ensuring a restored source mtime cannot
    make Cargo reuse a stale selected test executable. ``package`` remains part
    of the interface because the digest is attached to one selected target;
    inputs are deliberately conservative so include and build-script syntax is
    not reimplemented here.
    """
    del package
    inputs = workspace_files(ctx)

    digest = hashlib.sha256()
    for path in inputs:
        try:
            relative = path.relative_to(ctx.repo_root).as_posix().encode()
            content = path.read_bytes()
        except OSError as error:
            ctx.require(False, f"unable to read evidence digest input {path}: {error}")
            continue
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def workspace_files(ctx: Context) -> list[Path]:
    inputs: list[Path] = []
    for root, directories, filenames in os.walk(ctx.repo_root, topdown=True, followlinks=False):
        root_path = Path(root)
        relative_root = root_path.relative_to(ctx.repo_root)
        directories[:] = [
            directory
            for directory in directories
            if not _ignored(ctx, relative_root / directory)
        ]
        for filename in filenames:
            path = root_path / filename
            if _ignored(ctx, path.relative_to(ctx.repo_root)) or path.is_symlink():
                continue
            if path.is_file() and _safe_input(ctx, path, "workspace digest input"):
                inputs.append(path)
    return sorted(inputs, key=lambda path: path.relative_to(ctx.repo_root).as_posix())


def _ignored(ctx: Context, relative_path: Path) -> bool:
    if any(part in {".git", "node_modules"} for part in relative_path.parts):
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
