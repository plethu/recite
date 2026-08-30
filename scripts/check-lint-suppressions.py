#!/usr/bin/env python3
"""Diff-aware policy check for Rust lint suppressions.

Rust syntax and ancestry come from :mod:`lint_suppression_ast`, which delegates
parsing to the pinned ast-grep tool. This module owns Git ranges, matching, and
the repository policy only.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import PurePosixPath

from lint_suppression_ast import ParseError, scan_sources


ALLOWLIST_PATH = "scripts/generated-rust-allowlist.txt"


@dataclass(frozen=True)
class Suppression:
    path: str
    line: int
    kind: str
    lints: tuple[str, ...]
    reason: str | None
    inner: bool
    scope: str
    target: str
    category: str
    status: str = "current"
    owner_stable: bool = True

    @property
    def broad(self) -> bool:
        return self.scope in {"crate", "module"}

    @property
    def semantic_key(self) -> tuple[str, str, str, tuple[str, ...], str, str]:
        return (self.path, self.category, self.kind, self.lints, self.scope, self.target)

    @property
    def lint_key(self) -> tuple[str, tuple[str, ...], str, str, str]:
        return (self.path, self.category, self.kind, self.lints, self.target)


def git(repo: str, *args: str) -> bytes:
    result = subprocess.run(["git", "-C", repo, *args], stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if result.returncode:
        raise RuntimeError(result.stderr.decode("utf-8", "replace").strip() or "git command failed")
    return result.stdout


def resolve(repo: str, reference: str) -> str:
    return git(repo, "rev-parse", "--verify", f"{reference}^{{commit}}").decode().strip()


def is_zero_sha(reference: str) -> bool:
    return len(reference) == 40 and set(reference) == {"0"}


def tracked_paths(repo: str, revision: str) -> list[str]:
    return [path.decode("utf-8", "replace") for path in git(repo, "ls-tree", "-r", "--name-only", "-z", revision).split(b"\0") if path]


def changed_paths(repo: str, base: str, head: str) -> list[str]:
    zero_base = is_zero_sha(base)
    if zero_base:
        base = git(repo, "hash-object", "-t", "tree", "/dev/null").decode().strip()
    diff_args = [base, head] if zero_base else [f"{base}...{head}"]
    fields = git(repo, "diff", "--name-status", "-z", "-M", *diff_args, "--", "*.rs").split(b"\0")
    paths: list[str] = []
    index = 0
    while index < len(fields) and fields[index]:
        status = fields[index].decode("ascii", "replace")
        index += 1
        if status.startswith(("R", "C")):
            if index + 1 >= len(fields):
                break
            paths.append(fields[index + 1].decode("utf-8", "replace"))
            index += 2
        elif index < len(fields):
            path = fields[index].decode("utf-8", "replace")
            index += 1
            if status[0] != "D":
                paths.append(path)
    return paths


def file_at(repo: str, revision: str, path: str) -> str | None:
    if not git(repo, "ls-tree", "-r", "--name-only", revision, "--", path).strip():
        return None
    return git(repo, "show", f"{revision}:{path}").decode("utf-8", "replace")


def generated_allowlist(repo: str, revision: str) -> set[str]:
    content = file_at(repo, revision, ALLOWLIST_PATH)
    if content is None:
        return set()
    allowed: set[str] = set()
    for line_number, line in enumerate(content.splitlines(), 1):
        value = line.strip()
        if not value or value.startswith("#"):
            continue
        if value.startswith("/") or ".." in PurePosixPath(value).parts or not value.endswith(".rs"):
            raise ParseError(f"invalid generated Rust allowlist entry at line {line_number}")
        if file_at(repo, revision, value) is None:
            raise ParseError(f"generated Rust allowlist path is missing: {value}")
        allowed.add(value)
    return allowed


def records(raw: list[dict[str, object]]) -> list[Suppression]:
    return [Suppression(**item) for item in raw]


def match_suppressions(base: list[Suppression], current: list[Suppression]) -> list[Suppression]:
    remaining = [candidate for candidate in base if candidate.owner_stable]
    result: list[Suppression] = []

    def take(item: Suppression, predicate):
        candidates = [(abs(candidate.line - item.line), index, candidate) for index, candidate in enumerate(remaining) if predicate(candidate)]
        if not candidates:
            return None
        _, index, candidate = min(candidates)
        remaining.pop(index)
        return candidate

    for item in current:
        if not item.owner_stable:
            result.append(Suppression(**{**item.__dict__, "status": "new"}))
            continue
        matched = take(item, lambda candidate: candidate.semantic_key == item.semantic_key)
        if matched is not None:
            status = "baseline" if matched.reason == item.reason else "reason-changed"
            if matched.reason and not item.reason:
                status = "reason-removed"
            result.append(Suppression(**{**item.__dict__, "status": status}))
            continue
        matched = take(item, lambda candidate: candidate.lint_key == item.lint_key)
        if matched is not None:
            result.append(Suppression(**{**item.__dict__, "status": "scope-changed"}))
            continue
        matched = take(item, lambda candidate: candidate.path == item.path and candidate.category == item.category and candidate.kind == item.kind and candidate.target == item.target and set(candidate.lints).issubset(item.lints))
        if matched is not None:
            result.append(Suppression(**{**item.__dict__, "status": "expanded"}))
            continue
        matched = take(item, lambda candidate: candidate.path == item.path and candidate.category == item.category and candidate.kind == item.kind and candidate.target == item.target and set(item.lints).issubset(candidate.lints))
        if matched is not None:
            result.append(Suppression(**{**item.__dict__, "status": "narrowed"}))
            continue
        result.append(Suppression(**{**item.__dict__, "status": "new"}))
    return result


def reason_ok(item: Suppression, prefix: str | None = None) -> bool:
    return bool(item.reason and item.reason.strip()) and (prefix is None or item.reason.startswith(prefix))


def violation(item: Suppression) -> str | None:
    if item.category == "generated" or item.status == "baseline" or item.category in {"tests", "fixtures", "benchmarks"}:
        return None
    if item.broad and item.category == "production":
        return "new production crate/module-wide suppressions are not permitted"
    if item.category == "ffi":
        if item.broad:
            return "FFI suppressions must be item-scoped"
        return None if reason_ok(item, "ffi:") else 'FFI-boundary suppressions must carry reason = "ffi: ..."'
    if item.category == "compatibility":
        if item.broad and item.kind == "allow":
            return "compatibility #[allow] must be item-scoped"
        return None if reason_ok(item, "compatibility:") else 'public compatibility suppressions must carry reason = "compatibility: ..."'
    return None if reason_ok(item) else 'new production suppressions must carry a non-empty literal reason = "..."'


def display(item: Suppression) -> str:
    reason = "null" if item.reason is None else json.dumps(item.reason, ensure_ascii=False)
    rationale = "missing" if not item.reason or not item.reason.strip() else "present"
    if item.category in {"ffi", "compatibility"} and item.reason and item.reason.startswith(item.category + ":"):
        rationale = "scoped"
    return (f"{item.path}:{item.line}: {item.status} {item.kind}({','.join(item.lints)}) "
            f"scope={item.scope} owner={item.target} category={item.category} "
            f"owner_stable={'true' if item.owner_stable else 'false'} reason={reason} "
            f"rationale={rationale} baseline_status={item.status}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("refs", nargs="*", help="base and head Git refs")
    parser.add_argument("--full", action="store_true", help="inventory all tracked Rust source")
    parser.add_argument("--policy-revision", help="revision supplying repository-owned policy files")
    args = parser.parse_args()
    if len(args.refs) > 2:
        parser.error("expected at most base-ref and head-ref")
    try:
        repo = subprocess.run(["git", "rev-parse", "--show-toplevel"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=True).stdout.strip()
        base_ref = args.refs[0] if args.refs else os.environ.get("RECITE_BASE_REF", "origin/main")
        head_ref = args.refs[1] if len(args.refs) > 1 else os.environ.get("RECITE_HEAD_REF", "HEAD")
        head = resolve(repo, head_ref)
        base = None if args.full else (base_ref if is_zero_sha(base_ref) else resolve(repo, base_ref))
        policy = head if args.policy_revision is None else resolve(repo, args.policy_revision)
        paths = [path for path in (tracked_paths(repo, head) if args.full else changed_paths(repo, base, head)) if path.endswith(".rs")]
        generated = generated_allowlist(repo, policy)
    except (OSError, RuntimeError, ParseError, subprocess.CalledProcessError) as error:
        print(f"lint suppression policy setup failed: {error}", file=sys.stderr)
        return 2

    try:
        current_sources = [(path, file_at(repo, head, path)) for path in sorted(set(paths))]
        if any(source is None for _, source in current_sources):
            raise RuntimeError("head path is missing from the checked-out tree")
        current = records(scan_sources([(path, source) for path, source in current_sources if source is not None], generated))
        baseline_sources: list[tuple[str, str]] = []
        if base is not None and not is_zero_sha(base):
            for path in sorted(set(paths)):
                source = file_at(repo, base, path)
                if source is not None:
                    baseline_sources.append((path, source))
        baseline = records(scan_sources(baseline_sources, generated)) if base is not None else []
    except (OSError, RuntimeError, ParseError) as error:
        print(f"lint suppression policy setup failed: {error}", file=sys.stderr)
        return 2

    output = [Suppression(**{**item.__dict__, "status": "current"}) for item in current] if base is None else match_suppressions(baseline, current)
    if base is None:
        print(f"lint suppression inventory: {len(output)} current suppressions at {head}")
    else:
        print(f"lint suppression inventory: {len(baseline)} baseline / {len(current)} current suppressions")
        print(f"range: {base}...{head}")
    for item in sorted(output, key=lambda value: (value.path, value.line, value.kind, value.lints)):
        print(display(item))
    if args.full:
        print("full inventory mode is reporting-only; no existing suppressions are rejected")
        return 0
    failures = 0
    for item in output:
        message = violation(item)
        if message is not None and item.status != "baseline":
            print(f"lint suppression policy violation: {item.path}:{item.line}: {message}", file=sys.stderr)
            failures += 1
    if failures:
        print(f"lint suppression policy failed with {failures} violation(s)", file=sys.stderr)
        return 1
    print("lint suppression policy passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
