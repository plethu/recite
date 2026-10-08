# Recite Fixtures

Parser, compiler, runtime, CLI and LSP tests share the source files in this tree. `valid/` inputs
should parse, lower and validate without diagnostics; `invalid/` inputs exercise structured
failures. Parser and compiler expectations live in their crates' `tests/snapshots/` directories.

Keep reusable `.recite` inputs here rather than copying them into individual test suites. See
[contribution checks](../../CONTRIBUTING.md) for the repository commands.
