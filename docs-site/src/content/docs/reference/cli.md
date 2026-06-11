---
title: CLI
description: Reference for the shipped recite CLI commands.
---

The `recite` binary is the headless surface of the toolchain: everything a CI
job, a writer, or an engine build step needs without an editor or adapter.
This page documents the shipped commands; design rationale lives in the
[production spec §13](https://codeberg.org/plethu/recite/src/branch/main/docs/recite-production-spec.md).

Path arguments accept one or more `.recite` files or directories containing
them. Commands taking `<PROJECT_ROOT>` expect a directory containing
`recite.project.toml`. Validation-style commands report structured diagnostics
with stable codes; runtime and filesystem failures exit non-zero with a CLI error
message.

## Authoring checks

### `recite validate <PATHS>...`

Validates dialogue source — syntax, structure, references — without writing
compiled output.

### `recite check-ids <PATHS>...`

Reports stable line and choice ID diagnostics: missing, draft, malformed, or
duplicate anchors.

### `recite check-markup [--schema <SCHEMA>] <PATHS>...`

Validates inline markup in prose, optionally against a schema manifest's
markup policy.

### `recite check-metadata --schema <SCHEMA> <PATHS>...`

Validates metadata keys and values against a schema manifest.

## Building

### `recite compile --output <OUTPUT> [--schema <SCHEMA>] <PATHS>...`

Compiles source dialogue into a deterministic MessagePack `.recitec` asset.
Identical validated input produces identical bytes.

### `recite validate-project <PROJECT_ROOT>`

Validates `recite.project.toml` and the compiled assets it references.

### `recite check-fresh <PROJECT_ROOT>`

Checks whether the project's compiled assets are fresh relative to current
source fingerprints, schema fingerprint, and compiler compatibility version.
Wire this into CI to catch stale assets.

### `recite watch <PROJECT_ROOT>`

Watches project inputs and rebuilds manifest assets on change — the authoring
loop companion to editor diagnostics.

## Localisation

### `recite extract [--output <OUTPUT>] [--schema <SCHEMA>] <PATHS>...`

Extracts gettext POT entries. Each entry's `msgctxt` is the stable anchor, and
comments carry file, block, speaker, and source-ID context for translators.

## Running

### `recite run --block <BLOCK> --fixture <FIXTURE> <ASSET>`

Runs a compiled asset headlessly. The TOML fixture supplies condition results,
prompt answers, and effect acknowledgement policy; output lists lines,
prompts, selections, endings, and collected deferred effects. A missing
fixture entry fails with the exact key the prompt expects.

### `recite trace [--metrics] --block <BLOCK> --fixture <FIXTURE> <ASSET>`

The same headless run emitted as deterministic JSON — the format for snapshot
tests, conformance checks, and CI. `--metrics` adds trace counters.

### `recite play [OPTIONS] --block <BLOCK> <ASSET>`

Interactive playback for writers. `--ui auto|tui|plain` selects the surface,
`--keymap standard|vim` the TUI bindings, and `--dialogue-locale` plus
repeatable `--dialogue-catalog LOCALE=PATH` preview translations through the
runtime locale provider.

## A typical CI sequence

```bash
recite validate dialogue/
recite check-ids dialogue/
recite check-metadata dialogue/ --schema schema/recite.schema.json
recite compile dialogue/ -o build/scenes.recitec --schema schema/recite.schema.json
recite check-fresh .
recite trace build/scenes.recitec --block main --fixture tests/golden.toml
```
