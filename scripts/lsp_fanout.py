"""Deterministic shared-destination fixture for project invalidation measurements."""

from pathlib import Path


def generate(root: Path, *, documents=100, blocks=20, lines=10, shared_destinations=10):
    if not (documents >= shared_destinations >= 1 and blocks >= 1 and lines >= 1):
        raise ValueError("fanout requires documents >= destinations >= 1 and positive block/line counts")
    source = root / "src"
    source.mkdir(parents=True)
    (root / "recite.project.toml").write_text(
        'format_version = 1\n[project]\ncontent_set = "lsp-fanout"\nversion = "1"\n'
        '[[scenes]]\nid = "fanout"\nasset = "build/fanout.recitec"\nblock = "shared"\n')
    for document in range(documents):
        text = ":: shared default\n-> END\n" if document == 0 else ""
        for block in range(blocks):
            text += f":: scene_{document:03}_{block:03}\n"
            for line in range(lines):
                identity = document * blocks * lines + block * lines + line
                text += f"> line@{identity:020x}\n  Shared destination fixture prose.\n"
            target = block % shared_destinations
            text += f"-> src/shard-{target:03}.recite::scene_{target:03}_000\n"
        (source / f"shard-{document:03}.recite").write_text(text)
