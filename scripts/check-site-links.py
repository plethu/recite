#!/usr/bin/env python3
"""Check local links and fragments in the built documentation site."""

from __future__ import annotations

import sys
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit


class Page(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.links: list[str] = []
        self.anchors: set[str] = set()

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        values = dict(attrs)
        if values.get("id"):
            self.anchors.add(values["id"])
        if tag == "a":
            if values.get("name"):
                self.anchors.add(values["name"])
            if values.get("href"):
                self.links.append(values["href"])


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: check-site-links.py DIST_DIR", file=sys.stderr)
        return 2

    root = Path(sys.argv[1]).resolve()
    if not (root / "index.html").is_file():
        print(f"missing built site: {root}", file=sys.stderr)
        return 2

    pages: dict[Path, Page] = {}
    for file in root.rglob("*.html"):
        page = Page()
        page.feed(file.read_text(encoding="utf-8"))
        pages[file] = page

    errors: list[str] = []
    for source, page in pages.items():
        for href in page.links:
            url = urlsplit(href)
            if url.scheme or url.netloc:
                continue
            path = unquote(url.path)
            if not path:
                target = source
            else:
                base = root if path.startswith("/") else source.parent
                target = (base / path.lstrip("/")).resolve()
            if target.is_dir():
                target /= "index.html"
            elif not target.is_file() and not target.suffix:
                target /= "index.html"

            location = source.relative_to(root)
            if not target.is_relative_to(root) or not target.is_file():
                errors.append(f"{location}: {href} (missing target)")
            elif url.fragment and target.suffix == ".html":
                target_page = pages.get(target)
                if target_page and unquote(url.fragment) not in target_page.anchors:
                    errors.append(f"{location}: {href} (missing fragment)")

    for error in errors:
        print(error, file=sys.stderr)
    print(f"Checked {len(pages)} HTML pages; {len(errors)} broken local links.")
    return bool(errors)


if __name__ == "__main__":
    raise SystemExit(main())
