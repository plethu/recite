from pathlib import Path


class Context:
    """Shared validation state and error collection."""

    statuses = {"planned", "partial", "implemented", "unsupported"}
    clients = {"vscode", "vscodium", "neovim", "zed"}
    platforms = {"linux", "macos", "windows"}

    def __init__(self, repo_root: Path, errors: list[str], cargo_target_dir: Path):
        self.repo_root = repo_root.resolve()
        self.errors = errors
        self.cargo_target_dir = cargo_target_dir
        self.cargo_test_list_cache: dict[tuple[str, str], set[str]] = {}
        self.cargo_exact_selection_cache: dict[tuple[str, str, str], set[str]] = {}

    def require(self, condition: bool, message: str) -> None:
        if not condition:
            self.errors.append(message)
