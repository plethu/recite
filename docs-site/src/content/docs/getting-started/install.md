---
title: Install
description: Build the Recite CLI from source and verify the toolchain.
---

Recite is pre-release and not yet published to crates.io or package managers,
so the CLI installs from source. You need a Rust toolchain at or above the
workspace `rust-version` (currently 1.96); [rustup](https://rustup.rs/) is the
usual way to get one.

Install the `recite` binary straight from the repository:

```bash
cargo install --git https://codeberg.org/plethu/recite recite-cli
```

Or from a clone, which is the better path if you also want the fixtures and
examples:

```bash
git clone https://codeberg.org/plethu/recite
cd recite
cargo install --path crates/recite-cli
```

Verify the install:

```bash
recite --version
```

The language server installs the same way when you want editor diagnostics:

```bash
cargo install --git https://codeberg.org/plethu/recite recite-lsp
```

Because Recite is pre-release, on-disk formats and CLI flags may still change
between commits; recompile your `.recitec` assets with the same toolchain
version you author with, and see the
[CHANGELOG](https://codeberg.org/plethu/recite/src/branch/main/CHANGELOG.md)
for what's moving.

Next: write and run a scene in [First Scene](/getting-started/first-scene/).
