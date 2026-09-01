"""Content inputs used to invalidate selected Cargo test targets."""

import hashlib
from pathlib import Path

from .model import Context
from .paths import require_no_symlink_components


def selected_target_digest(ctx: Context, package: str) -> str:
    """Hash workspace manifests and all Rust sources in the selected package.

    Cargo remains responsible for parsing Rust and discovering tests. The digest
    is only an explicit rustc argument, ensuring a restored source mtime cannot
    make Cargo reuse a stale selected test executable.
    """
    inputs: list[Path] = []
    manifests = [ctx.repo_root / "Cargo.toml"]
    crates_root = ctx.repo_root / "crates"
    if crates_root.is_dir():
        manifests.extend(crates_root.rglob("Cargo.toml"))
    for path in sorted(set(manifests)):
        if path.is_file() and _safe_input(ctx, path, "Cargo manifest"):
            inputs.append(path)
    lockfile = ctx.repo_root / "Cargo.lock"
    if lockfile.is_file() and _safe_input(ctx, lockfile, "Cargo lockfile"):
        inputs.append(lockfile)

    package_root = ctx.repo_root / "crates" / package
    if not package_root.is_dir():
        ctx.require(False, f"evidence package source directory does not exist: {package}")
    else:
        for path in sorted(package_root.rglob("*.rs")):
            if _ignored(path):
                continue
            if path.is_file() and _safe_input(ctx, path, f"{package} Rust source"):
                inputs.append(path)

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


def _ignored(path: Path) -> bool:
    return any(part in {".git", "target", "node_modules"} for part in path.parts)


def _safe_input(ctx: Context, path: Path, label: str) -> bool:
    require_no_symlink_components(ctx, path, label)
    return not path.is_symlink() and ctx.repo_root in path.resolve().parents
