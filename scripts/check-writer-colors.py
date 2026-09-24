#!/usr/bin/env python3
"""Keep authored GUI colours behind the contrast-tested semantic palette."""
from pathlib import Path
import re
import sys

root = Path(__file__).resolve().parents[1] / "apps/writer/crates/freya/src"
# Material owns neutral lighting/shadows; palette owns all authored colours.
owners = {Path("design/palette.rs"), Path("design/material.rs")}
literal = re.compile(r"Color::(?:(?!TRANSPARENT\b)[A-Z][A-Z_]+\b|from_[a-z0-9_]+\(\s*[+-]?\d|parse\(|from_(?:hex|css|str)\()")
failures = []
for path in sorted(root.rglob("*.rs")):
    if path.relative_to(root) in owners or path.name == "tests.rs":
        continue
    source = path.read_text()
    for match in literal.finditer(source):
        line = source.count("\n", 0, match.start()) + 1
        failures.append(f"{path.relative_to(root)}:{line}: use a semantic palette colour")
if failures:
    print("\n".join(failures), file=sys.stderr)
    sys.exit(1)
print("Writer colour ownership checked; contrast ratios run in palette tests.")
