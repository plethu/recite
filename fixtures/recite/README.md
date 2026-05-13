# Recite Fixtures

This tree holds reusable Recite source fixtures for parser, compiler, CLI, and
future LSP tests.

- `valid/` contains sources expected to parse, lower, and validate without
  diagnostics.
- `invalid/` contains sources expected to produce stable structured
  diagnostics.
- Parser and compiler fixture expectations are stored as `insta` snapshots
  under each crate's `tests/snapshots/` tree.
- Shared fixture `.recite` inputs stay in this directory so parser, compiler,
  CLI, and future LSP tests can reuse the same source files.

Check commands:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```
