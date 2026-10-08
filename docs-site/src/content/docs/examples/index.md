---
title: Examples
description: Copyable projects and checks for Recite authoring and engine integration.
template: splash
---

Start with the [complete CLI workflow](/examples/headless-cli/). Its maintained project includes
source, schema, localisation and a runtime fixture; the same project is exercised through the Bevy
companion.

For engine-specific setup, use the examples shipped with the [Bevy](/adapters/bevy/),
[Godot](/adapters/godot/) or [Unity](/adapters/unity/) package. Follow
[edit and refresh](/adapters/authoring/) to try an invalid edit, a corrected build and a new
session.

For migration, the small inputs in
[`fixtures/import`](https://github.com/plethu/recite/tree/main/fixtures/import) cover the documented
JSON, CSV, Twee, Ink and Yarn subsets. The [migration report guide](/migration/importer-boundaries/)
explains how to inspect the result before writing native source.

Run the focused checks from the repository root:

```sh
mise exec -- just engines workflow
cargo test --locked -p recite-import
cargo test --locked -p recite-cli --test migration
```
