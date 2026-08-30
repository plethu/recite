"""Bounded interpreter for Rust lint-suppression attribute metadata."""

from __future__ import annotations


class MetadataError(ValueError):
    """Suppression metadata does not match the bounded attribute grammar."""


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
        raise MetadataError("malformed suppression attribute")
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
                raise MetadataError("malformed suppression attribute comment")
            continue
        output.append(char)
        index += 1
    if quote is not None:
        raise MetadataError("malformed suppression attribute literal")
    return "".join(output)


def _literal(value: str) -> str | None:
    value = value.strip()
    if len(value) < 2 or value[0] != '"':
        return None
    index = 1
    while index < len(value):
        if value[index] == "\\":
            return None
        if value[index] == '"':
            return value[1:index] if not value[index + 1:].strip() else None
        index += 1
    return None


def _meta(body: str) -> list[tuple[str, tuple[str, ...], str | None]]:
    body = _without_comments(body).strip()
    if "(" not in body or not body.endswith(")"):
        if body in {"allow", "expect"}:
            raise MetadataError(f"malformed {body} suppression attribute")
        return []
    name, payload = body.split("(", 1)
    name, payload = name.strip(), payload[:-1]
    if name == "cfg_attr":
        pieces = _split_top_level(payload)
        if len(pieces) < 2:
            raise MetadataError("malformed cfg_attr suppression attribute")
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
                raise MetadataError(f"malformed {name} suppression reason")
            reason = _literal(piece[equals + 1:])
            if reason is None:
                raise MetadataError(f"malformed {name} suppression reason")
            continue
        normalized = "".join(piece.split())
        if not normalized or not all(part.isidentifier() for part in normalized.split("::")):
            raise MetadataError(f"malformed {name} suppression lint")
        lints.append(normalized)
    if not lints:
        raise MetadataError(f"malformed {name} suppression attribute")
    return [(name, tuple(sorted(set(lints))), reason)]


def attributes(text: str, inner: bool) -> list[tuple[str, tuple[str, ...], str | None, bool]]:
    prefix = "#![" if inner else "#["
    if not text.startswith(prefix) or not text.endswith("]"):
        raise MetadataError("malformed Rust attribute")
    return [(kind, lints, reason, inner) for kind, lints, reason in _meta(text[len(prefix):-1])]
