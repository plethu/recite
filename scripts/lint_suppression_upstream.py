"""Verify the approved, frozen Xilem source baseline before classifying it."""

import hashlib
import json
import re
from pathlib import PurePosixPath

from lint_suppression_ast import ParseError

MANIFEST = "scripts/lint-suppressions-upstream.json"
PREFIX = "apps/writer/vendor/xilem/"


def frozen_upstream(file_at, repo, policy, head):
    content = file_at(repo, policy, MANIFEST)
    if content is None:
        return set()
    try:
        entries = json.loads(content)
    except ValueError as error:
        raise ParseError(f"invalid upstream lint baseline: {error}") from error
    if not isinstance(entries, list):
        raise ParseError("upstream lint baseline must be a list")
    frozen = set()
    for entry in entries:
        if (
            not isinstance(entry, dict)
            or set(entry) != {"path", "sha256", "upstream_path", "revision"}
            or not all(isinstance(value, str) for value in entry.values())
        ):
            raise ParseError("invalid upstream lint baseline entry")
        path = entry["path"]
        if (
            not path.startswith(PREFIX)
            or str(PurePosixPath(path)) != path
            or ".." in PurePosixPath(path).parts
            or any(character in path for character in "*?[]\\")
            or not path.endswith(".rs")
            or entry["upstream_path"] != path.removeprefix(PREFIX)
            or not re.fullmatch(r"[0-9a-f]{64}", entry["sha256"])
            or not re.fullmatch(r"[0-9a-f]{40}", entry["revision"])
            or path in frozen
        ):
            raise ParseError(f"invalid upstream lint baseline path or digest: {path}")
        source = file_at(repo, head, path)
        if source is None or hashlib.sha256(source.encode("utf-8")).hexdigest() != entry["sha256"]:
            raise ParseError(f"frozen upstream lint baseline changed: {path}")
        frozen.add(path)
    return frozen
