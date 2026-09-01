"""Content-sensitive Rust test-target reachability checks."""

import re
from pathlib import Path

from .model import Context
from .paths import symlink_component

PATH_ATTRIBUTE = re.compile(r'^\s*#\[path\s*=\s*"([^"]+)"\]\s*$')
MODULE = re.compile(r"^\s*mod\s+([A-Za-z_]\w*)\s*;\s*(?://.*)?$")
TEST_ATTRIBUTE = re.compile(r"^\s*#\[(?:test|tokio::test|async_std::test|rstest(?:\([^]]*\))?)\]\s*$")
FUNCTION = re.compile(r"^\s*(?:(?:pub|unsafe|async)\s+)*fn\s+([A-Za-z_]\w*)\s*\(")


def reachable_test_names(ctx: Context, root: Path) -> set[str]:
    """Return test names reachable from a Cargo integration-test root.

    This deliberately remains a small source-graph authority. Cargo still
    performs exact test discovery and selection; this guard prevents a stale
    binary from making a disconnected module look runnable.
    """
    names: set[str] = set()
    visited: set[Path] = set()

    def visit(path: Path, prefix: tuple[str, ...]) -> None:
        resolved = path.resolve()
        symlink = symlink_component(ctx.repo_root, path)
        if symlink is not None:
            relative = symlink.relative_to(ctx.repo_root)
            ctx.require(False, f"evidence source graph must not use symlink component: {relative}")
            return
        if ctx.repo_root not in resolved.parents:
            ctx.require(False, f"evidence source graph escapes the repository: {path}")
            return
        if resolved in visited:
            return
        visited.add(resolved)
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except OSError as error:
            ctx.require(False, f"unable to read evidence source graph file {path}: {error}")
            return
        pending_path: str | None = None
        test_attribute = False
        for line in lines:
            path_match = PATH_ATTRIBUTE.match(line)
            if path_match:
                pending_path = path_match.group(1)
                continue
            if TEST_ATTRIBUTE.match(line):
                test_attribute = True
                continue
            function_match = FUNCTION.match(line)
            if function_match:
                if test_attribute:
                    names.add("::".join((*prefix, function_match.group(1))))
                test_attribute = False
                continue
            module_match = MODULE.match(line)
            if module_match:
                module_name = module_match.group(1)
                module_path = path.parent / (pending_path or f"{module_name}.rs")
                pending_path = None
                if module_path.is_file():
                    visit(module_path, (*prefix, module_name))
                else:
                    ctx.require(False, f"evidence source graph module is missing: {module_path}")
                test_attribute = False
                continue
            if line.strip() and not line.lstrip().startswith("//") and not line.lstrip().startswith("#"):
                test_attribute = False

    visit(root, ())
    return names
