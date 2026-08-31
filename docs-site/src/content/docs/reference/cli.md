---
title: CLI
description: Reference for the shipped recite CLI commands.
---

The `recite` binary is the headless surface of the toolchain: everything a CI
job, a writer, or an engine build step needs without an editor or adapter.
This page documents the shipped commands; design rationale lives in the
[production spec §13](https://github.com/plethu/recite/blob/main/docs/recite-production-spec.md).

Path arguments accept one or more `.recite` files or directories containing
them. Commands taking `<PROJECT_ROOT>` expect a directory containing
`recite.project.toml`. Validation-style commands report structured diagnostics
with stable codes; runtime and filesystem failures exit non-zero with a CLI error
message.

## Authoring checks

### `recite validate <PATHS>...`

Validates dialogue source without writing compiled output: syntax, structure, and
references.

### `recite check-ids <PATHS>...`

Reports stable line and choice ID diagnostics: missing, draft, malformed, or
duplicate anchors.

### `recite check-markup [--schema <SCHEMA>] <PATHS>...`

Validates inline markup in prose, optionally against a schema manifest's
markup policy.

### `recite check-metadata --schema <SCHEMA> <PATHS>...`

Validates metadata keys and values against a schema manifest.

### `recite inspect-schema <SCHEMA>`

Projects a standalone schema TOML or generated schema manifest JSON to a
versioned, deterministic JSON summary on stdout. The input extension selects
the authoritative loader: `.toml` uses standalone source loading and `.json`
uses the generated-manifest loader. Generated manifests are read-only; this
command does not invoke producers or write files. The projection includes
schema ownership, producer identity, scoped fingerprints, available freshness
channels, declaration origins, capabilities, and producer action evidence.

## Building

### `recite compile --output <OUTPUT> [--schema <SCHEMA>] <PATHS>...`

Compiles source dialogue into a deterministic MessagePack `.recitec` asset.
Identical validated input produces identical bytes when compile options,
including the output asset ID/path, are the same.

### `recite validate-project <PROJECT_ROOT>`

Validates `recite.project.toml` and the compiled assets it references.

### `recite check-fresh <PROJECT_ROOT>`

Checks whether the project's compiled assets are fresh relative to current
source fingerprints, schema fingerprint, and compiler compatibility version.
Wire this into CI to catch stale assets.

### `recite watch <PROJECT_ROOT>`

Watches project inputs and rebuilds manifest assets on change. This is the
authoring loop companion to editor diagnostics.

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

Emits the same headless run as deterministic JSON, the default format for
snapshot tests, conformance checks, and CI. `--metrics` adds instrumentation,
including timing fields that are not snapshot-stable.

### `recite play [OPTIONS] --block <BLOCK> <ASSET>`

Interactive playback for writers. `--ui auto|tui|plain` selects the surface,
`--keymap standard|vim` the TUI bindings, and `--dialogue-locale` plus
repeatable `--dialogue-catalog LOCALE=PATH` preview translations through the
runtime locale provider.

`--ui auto` uses the TUI only when stdin and stdout are interactive terminals;
otherwise it falls back to `--ui plain`. Use `--ui plain` for screen readers,
scripts, pipes, and CI. Plain mode keeps the same ordered runtime event stream
as the TUI and emits prompts, choices, conditions, effects, acknowledgements,
deferred effects, and end state as line-oriented text.

Interactive UI preferences live in `$RECITE_CONFIG`,
`$XDG_CONFIG_HOME/recite/config.toml`, or `~/.config/recite/config.toml`, not in
project manifests:

```toml
[ui]
keymap = "standard"      # "standard" or "vim"
key_hints = "contextual" # "contextual", "compact", or "hidden"
color = "auto"           # "auto", "always", or "never"
contrast = "standard"    # "standard" or "accessible"

[play]
show_unavailable_choices = true
```

With `color = "auto"`, the TUI disables color when `NO_COLOR` is present or
`CLICOLOR=0`. `color = "always"` overrides those environment variables, and
`color = "never"` disables color. `contrast = "accessible"` selects a
higher-contrast TUI palette when color is enabled. Color is never the only
meaning carrier: selected choices keep a `>` marker, unavailable choices keep
textual unavailable/reason text, condition rows keep `yes`/`no` labels, and
prompt, effect, transcript, and footer labels remain visible without color.

Required play actions are reachable through typed input: choices accept ID or
index, conditions accept typed values, and blocking effects accept Enter or
`ack`.

## A typical CI sequence

```bash
recite validate dialogue/
recite check-ids dialogue/
recite check-metadata dialogue/ --schema schema/recite.schema.json
recite compile dialogue/ -o build/scenes.recitec --schema schema/recite.schema.json
recite check-fresh .
recite trace build/scenes.recitec --block main --fixture tests/golden.toml
```
