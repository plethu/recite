---
title: Install
description: Build the Recite CLI from source and verify the toolchain.
---

Recite is pre-release. An old `recite` 0.0.1 snapshot exists on crates.io, but the current
`recite-cli` and `recite-lsp` packages are not published, so install them from source. The workspace
requires Rust 1.96 or newer; use the pinned `.mise.toml` toolchain when reproducing checks.
[rustup](https://rustup.rs/) is the usual way to get one.

Install the `recite` binary straight from the repository:

```bash
cargo install --locked --git https://github.com/plethu/recite recite-cli
```

Or from a clone, which is the better path if you also want the fixtures and examples:

```bash
git clone https://github.com/plethu/recite
cd recite
cargo install --locked --path crates/recite-cli
```

Verify the install:

```bash
recite --version
```

The language server installs the same way when you want editor diagnostics:

```bash
cargo install --locked --git https://github.com/plethu/recite recite-lsp
```

Because Recite is pre-release, on-disk formats and CLI flags may still change between commits;
recompile your `.recitec` assets with the same toolchain version you author with, and see the
[CHANGELOG](https://github.com/plethu/recite/blob/main/CHANGELOG.md) for what's moving.

Next: write and run a scene in [First Scene](/getting-started/first-scene/).

For Writer, Nix, editor and engine packages, use the
[package preparation guide](/guides/distribution/). Current artifacts can be built and tested
locally before the first full release.
