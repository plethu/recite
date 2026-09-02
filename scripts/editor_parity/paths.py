from pathlib import Path

from .model import Context


def symlink_component(repo_root: Path, path: Path) -> Path | None:
    try:
        relative = path.relative_to(repo_root)
    except ValueError:
        return None
    current = repo_root
    for component in relative.parts:
        current /= component
        if current.is_symlink():
            return current
    return None


def require_no_symlink_components(ctx: Context, path: Path, label: str) -> None:
    symlink = symlink_component(ctx.repo_root, path)
    if symlink is None:
        return
    relative = symlink.relative_to(ctx.repo_root)
    if symlink == path:
        ctx.require(False, f"{label} must not be a symlink: {relative}")
    else:
        ctx.require(False, f"{label} must not traverse symlink component: {relative}")


def require_repo_file(ctx: Context, path: str, label: str) -> tuple[Path, Path]:
    candidate = ctx.repo_root / path
    require_no_symlink_components(ctx, candidate, label)
    try:
        resolved = candidate.resolve(strict=True)
    except OSError:
        resolved = candidate.resolve()
    ctx.require(ctx.repo_root in resolved.parents, f"{label} escapes the repository: {path}")
    ctx.require(resolved.is_file(), f"{label} does not exist: {path}")
    return candidate, resolved


def require_control_file(ctx: Context, path: Path, label: str) -> Path:
    """Validate a checker control input before opening or parsing it."""
    candidate = path if path.is_absolute() else ctx.repo_root / path
    require_no_symlink_components(ctx, candidate, label)
    try:
        resolved = candidate.resolve(strict=True)
    except OSError:
        resolved = candidate.resolve()
    ctx.require(ctx.repo_root in resolved.parents, f"{label} escapes the repository: {path}")
    ctx.require(resolved.is_file(), f"{label} does not exist: {path}")
    return candidate
