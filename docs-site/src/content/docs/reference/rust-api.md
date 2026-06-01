---
title: Rust API
description: Stable entry point for Recite Rustdoc links.
---

Rustdoc remains the API reference for Recite crates.

For local development, build the current workspace API reference with:

```bash
cargo doc --workspace --no-deps --open
```

Published crate API references will live on docs.rs:

- [`recite-core`](https://docs.rs/recite-core/)
- [`recite-parser`](https://docs.rs/recite-parser/)
- [`recite-compiler`](https://docs.rs/recite-compiler/)
- [`recite-runtime`](https://docs.rs/recite-runtime/)
- [`recite-cli`](https://docs.rs/recite-cli/)
- [`recite-lsp`](https://docs.rs/recite-lsp/)
- [`recite-fixturegen`](https://docs.rs/recite-fixturegen/)

Until crates are published, local Rustdoc is the reliable API reference.
