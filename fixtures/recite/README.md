# Recite Fixtures

This tree holds reusable Recite source fixtures for parser, compiler, CLI, and
future LSP tests.

- `valid/` contains sources expected to parse, lower, and validate without
  diagnostics.
- `invalid/` contains sources expected to produce stable structured
  diagnostics.
- Each `.recite` source with diagnostic coverage has a sibling
  `.diagnostics.txt` file.

Check commands:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```
