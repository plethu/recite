"""Rust suppression discovery backed by the repository's ast-grep parser.

The policy checker deliberately delegates Rust syntax and ancestry boundaries to
ast-grep. This module turns its structural ranges into small records; it does
not attempt to implement a Rust lexer or declaration parser.
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath


MARKERS = {
    "compatibility": "recite-lint-suppression: compatibility",
    "ffi": "recite-lint-suppression: ffi",
}
RULES = {
    "outer_attribute": "kind: attribute_item",
    "inner_attribute": "kind: inner_attribute_item",
    "rust_error": "kind: ERROR",
    "line_comment": 'pattern: "//"',
    "block_comment": "pattern: '/* $$$TEXT */'",
    "mod_item": "kind: mod_item",
    "impl_item": "kind: impl_item",
    "trait_item": "kind: trait_item",
    "struct_item": "kind: struct_item",
    "enum_item": "kind: enum_item",
    "union_item": "kind: union_item",
    "type_item": "kind: type_item",
    "function_item": "kind: function_item",
    "function_signature_item": "kind: function_signature_item",
    "const_item": "kind: const_item",
    "static_item": "kind: static_item",
    "use_declaration": "kind: use_declaration",
    "associated_type": "kind: associated_type",
    "enum_variant": "kind: enum_variant",
    "field_declaration": "kind: field_declaration",
    "declaration_list": "kind: declaration_list",
    "block": "kind: block",
    "closure": "kind: closure_expression",
    "macro_invocation": "kind: macro_invocation",
    "macro_definition": "kind: macro_definition",
    "foreign_mod_item": "kind: foreign_mod_item",
    "extern_crate": "kind: extern_crate_declaration",
}


class ParseError(ValueError):
    """ast-grep could not provide a complete structural source view."""


@dataclass(frozen=True)
class AstEvent:
    rule: str
    start: int
    end: int
    text: str


def _rule_text() -> str:
    return "\n---\n".join(
        f"id: {rule_id}\nlanguage: Rust\nrule:\n  {rule}"
        for rule_id, rule in RULES.items()
    )


def ast_grep_scan(sources: list[tuple[str, str]]) -> dict[str, list[AstEvent]]:
    if not sources:
        return {}
    executable = shutil.which("ast-grep")
    if executable is None:
        raise ParseError("missing required tool: ast-grep (run the maintainability mise environment)")
    with tempfile.TemporaryDirectory(prefix="recite-lint-ast-") as temporary:
        root = Path(temporary)
        for path, source in sources:
            destination = root / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(source, encoding="utf-8")
        result = subprocess.run(
            [executable, "scan", "--inline-rules", _rule_text(), "--json=stream",
             "--no-ignore", "hidden", str(root)],
            capture_output=True, text=True, check=False,
        )
        if result.returncode:
            raise ParseError(result.stderr.strip() or result.stdout.strip() or "ast-grep failed")
        parsed: dict[str, list[AstEvent]] = {path: [] for path, _ in sources}
        for raw in result.stdout.splitlines():
            try:
                match = json.loads(raw)
                path = Path(match["file"]).relative_to(root).as_posix()
                span = match["range"]["byteOffset"]
                parsed.setdefault(path, []).append(
                    AstEvent(match["ruleId"], int(span["start"]), int(span["end"]), match["text"])
                )
            except (json.JSONDecodeError, KeyError, TypeError, ValueError) as error:
                raise ParseError(f"malformed ast-grep result: {error}") from error
        for events in parsed.values():
            events.sort(key=lambda event: (event.start, event.end, event.rule))
        return parsed


def _split_top_level(source: str) -> list[str]:
    pieces: list[str] = []
    start = 0
    depth = {"(": 0, "[": 0, "{": 0}
    matching = {")": "(", "]": "[", "}": "{",
    }
    quote: str | None = None
    raw_hashes: int | None = None
    index = 0
    while index < len(source):
        char = source[index]
        if raw_hashes is not None:
            closing = '"' + ("#" * raw_hashes)
            if source.startswith(closing, index):
                index += len(closing)
                raw_hashes = None
            else:
                index += 1
            continue
        if quote is not None:
            if char == "\\":
                index += 2
            elif char == quote:
                quote = None
                index += 1
            else:
                index += 1
            continue
        if char in {'"', "'"}:
            quote = char
            index += 1
            continue
        if char == "r" and index + 1 < len(source):
            cursor = index + 1
            while cursor < len(source) and source[cursor] == "#":
                cursor += 1
            if cursor < len(source) and source[cursor] == '"':
                raw_hashes = cursor - index - 1
                index = cursor + 1
                continue
        if char in depth:
            depth[char] += 1
        elif char in matching and depth[matching[char]]:
            depth[matching[char]] -= 1
        elif char == "," and not any(depth.values()):
            pieces.append(source[start:index])
            start = index + 1
        index += 1
    if quote is not None or raw_hashes is not None or any(depth.values()):
        raise ParseError("malformed suppression attribute")
    pieces.append(source[start:])
    return pieces


def _without_comments(source: str) -> str:
    output: list[str] = []
    quote: str | None = None
    index = 0
    while index < len(source):
        char = source[index]
        if quote is not None:
            output.append(char)
            if char == "\\" and index + 1 < len(source):
                output.append(source[index + 1])
                index += 2
            elif char == quote:
                quote = None
                index += 1
            else:
                index += 1
            continue
        if char in {'"', "'"}:
            quote = char
            output.append(char)
            index += 1
            continue
        if source.startswith("//", index):
            newline = source.find("\n", index + 2)
            index = len(source) if newline < 0 else newline
            continue
        if source.startswith("/*", index):
            depth = 1
            index += 2
            while index < len(source) and depth:
                if source.startswith("/*", index):
                    depth += 1
                    index += 2
                elif source.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    output.append("\n" if source[index] == "\n" else " ")
                    index += 1
            if depth:
                raise ParseError("malformed suppression attribute comment")
            continue
        output.append(char)
        index += 1
    if quote is not None:
        raise ParseError("malformed suppression attribute literal")
    return "".join(output)


def _literal(value: str) -> str | None:
    value = value.strip()
    if len(value) < 2 or value[0] != '"':
        return None
    index = 1
    while index < len(value):
        if value[index] == "\\":
            index += 2
        elif value[index] == '"':
            return value[1:index] if not value[index + 1:].strip() else None
        else:
            index += 1
    return None


def _meta(body: str) -> list[tuple[str, tuple[str, ...], str | None]]:
    body = _without_comments(body).strip()
    if "(" not in body or not body.endswith(")"):
        if body in {"allow", "expect"}:
            raise ParseError(f"malformed {body} suppression attribute")
        return []
    name, payload = body.split("(", 1)
    name, payload = name.strip(), payload[:-1]
    if name == "cfg_attr":
        pieces = _split_top_level(payload)
        if len(pieces) < 2:
            raise ParseError("malformed cfg_attr suppression attribute")
        records: list[tuple[str, tuple[str, ...], str | None]] = []
        for piece in pieces[1:]:
            records.extend(_meta(piece))
        return records
    if name not in {"allow", "expect"}:
        return []
    lints: list[str] = []
    reason: str | None = None
    for piece in _split_top_level(payload):
        piece = piece.strip()
        if not piece:
            continue
        if piece.startswith("reason"):
            equals = piece.find("=")
            if equals < 0 or piece[:equals].strip() != "reason":
                raise ParseError(f"malformed {name} suppression reason")
            reason = _literal(piece[equals + 1:])
            if reason is None:
                raise ParseError(f"malformed {name} suppression reason")
            continue
        normalized = "".join(piece.split())
        if not normalized or not all(part.isidentifier() for part in normalized.split("::")):
            raise ParseError(f"malformed {name} suppression lint")
        lints.append(normalized)
    if not lints:
        raise ParseError(f"malformed {name} suppression attribute")
    return [(name, tuple(sorted(set(lints))), reason)]


def _attributes(event: AstEvent) -> list[tuple[str, tuple[str, ...], str | None, bool]]:
    text = event.text
    inner = event.rule == "inner_attribute"
    prefix = "#![" if inner else "#["
    if not text.startswith(prefix) or not text.endswith("]"):
        raise ParseError("malformed Rust attribute")
    return [(kind, lints, reason, inner) for kind, lints, reason in _meta(text[len(prefix):-1])]


NAMED = {
    "mod_item": "mod", "trait_item": "trait", "struct_item": "struct",
    "enum_item": "enum", "union_item": "union", "type_item": "type",
    "function_item": "fn", "function_signature_item": "fn",
    "const_item": "const", "static_item": "static", "associated_type": "type",
}


def _target(node: AstEvent, source: str, bodies: list[AstEvent]) -> tuple[str, bool]:
    if node.rule == "impl_item":
        body = next((item for item in bodies if node.start < item.start < item.end <= node.end), None)
        raw = source.encode()[node.start:(body.start if body else node.end)].decode("utf-8", "replace")
        header = re.sub(r"^(?:unsafe\s+)?impl\b\s*", "", " ".join(raw.split()))
        return (f"impl:{header}", bool(header))
    if node.rule == "use_declaration":
        use = " ".join(node.text.split())
        if re.search(r"\bas\s+(?:r#)?_\b", use):
            return "unstable", False
        return f"use:{re.sub(r';\s*$', '', re.sub(r'^use\s+', '', use))}", True
    keyword = NAMED.get(node.rule)
    if keyword is None:
        return "unstable", False
    match = re.search(rf"(?<![\w#]){re.escape(keyword)}\s+([^\s(<{{;,=:\[]+)", node.text)
    if match is None or match.group(1) in {"_", "r#_"}:
        return "unstable", False
    return f"{keyword}:{match.group(1)}", True


def _comments(events: list[AstEvent], source: bytes) -> list[tuple[int, int]]:
    comments: list[tuple[int, int]] = []
    for event in events:
        if event.rule == "block_comment":
            comments.append((event.start, event.end))
        elif event.rule == "line_comment":
            newline = source.find(b"\n", event.start)
            comments.append((event.start, len(source) if newline < 0 else newline))
    return comments


def _trivia_gap(source: bytes, start: int, end: int, spans: list[tuple[int, int]]) -> bool:
    cursor = start
    for left, right in spans:
        if right <= cursor:
            continue
        if left >= end:
            break
        if left > cursor and source[cursor:min(left, end)].strip():
            return False
        cursor = max(cursor, min(right, end))
        if cursor >= end:
            return True
    return not source[cursor:end].strip()


def _category(path: str, offset: int, source: bytes, comments: list[tuple[int, int]], generated: set[str]) -> str:
    normalized = path.lower()
    parts = PurePosixPath(normalized).parts
    if path in generated:
        return "generated"
    if "fixtures" in parts:
        return "fixtures"
    if "tests" in parts or normalized.endswith("/tests.rs") or normalized.startswith("tests/"):
        return "tests"
    if "benches" in parts:
        return "benchmarks"
    if "recite-ffi" in parts or "ffi" in parts or any(part.startswith("ffi_") or part.endswith("_ffi") for part in parts):
        return "ffi"
    if "compat" in normalized or "compatibility" in normalized:
        return "compatibility"
    previous = [span for span in comments if span[1] <= offset]
    if previous:
        start, end = max(previous, key=lambda span: span[1])
        comment = source[start:end].decode("utf-8", "replace")
        commented_attribute = re.search(r"#\s*!?\s*\[\s*(?:allow|expect)\b", comment)
        if not source[end:offset].strip() and commented_attribute is None:
            for category, marker in MARKERS.items():
                if marker in comment:
                    return category
    return "production"


def scan_sources(sources: list[tuple[str, str]], generated_paths: set[str]) -> list[dict[str, object]]:
    events_by_path = ast_grep_scan(sources)
    records: list[dict[str, object]] = []
    for path, source in sources:
        events = events_by_path.get(path, [])
        if any(event.rule == "rust_error" for event in events):
            raise ParseError(f"malformed Rust syntax in {path}")
        data = source.encode("utf-8")
        comments = _comments(events, data)
        attrs = [event for event in events if event.rule in {"outer_attribute", "inner_attribute"}]
        nodes = [event for event in events if event.rule not in {
            "outer_attribute", "inner_attribute", "rust_error", "line_comment", "block_comment", "declaration_list",
        }]
        bodies = [event for event in events if event.rule == "declaration_list"]
        trivia = sorted(comments + [(event.start, event.end) for event in attrs])
        info = {id(node): _target(node, source, bodies) for node in nodes}
        named = [node for node in nodes if info[id(node)][0] != "unstable"]

        for attr in attrs:
            for kind, lints, reason, inner in _attributes(attr):
                owner = None
                if inner:
                    owner = min(
                        (node for node in nodes if node.start <= attr.start and attr.end <= node.end),
                        key=lambda node: (node.end - node.start, node.start), default=None,
                    )
                    if owner is None:
                        scope, target, stable, prefix_nodes = "crate", "crate", True, []
                    elif owner.rule == "mod_item":
                        scope, target, stable = "module", "", True
                        prefix_nodes = [node for node in named if node.start <= owner.start and owner.end <= node.end] + [owner]
                    elif owner.rule == "block":
                        scope, target, stable = "block", "block", False
                        prefix_nodes = [node for node in named if node.start <= owner.start and owner.end <= node.end]
                    else:
                        scope, target, stable, prefix_nodes = "item", "unstable", False, []
                else:
                    owner = next(
                        (node for node in sorted((node for node in nodes if node.start >= attr.end), key=lambda node: (node.start, node.end - node.start))
                         if _trivia_gap(data, attr.end, node.start, trivia)), None,
                    )
                    if owner is None:
                        scope, target, stable, prefix_nodes = "item", "unstable", False, []
                    else:
                        owner_target, owner_named = info[id(owner)]
                        prefix_nodes = [node for node in named if node is not owner and node.start <= owner.start and owner.end <= node.end]
                        unstable_parent = any(
                            node.rule in {
                                "block", "closure", "macro_invocation", "macro_definition",
                                "enum_variant", "field_declaration", "foreign_mod_item", "extern_crate",
                            }
                            and node is not owner and node.start <= owner.start and owner.end <= node.end for node in nodes
                        )
                        stable = owner_named and not unstable_parent
                        if owner.rule == "mod_item":
                            scope, target = "module", owner_target
                        elif owner.rule in {"block", "closure", "macro_invocation"}:
                            scope, target, stable = "block", "block", False
                        elif owner.rule in {"macro_definition", "enum_variant", "field_declaration"}:
                            scope, target, stable = "item", "unstable", False
                        else:
                            scope, target = "item", owner_target
                prefix_nodes.sort(key=lambda node: node.end - node.start, reverse=True)
                prefix = [info[id(node)][0] for node in prefix_nodes if info[id(node)][0] != "unstable"]
                if prefix:
                    if target in {"", "crate"}:
                        target = "::".join(prefix + ([target] if target else []))
                    elif target == "block":
                        target = "::".join(prefix + [target])
                    elif target != "unstable":
                        target = "::".join(prefix + [target])
                records.append({
                    "path": path, "line": data[:attr.start].count(b"\n") + 1,
                    "kind": kind, "lints": lints, "reason": reason, "inner": inner,
                    "scope": scope, "target": target, "category": _category(path, attr.start, data, comments, generated_paths),
                    "owner_stable": stable,
                })
    return records
