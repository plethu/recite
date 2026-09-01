"""Rust suppression discovery backed by rustfmt and ast-grep.

The policy checker first asks the pinned workspace rustfmt to parse each source
file without writing it, then delegates structural ranges and ancestry
boundaries to ast-grep. This module turns those ranges into small records; it
does not attempt to implement a Rust declaration parser.
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath

from lint_suppression_meta import MetadataError, _normalize_identifier, _without_comments, attributes


MARKERS = {
    "compatibility": "recite-lint-suppression: compatibility",
    "ffi": "recite-lint-suppression: ffi",
}
RULES = {
    "outer_attribute": "kind: attribute_item",
    "inner_attribute": "kind: inner_attribute_item",
    "identifier": "kind: identifier",
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
    """The pinned syntax or structural parser could not provide a source view."""


@dataclass(frozen=True)
class AstEvent:
    rule: str
    start: int
    end: int
    text: str


LINT_CONTROLS = {"allow", "expect", "cfg_attr"}


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


def rustfmt_parse(sources: list[tuple[str, str]]) -> None:
    """Parse sources with rustfmt, discarding stdout and never rewriting them."""
    if not sources:
        return
    executable = shutil.which("rustfmt")
    if executable is None:
        raise ParseError("missing required tool: rustfmt (run the pinned workspace toolchain)")
    with tempfile.TemporaryDirectory(prefix="recite-lint-rustfmt-") as temporary:
        root = Path(temporary)
        for path, source in sources:
            destination = root / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(source, encoding="utf-8")
            result = subprocess.run(
                [
                    executable, "--edition", "2024", "--config", "skip_children=true",
                    "--emit", "stdout", str(destination),
                ],
                stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, check=False,
            )
            if result.returncode:
                detail = next(
                    (line for line in result.stderr.splitlines() if line.startswith("error:")),
                    "parse error",
                )
                raise ParseError(f"rustfmt rejected Rust syntax in {path}: {detail}")


def _is_configuration_attribute(event: AstEvent) -> bool:
    prefix = "#![" if event.rule == "inner_attribute" else "#["
    if not event.text.startswith(prefix):
        return False
    body = _without_comments(event.text[len(prefix):-1]).lstrip()
    match = re.match(r"(?:r#)?[A-Za-z_][A-Za-z0-9_]*\b", body)
    return match is not None and _normalize_identifier(match.group()) in {"cfg", "cfg_attr"}


def _attributes(event: AstEvent) -> list[tuple[str, tuple[str, ...], str | None, bool]]:
    try:
        return attributes(event.text, event.rule == "inner_attribute")
    except MetadataError as error:
        raise ParseError(str(error)) from error


def _owner_for_attribute(
    attr: AstEvent, nodes: list[AstEvent], source: bytes, trivia: list[tuple[int, int]]
) -> AstEvent | None:
    if attr.rule == "inner_attribute":
        return min(
            (node for node in nodes if node.start <= attr.start and attr.end <= node.end),
            key=lambda node: (node.end - node.start, node.start), default=None,
        )
    return next(
        (node for node in sorted(
            (node for node in nodes if node.start >= attr.end),
            key=lambda node: (node.start, node.end - node.start),
        ) if _trivia_gap(source, attr.end, node.start, trivia)),
        None,
    )


def _opaque_record(
    path: str,
    event: AstEvent,
    identifiers: list[AstEvent],
    source: bytes,
    comments: list[tuple[int, int]],
    generated: set[str],
) -> dict[str, object] | None:
    controls = tuple(sorted({
        _normalize_identifier(identifier.text)
        for identifier in identifiers
        if event.start <= identifier.start and identifier.end <= event.end
        and _normalize_identifier(identifier.text) in LINT_CONTROLS
    }))
    if not controls:
        return None
    return {
        "path": path, "line": source[:event.start].count(b"\n") + 1,
        "kind": "opaque_macro", "lints": controls, "reason": None, "inner": False,
        "scope": "item", "target": "unstable", "category": _category(
            path, event.start, source, comments, generated
        ), "owner_stable": False,
    }


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
        commented_attribute = re.search(r"#\s*!?\s*\[\s*(?:r#)?(?:allow|expect)\b", comment)
        if not source[end:offset].strip() and commented_attribute is None:
            for category, marker in MARKERS.items():
                if marker in comment:
                    return category
    return "production"


def scan_sources(sources: list[tuple[str, str]], generated_paths: set[str]) -> list[dict[str, object]]:
    rustfmt_parse(sources)
    events_by_path = ast_grep_scan(sources)
    records: list[dict[str, object]] = []
    for path, source in sources:
        events = events_by_path.get(path, [])
        data = source.encode("utf-8")
        comments = _comments(events, data)
        attrs = [event for event in events if event.rule in {"outer_attribute", "inner_attribute"}]
        nodes = [event for event in events if event.rule not in {
            "outer_attribute", "inner_attribute", "identifier", "line_comment", "block_comment",
            "declaration_list",
        }]
        identifiers = [event for event in events if event.rule == "identifier"]
        bodies = [event for event in events if event.rule == "declaration_list"]
        trivia = sorted(comments + [(event.start, event.end) for event in attrs])
        info = {id(node): _target(node, source, bodies) for node in nodes}
        named = [node for node in nodes if info[id(node)][0] != "unstable"]
        attribute_owners = {
            id(attr): _owner_for_attribute(attr, nodes, data, trivia) for attr in attrs
        }
        configured_nodes = {
            id(owner) for attr in attrs
            if _is_configuration_attribute(attr)
            for owner in [attribute_owners[id(attr)]] if owner is not None
        }
        configured_crate = any(
            attr.rule == "inner_attribute" and _is_configuration_attribute(attr)
            and attribute_owners[id(attr)] is None for attr in attrs
        )

        for attr in attrs:
            for kind, lints, reason, inner in _attributes(attr):
                owner = attribute_owners[id(attr)]
                configuration = _is_configuration_attribute(attr) or configured_crate
                if inner:
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
                configuration = configuration or (
                    (owner is not None and id(owner) in configured_nodes)
                    or any(id(node) in configured_nodes for node in prefix_nodes)
                )
                stable = stable and not configuration
                records.append({
                    "path": path, "line": data[:attr.start].count(b"\n") + 1,
                    "kind": kind, "lints": lints, "reason": reason, "inner": inner,
                    "scope": scope, "target": target, "category": _category(path, attr.start, data, comments, generated_paths),
                    "owner_stable": stable,
                })
        for event in events:
            if event.rule in {"macro_definition", "macro_invocation"}:
                opaque = _opaque_record(path, event, identifiers, data, comments, generated_paths)
                if opaque is not None:
                    records.append(opaque)
    return records
