#!/usr/bin/env python3
"""Diff-aware policy check for Rust lint suppressions.

This deliberately is not a Rust type or lint checker.  It lexes enough Rust to
find attribute syntax without treating comments and strings as source, then
compares suppression attributes in a Git range.  Cargo/rustc remain the
authority for the meaning of a lint or attribute.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import PurePosixPath


IDENT_START = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_"
IDENT_CONT = IDENT_START + "0123456789"
OPEN = "([{"
CLOSE = ")] }".replace(" ", "")
PAIRS = {')': '(', ']': '[', '}': '{'}
ALLOWLIST_PATH = "scripts/generated-rust-allowlist.txt"


class ParseError(ValueError):
    """The source contains a suppression-shaped attribute we cannot parse."""


def is_ident_start(char: str) -> bool:
    return bool(char) and char in IDENT_START


def is_ident_continue(char: str) -> bool:
    return bool(char) and char in IDENT_CONT


def raw_string_end(source: str, start: int) -> int | None:
    """Return the exclusive end of a Rust raw string starting at *start*."""

    index = start
    if source.startswith("br", index):
        index += 2
    elif source.startswith("r", index):
        index += 1
    else:
        return None

    hashes = 0
    while index < len(source) and source[index] == '#':
        hashes += 1
        index += 1
    if index >= len(source) or source[index] != '"':
        return None
    closing = '"' + ('#' * hashes)
    end = source.find(closing, index + 1)
    return len(source) if end < 0 else end + len(closing)


def quoted_string_end(source: str, start: int, quote: str = '"') -> int:
    index = start + 1
    while index < len(source):
        if source[index] == '\\':
            index += 2
        elif source[index] == quote:
            return index + 1
        else:
            index += 1
    return len(source)


def block_comment_end(source: str, start: int) -> int:
    depth = 1
    index = start + 2
    while index < len(source) - 1:
        if source.startswith('/*', index):
            depth += 1
            index += 2
        elif source.startswith('*/', index):
            depth -= 1
            index += 2
            if depth == 0:
                return index
        else:
            index += 1
    return len(source)


def skip_space_comments(source: str, start: int) -> int:
    index = start
    while index < len(source):
        if source[index].isspace():
            index += 1
        elif source.startswith('//', index):
            newline = source.find('\n', index + 2)
            index = len(source) if newline < 0 else newline + 1
        elif source.startswith('/*', index):
            index = block_comment_end(source, index)
        else:
            break
    return index


def matching_delimiter(source: str, start: int) -> int | None:
    """Find the matching delimiter, respecting Rust comments and strings."""

    if start >= len(source) or source[start] not in OPEN:
        return None
    stack = [source[start]]
    index = start + 1
    while index < len(source):
        raw_end = raw_string_end(source, index)
        if raw_end is not None:
            index = raw_end
            continue
        if source.startswith('//', index):
            newline = source.find('\n', index + 2)
            index = len(source) if newline < 0 else newline + 1
            continue
        if source.startswith('/*', index):
            index = block_comment_end(source, index)
            continue
        if source[index] == '"':
            index = quoted_string_end(source, index)
            continue
        if source[index] == "'":
            # A lifetime (`'name`) has no closing quote.  Only skip a quote
            # when a short character literal is actually present.
            candidate = quoted_string_end(source, index, "'")
            if candidate <= index + 8 and candidate > index + 1:
                index = candidate
                continue
        char = source[index]
        if char in OPEN:
            stack.append(char)
        elif char in CLOSE:
            if not stack or PAIRS[char] != stack[-1]:
                return None
            stack.pop()
            if not stack:
                return index
        index += 1
    return None


def strip_comments(source: str) -> str:
    """Replace comments with spaces while preserving strings and newlines."""

    output = list(source)
    index = 0
    while index < len(source):
        raw_end = raw_string_end(source, index)
        if raw_end is not None:
            index = raw_end
            continue
        if source.startswith('//', index):
            newline = source.find('\n', index + 2)
            end = len(source) if newline < 0 else newline
            for position in range(index, end):
                output[position] = ' '
            index = end
            continue
        if source.startswith('/*', index):
            end = block_comment_end(source, index)
            for position in range(index, end):
                if source[position] != '\n':
                    output[position] = ' '
            index = end
            continue
        if source[index] == '"':
            index = quoted_string_end(source, index)
            continue
        if source[index] == "'":
            candidate = quoted_string_end(source, index, "'")
            if candidate <= index + 8 and candidate > index + 1:
                index = candidate
                continue
        index += 1
    return ''.join(output)


def split_top_level(source: str, delimiter: str = ',') -> list[str]:
    pieces: list[str] = []
    start = 0
    stack: list[str] = []
    index = 0
    while index < len(source):
        raw_end = raw_string_end(source, index)
        if raw_end is not None:
            index = raw_end
            continue
        if source[index] == '"':
            index = quoted_string_end(source, index)
            continue
        char = source[index]
        if char in OPEN:
            stack.append(char)
        elif char in CLOSE and stack and PAIRS[char] == stack[-1]:
            stack.pop()
        elif char == delimiter and not stack:
            pieces.append(source[start:index])
            start = index + 1
        index += 1
    pieces.append(source[start:])
    return pieces


def parse_double_string(value: str) -> str | None:
    value = value.strip()
    if not value.startswith('"'):
        return None
    end = quoted_string_end(value, 0)
    if end != len(value):
        return None
    # The policy only needs a non-empty literal and a scoped prefix.  Preserve
    # escapes rather than pretending to perform Rust string interpretation.
    return value[1:-1]


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

    @property
    def broad(self) -> bool:
        return self.scope in {"crate", "module"}

    @property
    def semantic_key(self) -> tuple[str, tuple[str, ...], str, str, str, str]:
        return (self.path, self.category, self.kind, self.lints, self.scope, self.target)

    @property
    def lint_key(self) -> tuple[str, tuple[str, ...], str, str, str]:
        return (self.path, self.category, self.kind, self.lints, self.target)


def parse_attribute_body(body: str) -> list[tuple[str, tuple[str, ...], str | None, bool]]:
    """Parse direct suppressions and suppressions nested in cfg_attr."""

    body = strip_comments(body).strip()
    index = 0
    while index < len(body) and is_ident_continue(body[index]):
        index += 1
    kind = body[:index]
    remainder = body[index:].strip()
    if kind == "cfg_attr":
        if not remainder.startswith('(') or not remainder.endswith(')'):
            raise ParseError("malformed cfg_attr suppression attribute")
        pieces = split_top_level(remainder[1:-1])
        if len(pieces) < 2:
            raise ParseError("malformed cfg_attr suppression attribute")
        nested: list[tuple[str, tuple[str, ...], str | None, bool]] = []
        for piece in pieces[1:]:
            nested.extend(parse_attribute_body(piece))
        return nested
    if kind not in {"allow", "expect"}:
        return []
    if not remainder.startswith('(') or not remainder.endswith(')'):
        raise ParseError(f"malformed {kind} suppression attribute")
    payload = remainder[1:-1]
    lints: list[str] = []
    reason: str | None = None
    for piece in split_top_level(payload):
        piece = piece.strip()
        if not piece:
            continue
        if piece.startswith("reason"):
            equals = piece.find('=')
            if equals < 0 or piece[:equals].strip() != "reason":
                raise ParseError(f"malformed {kind} suppression reason")
            parsed_reason = parse_double_string(piece[equals + 1:])
            if parsed_reason is None:
                raise ParseError(f"malformed {kind} suppression reason")
            reason = parsed_reason
            continue
        lint = ''.join(piece.split())
        components = lint.split('::')
        if not components or any(not part or not all(is_ident_continue(char) for char in part)
                                 or not is_ident_start(part[0]) for part in components):
            raise ParseError(f"malformed {kind} suppression lint")
        lints.append(lint)
    if not lints:
        raise ParseError(f"malformed {kind} suppression attribute")
    return [(kind, tuple(sorted(set(lints))), reason, False)]


def parse_attr(source: str, hash_index: int, bracket_index: int, close: int,
               path: str) -> list[tuple[str, tuple[str, ...], str | None, bool]]:
    inner = source[hash_index + 1] == '!'
    body = strip_comments(source[bracket_index + 1:close]).strip()
    parsed = parse_attribute_body(body)
    return [(kind, lints, reason, inner if inner else nested_inner)
            for kind, lints, reason, nested_inner in parsed]


def next_scope(source: str, start: int, inner: bool) -> tuple[str, str, int]:
    if inner:
        return "crate", "crate", start
    index = start
    while True:
        index = skip_space_comments(source, index)
        if index < len(source) and source[index] == '#':
            bracket = index + 1
            if bracket < len(source) and source[bracket] == '!':
                bracket += 1
            if bracket < len(source) and source[bracket] == '[':
                close = matching_delimiter(source, bracket)
                if close is not None:
                    index = close + 1
                    continue
        break
    modifiers = {"async", "const", "default", "extern", "pub", "safe", "unsafe"}
    while index < len(source) and is_ident_start(source[index]):
        end = index + 1
        while end < len(source) and is_ident_continue(source[end]):
            end += 1
        word = source[index:end]
        if word == "mod":
            name_start = skip_space_comments(source, end)
            name_end = name_start
            if name_start < len(source) and is_ident_start(source[name_start]):
                name_end += 1
                while name_end < len(source) and is_ident_continue(source[name_end]):
                    name_end += 1
            target = source[name_start:name_end] or "mod"
            return "module", f"mod:{target}", index
        if word not in modifiers:
            name_start = skip_space_comments(source, end)
            if word == "macro_rules" and name_start < len(source) and source[name_start] == '!':
                name_start = skip_space_comments(source, name_start + 1)
            name_end = name_start
            if name_start < len(source) and is_ident_start(source[name_start]):
                name_end += 1
                while name_end < len(source) and is_ident_continue(source[name_end]):
                    name_end += 1
            target = source[name_start:name_end] or word
            return "item", f"{word}:{target}", index
        index = skip_space_comments(source, end)
        if index < len(source) and source[index] == '(':
            close = matching_delimiter(source, index)
            if close is None:
                return "item", "item", index
            index = close + 1
            index = skip_space_comments(source, index)
    return "item", "item", index


def category_for(path: str, source: str, offset: int, generated_paths: set[str]) -> str:
    normalized = path.lower()
    parts = PurePosixPath(normalized).parts
    if path in generated_paths:
        return "generated"
    if "fixtures" in parts:
        return "fixtures"
    if ("tests" in parts or normalized.endswith("/tests.rs")
            or normalized.startswith("tests/")):
        return "tests"
    if "benches" in parts:
        return "benchmarks"
    if ("recite-ffi" in parts or "ffi" in parts
            or any(part.startswith("ffi_") or part.endswith("_ffi") for part in parts)):
        return "ffi"
    if "compat" in normalized or "compatibility" in normalized:
        return "compatibility"
    preceding = source[max(0, source.rfind('\n', 0, offset - 1) - 512):offset]
    if "recite-lint-suppression: compatibility" in preceding:
        return "compatibility"
    if "recite-lint-suppression: ffi" in preceding:
        return "ffi"
    return "production"


def declaration_owner(tokens: list[str]) -> str | None:
    """Return the owner introduced by a module, impl, or trait brace."""

    def structured(kind: str, components: list[str]) -> str:
        encoded = ','.join(f"{len(component)}:{component}" for component in components)
        return f"{kind}:[{encoded}]"

    for index, token in enumerate(tokens):
        if token not in {"mod", "trait", "impl"} or index + 1 >= len(tokens):
            continue
        components = tokens[index + 1:]
        if any(component and is_ident_start(component[0]) for component in components):
            return structured(token, components)
    return None


class OwnerTracker:
    """Track lexical module/impl/trait ancestry while scanning one source file."""

    def __init__(self) -> None:
        self.contexts: list[str | None] = []
        self.segment: list[str] = []

    def word(self, token: str) -> None:
        self.segment.append(token)

    def punctuation(self, token: str) -> None:
        if token == "{":
            self.contexts.append(declaration_owner(self.segment))
            self.segment.clear()
        elif token == "}":
            if self.contexts:
                self.contexts.pop()
            self.segment.clear()
        elif token == ";":
            self.segment.clear()
        else:
            self.segment.append(token)

    def prefix(self) -> str:
        return "::".join(context for context in self.contexts if context is not None)


def scan_source(path: str, source: str, generated_paths: set[str]) -> list[Suppression]:
    suppressions: list[Suppression] = []
    owners = OwnerTracker()
    index = 0
    while index < len(source):
        raw_end = raw_string_end(source, index)
        if raw_end is not None:
            index = raw_end
            continue
        if source.startswith('//', index):
            newline = source.find('\n', index + 2)
            index = len(source) if newline < 0 else newline + 1
            continue
        if source.startswith('/*', index):
            index = block_comment_end(source, index)
            continue
        if source[index] == '"':
            index = quoted_string_end(source, index)
            continue
        if source[index] == "'":
            candidate = quoted_string_end(source, index, "'")
            if candidate <= index + 8 and candidate > index + 1:
                index = candidate
                continue
        if source[index] != '#':
            if is_ident_start(source[index]):
                end = index + 1
                while end < len(source) and is_ident_continue(source[end]):
                    end += 1
                owners.word(source[index:end])
                index = end
                continue
            if source[index] in '{};':
                owners.punctuation(source[index])
            elif not source[index].isspace():
                owners.punctuation(source[index])
            index += 1
            continue

        bracket = index + 1
        if bracket < len(source) and source[bracket] == '!':
            bracket += 1
        if bracket >= len(source) or source[bracket] != '[':
            index += 1
            continue
        close = matching_delimiter(source, bracket)
        if close is None:
            raise ParseError(
                f"malformed Rust attribute in {path}:{source.count(chr(10), 0, index) + 1}"
            )
        parsed = parse_attr(source, index, bracket, close, path)
        for kind, lints, reason, inner in parsed:
            scope, target, _ = next_scope(source, close + 1, inner)
            prefix = owners.prefix()
            if prefix:
                target = f"{prefix}::{target}"
            line = source.count('\n', 0, index) + 1
            suppressions.append(Suppression(
                path=path,
                line=line,
                kind=kind,
                lints=lints,
                reason=reason,
                inner=inner,
                scope=scope,
                target=target,
                category=category_for(path, source, index, generated_paths),
            ))
        index = close + 1
    return suppressions


def git(repo: str, *args: str) -> bytes:
    result = subprocess.run(["git", "-C", repo, *args], stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, check=False)
    if result.returncode != 0:
        detail = result.stderr.decode("utf-8", "replace").strip()
        raise RuntimeError(detail or "git command failed")
    return result.stdout


def resolve(repo: str, reference: str) -> str:
    return git(repo, "rev-parse", "--verify", f"{reference}^{{commit}}").decode().strip()


def is_zero_sha(reference: str) -> bool:
    return len(reference) == 40 and set(reference) == {'0'}


def tracked_paths(repo: str, revision: str) -> list[str]:
    return [path.decode("utf-8", "replace") for path in git(
        repo, "ls-tree", "-r", "--name-only", "-z", revision).split(b'\0') if path]


def changed_paths(repo: str, base: str, head: str) -> list[str]:
    if is_zero_sha(base):
        empty_tree = git(repo, "hash-object", "-t", "tree", "/dev/null").decode().strip()
        arguments = [empty_tree, head]
    else:
        arguments = [f"{base}...{head}"]
    fields = git(repo, "diff", "--name-status", "-z", "-M", *arguments, "--", "*.rs").split(b'\0')
    paths: list[str] = []
    index = 0
    while index < len(fields) and fields[index]:
        status = fields[index].decode("ascii", "replace")
        index += 1
        if status.startswith(('R', 'C')):
            if index + 1 >= len(fields):
                break
            new = fields[index + 1].decode("utf-8", "replace")
            index += 2
            paths.append(new)
        else:
            if index >= len(fields):
                break
            path = fields[index].decode("utf-8", "replace")
            index += 1
            if status[0] != 'D':
                paths.append(path)
    return paths


def file_at(repo: str, revision: str, path: str) -> str | None:
    # ls-tree distinguishes a legitimately absent path from an object or Git
    # failure. Do not turn a missing head blob into an empty source file.
    entries = git(repo, "ls-tree", "-r", "--name-only", revision, "--", path)
    if not entries.strip():
        return None
    return git(repo, "show", f"{revision}:{path}").decode("utf-8", "replace")


def generated_allowlist(repo: str, revision: str) -> set[str]:
    content = file_at(repo, revision, ALLOWLIST_PATH)
    if content is None:
        return set()
    allowed: set[str] = set()
    for line_number, line in enumerate(content.splitlines(), 1):
        value = line.strip()
        if not value or value.startswith('#'):
            continue
        if (value.startswith('/') or '..' in PurePosixPath(value).parts
                or not value.endswith('.rs')):
            raise ParseError(f"invalid generated Rust allowlist entry at line {line_number}")
        if file_at(repo, revision, value) is None:
            raise ParseError(f"generated Rust allowlist path is missing: {value}")
        allowed.add(value)
    return allowed


def match_suppressions(base: list[Suppression], current: list[Suppression]) -> list[Suppression]:
    remaining = list(base)
    result: list[Suppression] = []

    def take(predicate):
        candidates = [(abs(candidate.line - item.line), index, candidate)
                      for index, candidate in enumerate(remaining) if predicate(candidate)]
        if not candidates:
            return None
        _, index, candidate = min(candidates)
        remaining.pop(index)
        return candidate

    for item in current:
        matched = take(lambda candidate: candidate.semantic_key == item.semantic_key)
        if matched is not None:
            status = "baseline"
            if matched.reason and not item.reason:
                status = "reason-removed"
            elif matched.reason != item.reason:
                status = "reason-changed"
            result.append(Suppression(**{**item.__dict__, "status": status}))
            continue

        matched = take(lambda candidate: candidate.lint_key == item.lint_key)
        if matched is not None:
            result.append(Suppression(**{**item.__dict__, "status": "scope-changed"}))
            continue

        matched = take(lambda candidate: candidate.path == item.path
                       and candidate.category == item.category
                       and candidate.kind == item.kind
                       and candidate.target == item.target
                       and set(candidate.lints).issubset(item.lints))
        if matched is not None:
            result.append(Suppression(**{**item.__dict__, "status": "expanded"}))
            continue

        matched = take(lambda candidate: candidate.path == item.path
                       and candidate.category == item.category
                       and candidate.kind == item.kind
                       and candidate.target == item.target
                       and set(item.lints).issubset(candidate.lints))
        if matched is not None:
            result.append(Suppression(**{**item.__dict__, "status": "narrowed"}))
            continue

        result.append(Suppression(**{**item.__dict__, "status": "new"}))
    return result


def reason_ok(item: Suppression, prefix: str | None = None) -> bool:
    if item.reason is None or not item.reason.strip():
        return False
    return prefix is None or item.reason.startswith(prefix)


def violation(item: Suppression) -> str | None:
    if item.category == "generated" or item.status == "baseline":
        return None
    if item.category in {"tests", "fixtures", "benchmarks"}:
        return None
    if item.broad and item.category == "production":
        return "new production crate/module-wide suppressions are not permitted"
    if item.category == "ffi":
        if item.broad:
            return "FFI suppressions must be item-scoped"
        if not reason_ok(item, "ffi:"):
            return "FFI-boundary suppressions must carry reason = \"ffi: ...\""
        return None
    if item.category == "compatibility":
        if item.broad and item.kind == "allow":
            return "compatibility #[allow] must be item-scoped"
        if not reason_ok(item, "compatibility:"):
            return "public compatibility suppressions must carry reason = \"compatibility: ...\""
        return None
    if not reason_ok(item):
        return "new production suppressions must carry a non-empty literal reason = \"...\""
    return None


def display(item: Suppression) -> str:
    lints = ','.join(item.lints)
    reason = "null" if item.reason is None else json.dumps(item.reason, ensure_ascii=False)
    rationale = "missing"
    if item.reason is not None and item.reason.strip():
        if item.category == "ffi" and item.reason.startswith("ffi:"):
            rationale = "scoped"
        elif item.category == "compatibility" and item.reason.startswith("compatibility:"):
            rationale = "scoped"
        else:
            rationale = "present"
    return (f"{item.path}:{item.line}: {item.status} {item.kind}({lints}) "
            f"scope={item.scope} owner={item.target} category={item.category} "
            f"reason={reason} rationale={rationale} baseline_status={item.status}")


def rust_path(path: str) -> bool:
    # Every tracked Rust file is reviewable. Generated output is exempt only
    # when its exact path appears in the repository-owned allowlist.
    return path.endswith('.rs')


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("refs", nargs="*", help="base and head Git refs")
    parser.add_argument("--full", action="store_true", help="inventory all tracked Rust source")
    parser.add_argument(
        "--policy-revision",
        help="revision supplying repository-owned policy files such as the generated allowlist",
    )
    args = parser.parse_args()
    if len(args.refs) > 2:
        parser.error("expected at most base-ref and head-ref")

    try:
        repo = subprocess.run(["git", "rev-parse", "--show-toplevel"], stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE, text=True, check=True).stdout.strip()
        base_ref = args.refs[0] if args.refs else os.environ.get("RECITE_BASE_REF", "origin/main")
        head_ref = args.refs[1] if len(args.refs) > 1 else os.environ.get("RECITE_HEAD_REF", "HEAD")
        head = resolve(repo, head_ref)
        base = None if args.full else (base_ref if is_zero_sha(base_ref) else resolve(repo, base_ref))
        policy_revision = (
            head if args.policy_revision is None else resolve(repo, args.policy_revision)
        )
        if args.full:
            paths = [path for path in tracked_paths(repo, head) if rust_path(path)]
        else:
            paths = changed_paths(repo, base, head)
            paths = [path for path in paths if rust_path(path)]
        generated_paths = generated_allowlist(repo, policy_revision)
    except (OSError, RuntimeError, ParseError, subprocess.CalledProcessError) as error:
        print(f"lint suppression policy setup failed: {error}", file=sys.stderr)
        return 2

    current: list[Suppression] = []
    baseline: list[Suppression] = []
    try:
        for path in sorted(set(paths)):
            source = file_at(repo, head, path)
            if source is None:
                raise RuntimeError(f"head path is missing from the checked-out tree: {path}")
            current.extend(scan_source(path, source, generated_paths))
            if base is not None and not is_zero_sha(base):
                # A rename or split is deliberately new at its destination;
                # never consume an attribute from the old path.
                baseline_source = file_at(repo, base, path)
                if baseline_source is not None:
                    baseline.extend(scan_source(path, baseline_source, generated_paths))
    except (OSError, RuntimeError, ParseError) as error:
        print(f"lint suppression policy setup failed: {error}", file=sys.stderr)
        return 2

    if base is None:
        records = [Suppression(**{**item.__dict__, "status": "current"}) for item in current]
        print(f"lint suppression inventory: {len(records)} current suppressions at {head}")
    else:
        records = match_suppressions(baseline, current)
        print(f"lint suppression inventory: {len(baseline)} baseline / {len(current)} current suppressions")
        print(f"range: {base}...{head}")
    for item in sorted(records, key=lambda value: (value.path, value.line, value.kind, value.lints)):
        print(display(item))

    if args.full:
        print("full inventory mode is reporting-only; no existing suppressions are rejected")
        return 0

    failures = 0
    for item in records:
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
