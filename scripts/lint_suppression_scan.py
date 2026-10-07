"""Pinned rustfmt and ast-grep parsing; no policy decisions."""

from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path

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


def _rule_text() -> str:
    return "\n---\n".join(
        f"id: {rule_id}\nlanguage: Rust\nrule:\n  {rule}" for rule_id, rule in RULES.items()
    )


def ast_grep_scan(sources: list[tuple[str, str]]) -> dict[str, list[AstEvent]]:
    if not sources:
        return {}
    executable = shutil.which("ast-grep")
    if executable is None:
        raise ParseError(
            "missing required tool: ast-grep (run the maintainability mise environment)"
        )
    with tempfile.TemporaryDirectory(prefix="recite-lint-ast-") as temporary:
        root = Path(temporary)
        for path, source in sources:
            destination = root / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(source, encoding="utf-8")
        result = subprocess.run(
            [
                executable,
                "scan",
                "--inline-rules",
                _rule_text(),
                "--json=stream",
                "--no-ignore",
                "hidden",
                str(root),
            ],
            capture_output=True,
            text=True,
            check=False,
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
                    executable,
                    "--edition",
                    "2024",
                    "--config",
                    "skip_children=true",
                    "--emit",
                    "stdout",
                    str(destination),
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
                text=True,
                check=False,
            )
            if result.returncode:
                detail = next(
                    (line for line in result.stderr.splitlines() if line.startswith("error:")),
                    "parse error",
                )
                raise ParseError(f"rustfmt rejected Rust syntax in {path}: {detail}")
