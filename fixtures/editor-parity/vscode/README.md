# VS Code syntax fixtures

The TextMate projection is exercised against the canonical Recite source
fixtures rather than copied examples:

- `fixtures/recite/valid/core_language_spike.recite` covers directives,
  stable anchors, conditions, effects, interpolation, and comments.
- `fixtures/recite/valid/language_pressure.recite` covers Unicode labels,
  metadata, plural prose, markup, and runtime bindings.
- `fixtures/recite/invalid/parser_marker_leading_prose.recite` covers
  malformed marker-leading prose while retaining the source owned by the
  parser fixtures.

`incomplete.recite` is the one derived editor buffer. It deliberately leaves an
anchor, interpolation, choice anchor, and divert target unfinished. The
grammar must keep those lexical regions editable; parser/compiler/LSP
diagnostics remain the correctness authority.
