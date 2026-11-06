---
title: CLI
description: Reference for the shipped recite CLI commands.
---

The `recite` binary is the headless surface of the toolchain: everything a CI job, a writer, or an
engine build step needs without an editor or adapter. This page covers common commands; `recite
--help` owns the full command and flag list, while design rationale lives in the
[production spec §13](https://github.com/plethu/recite/blob/main/docs/spec/build-cli.md#13-cli).

Path arguments accept one or more `.recite` files or directories containing them. Commands taking
`<PROJECT_ROOT>` expect a directory containing `recite.project.toml`. Validation-style commands
report structured diagnostics with stable codes; runtime and filesystem failures exit non-zero with
a CLI error message.

## Command map

Top-level commands act on dialogue source or compiled assets in the build pipeline. A noun group
exists where a thing has two or more operations of its own, so related work sits under one name:

| Group              | Commands                                                                                        |
| ------------------ | ----------------------------------------------------------------------------------------------- |
| `id`               | `check`, `assign`                                                                               |
| `schema`           | `inspect`, `export`, `check-fresh`                                                              |
| `project`          | `validate`, `check-fresh`                                                                       |
| Top-level commands | `validate`, `compile`, `extract`, `watch`, `run`, `trace`, `play`, `bench`, `import`, `explain` |

`recite <group>` lists its commands, and `recite <group> <command> --help` documents one.

## Authoring checks

### `recite validate [--schema <SCHEMA>] [--only <CATEGORY>] <PATHS>...`

Validates dialogue source without writing compiled output: syntax, structure, and references.

With `--schema`, it also checks metadata, markup and declarations against a schema manifest.
`--only markup` or `--only metadata` reports just that diagnostic category; syntax errors are always
reported because they stop validation. `--only metadata` requires `--schema`.

### `recite id check [--since <REF>] [--allow-removed <ID>]... <PATHS>...`

Reports stable line and choice ID diagnostics: missing, malformed, or duplicate IDs.

With `--since`, it also fails for every ID that existed under the same paths at that git revision
but is now gone, because translations and saves keyed on a shipped ID no longer match. Use your last
release tag, and pass `--allow-removed <ID>` for each line you removed on purpose. Moving a line to
another file keeps its ID. If the revision is not available, for example in a shallow CI clone
without tags, the command fails instead of comparing against nothing.

### `recite id assign <PATHS>...`

Appends a generated stable ID to every line and choice header that has none, after the header's last
field. Existing IDs are never rewritten. If a file has syntax errors or a malformed ID, it reports
those diagnostics and leaves every file unchanged.

## Schema

### `recite schema inspect <SCHEMA>`

Projects a standalone schema TOML or generated schema manifest JSON to a versioned, deterministic
JSON summary on stdout. The input extension selects the authoritative loader: `.toml` uses
standalone source loading and `.json` uses the generated-manifest loader. Generated manifests are
read-only; this command does not invoke producers or write files. The JSON retains typed ownership,
freshness and provenance; machine paths use an explicit encoding rather than lossy text conversion.

### `recite schema export --schema <SCHEMA> --output <OUTPUT>`

Engine authoring tools can add paired `--producer-kind <KIND> --producer-id <STABLE-ID>` flags. The
canonical export then records the engine resource as its producer, with a fingerprint derived from
that identity and validated schema content. The default export preserves the standalone source
producer.

Exports standalone schema TOML or a generated manifest JSON as validated, canonical manifest JSON.
Engine schema producers can use the public `recite_core::schema::export_schema_manifest_json`
function for typed Rust declarations and the same canonical serializer. The command rejects an
output path that aliases its input and replaces the output only after validation. `--output-format
structured` reports versioned command records with diagnostics or artifact metadata for authoring
tools; their `command` field is `schema export`.

### `recite schema check-fresh --expected <EXPECTED> --actual <ACTUAL>`

Compares the schema producer fingerprints of a previously exported manifest with a current export
and reports whether the producer is fresh.

## Building

### `recite compile --output <OUTPUT> [--schema <SCHEMA>] <PATHS>...`

Compiles source dialogue into a deterministic MessagePack `.recitec` asset. Identical validated
input produces identical bytes when compile options, including the output asset ID/path, are the
same.

### `recite project validate <PROJECT_ROOT>`

Validates `recite.project.toml` and the compiled assets it references.

### `recite project check-fresh <PROJECT_ROOT>`

Checks whether the project's compiled assets are fresh relative to current source fingerprints,
schema fingerprint, and compiler compatibility version. Wire this into CI to catch stale assets.

### `recite watch <PROJECT_ROOT>`

Watches project inputs and rebuilds manifest assets on change. This is the authoring loop companion
to editor diagnostics.

## Localisation

### `recite extract [--output <OUTPUT>] [--schema <SCHEMA>] <PATHS>...`

Extracts gettext POT entries. Each entry's `msgctxt` is the stable ID, and comments carry file,
block and speaker context for translators.

## Running

### `recite run --block <BLOCK> --fixture <FIXTURE> <ASSET>`

Runs a compiled asset headlessly. The TOML fixture supplies condition results, prompt answers, and
effect acknowledgement policy; output lists lines, prompts, selections, endings, and collected
deferred effects. A missing fixture entry fails with the exact key the prompt expects.

### `recite trace [--metrics] --block <BLOCK> --fixture <FIXTURE> <ASSET>`

Emits the same headless run as deterministic JSON, the default format for snapshot tests,
conformance checks, and CI. `--metrics` adds instrumentation, including timing fields that are not
snapshot-stable.

### `recite play [OPTIONS] --block <BLOCK> <ASSET>`

Interactive playback for writers. `--ui auto|tui|plain` selects the surface, `--keymap standard|vim`
the TUI bindings, and `--dialogue-locale` plus repeatable `--dialogue-catalog LOCALE=PATH` preview
translations through the runtime locale provider.

`--ui auto` uses the TUI only when stdin and stdout are interactive terminals; otherwise it falls
back to `--ui plain`. Use `--ui plain` for screen readers, scripts, pipes, and CI. Plain mode keeps
the same ordered runtime event stream as the TUI and emits prompts, choices, conditions, effects,
acknowledgements, deferred effects, and end state as line-oriented text.

Interactive UI preferences live in `$RECITE_CONFIG`, `$XDG_CONFIG_HOME/recite/config.toml`, or
`~/.config/recite/config.toml`, not in project manifests:

```toml
[ui]
keymap = "standard"      # "standard" or "vim"
key_hints = "contextual" # "contextual", "compact", or "hidden"
color = "auto"           # "auto", "always", or "never"
contrast = "standard"    # "standard" or "accessible"

[play]
show_unavailable_choices = true
```

With `color = "auto"`, the TUI disables color when `NO_COLOR` is present or `CLICOLOR=0`. `color =
"always"` overrides those environment variables, and `color = "never"` disables color. `contrast =
"accessible"` selects a higher-contrast TUI palette when color is enabled. Color is never the only
meaning carrier: selected choices keep a `>` marker, unavailable choices keep textual
unavailable/reason text, condition rows keep `yes`/`no` labels, and prompt, effect, transcript, and
footer labels remain visible without color.

Required play actions are reachable through typed input: choices accept ID or index, conditions
accept typed values, and blocking effects accept Enter or `ack`.

## A typical CI sequence

```bash
recite validate dialogue/
recite id check dialogue/
recite validate dialogue/ --only metadata --schema schema/recite.schema.json
recite compile dialogue/ -o build/scenes.recitec --schema schema/recite.schema.json
recite project check-fresh .
recite trace build/scenes.recitec --block main --fixture tests/golden.toml
```

## Migration inspection

`recite import INPUT --from json|csv|twee|ink|yarn` prints a versioned JSON report and candidate
source. JSON/CSV requires `--mapping mapping.json`. Use `--schema manifest.json` to validate against
game declarations. See [import inspection](/migration/importer-boundaries/) for destination
handling, result states, partial conversion, stable import namespaces and cross-file target
mappings.
