import json
import os
import sys
from pathlib import Path

if __package__ in {None, ""}:
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from editor_parity.contract import validate
from editor_parity.model import Context
from editor_parity.paths import require_control_file
from editor_parity.portable_lock import PortableLock


def main(argv: list[str]) -> int:
    if len(argv) != 4:
        print("usage: check.py REPO_ROOT FIXTURE DOCUMENT", file=sys.stderr)
        return 2
    repo_root, fixture_path, document_path = map(Path, argv[1:])
    cargo_target_dir = Path(os.environ.get("CARGO_TARGET_DIR", str(repo_root / "target")))
    if not cargo_target_dir.is_absolute():
        cargo_target_dir = repo_root / cargo_target_dir
    errors: list[str] = []
    ctx = Context(repo_root, errors, cargo_target_dir)
    lock_path = cargo_target_dir / "editor-parity.lock"
    with PortableLock(lock_path):
        fixture_input = require_control_file(ctx, fixture_path, "editor parity fixture")
        document_input = require_control_file(ctx, document_path, "editor parity documentation")
        if ctx.errors:
            for error in ctx.errors:
                print(f"editor parity contract: {error}", file=sys.stderr)
            return 1
        try:
            data = json.loads(fixture_input.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            print(f"unable to read editor parity contract: {error}", file=sys.stderr)
            return 2
        if not isinstance(data, dict):
            print("editor parity contract must contain a JSON object", file=sys.stderr)
            return 2
        validate(ctx, data, document_input)
    if errors:
        for error in errors:
            print(f"editor parity contract: {error}", file=sys.stderr)
        return 1
    counts = {key: len(data.get(key, [])) for key in ("capabilities", "scenarios", "clients", "artifacts")}
    print("Editor parity contract passed: " + ", ".join(f"{value} {key}" for key, value in counts.items()) + ".")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
