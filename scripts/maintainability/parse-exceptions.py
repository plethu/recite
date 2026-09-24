#!/usr/bin/env python3
"""Validate the small TOML exception list before shell policy checks."""

import re
import sys
import tomllib
from pathlib import Path


def main() -> int:
    try:
        data = tomllib.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
        if set(data) != {"exceptions"} or not isinstance(data["exceptions"], list):
            raise ValueError("expected only an exceptions array")
        seen: set[str] = set()
        for entry in data["exceptions"]:
            if not isinstance(entry, dict) or set(entry) != {"path", "max_lines", "issue", "reason"}:
                raise ValueError("each exception needs path, max_lines, issue, and reason")
            path, maximum, issue, reason = (
                entry["path"], entry["max_lines"], entry["issue"], entry["reason"]
            )
            if not isinstance(path, str) or not path or path in seen or any(c in path for c in "\t\r\n"):
                raise ValueError(f"duplicate or invalid exception path: {path!r}")
            if type(maximum) is not int or maximum < 1:
                raise ValueError(f"invalid max_lines for {path}")
            if not isinstance(issue, str) or not re.fullmatch(r"#[1-9][0-9]*", issue):
                raise ValueError(f"invalid issue for {path}")
            if not isinstance(reason, str) or not reason.strip() or any(c in reason for c in "\t\r\n"):
                raise ValueError(f"invalid reason for {path}")
            seen.add(path)
            print(f"{path}\t{maximum}\t{issue}\t{reason}")
    except (OSError, tomllib.TOMLDecodeError, ValueError) as error:
        print(f"invalid maintainability exceptions: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
