# Recite Tree-sitter grammar

This directory contains the syntax-only Tree-sitter feasibility grammar for
Recite source. It is intended for editor highlighting and structural tooling;
it is not a second Recite parser.

The Rowan parser, compiler, and LSP remain authoritative for source recovery,
stable IDs, references, schema, conditions, effects, markup validation, and
match exhaustiveness. In particular, this grammar does not use indentation to
decide ownership of a body or branch. It accepts partial editor buffers and
allows Tree-sitter recovery to expose `ERROR` nodes while the authoritative
tooling reports the real diagnostic.

The checked-in `src/parser.c` is generated from `grammar.js`. `queries/` holds
host-neutral captures; Neovim and Zed integrations choose how those captures
map to their own presentation APIs.

Run the local grammar checks from the repository root:

```sh
mise run check-tree-sitter
```

The check requires the Tree-sitter CLI used by the editor toolchain. It
regenerates the parser in a temporary directory, compares the generated files
with the checked-in parser, runs the corpus, verifies that the canonical corpus
source matches `fixtures/recite/valid/language_pressure.recite`, exercises the
required highlight captures, and checks recovery on incomplete input. The
grammar CLI is an editor-development tool only and is not a Recite runtime
dependency.
