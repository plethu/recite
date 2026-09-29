---
title: Inspecting an import
description: Inspect source mappings and losses before writing editable Recite source.
---

`recite import` reads one document and prints a JSON report. It does not write
files unless you supply `--output-dir`. The input family is explicit:
`json`, `csv`, `twee`, `ink` or `yarn`.

From the Recite checkout, after building the CLI:

```sh
recite import fixtures/import/passages.twee --from twee
recite import fixtures/import/passages.twee --from twee --output-dir migrated
```

The destination must not exist. A successful write creates `report.json` and
`imported.recite`; it never replaces existing source. A failed write can leave
a new, incomplete directory, which you should inspect before retrying elsewhere.
Reports also remain on stdout, so redirect them if you want to retain an
inspection without writing generated source.

## Read the result

The report has a version, source family and file, generated source, mappings,
import items and native validation diagnostics. Each mapping names the generated
record and line, its original ID when present, and its source location. Locations
use text spans, JSON Pointer paths, or CSV record row/column/header fields. CSV
row numbers count records, including the header; quoted multiline records do not
turn subsequent record numbers into physical line numbers.

| Status | Meaning | Writing |
| --- | --- | --- |
| `complete` | All encountered constructs in the documented subset converted and native validation passed. | Allowed into a new directory. |
| `partial` | Native validation passed, but the report records skipped or lossy constructs. | Requires `--accept-partial` as well as `--output-dir`. |
| `invalid` | Input/mapping errors, no supported blocks, or native validation failure. | Refused, including with `--accept-partial`. |

Import items carry the shared diagnostic record, provenance, construct, action
and follow-up. `RECITE_IMPORT001` rejects bad input or mappings;
`RECITE_IMPORT002` reports skipped constructs; `RECITE_IMPORT003` marks a lossy
conversion. Native diagnostics retain their usual codes and point into
`imported.recite`; use the generated-line mappings to relate them to the input.

Leading or trailing whitespace in dialogue and choice labels is reported as a
loss before normalization. These conversions are partial and require review
before writing, even when the normalized text passes native validation.

Inspection exits successfully for complete or partial results. Invalid results
exit 1. A write request for partial output without `--accept-partial` also exits 1.
Operational failures, including unreadable files or malformed mapping JSON,
are reported on stderr and may occur before a report exists.

## Review before adopting source

Read every skipped/lossy item against the original. Text readers hold back an
entire affected block when they encounter unsupported control flow, rather than
lifting its dialogue out of a condition. References to a held-back block fail
native target validation. The report retains candidate source even when it
cannot be written.

Existing valid Recite `label@anchor` IDs are retained; bare valid anchors are
retained with a generated label. Other IDs receive deterministic generated IDs
and an old-to-new mapping. Generation depends on the supplied input path and
record identity/location, so same-named files in different directories receive
different generated IDs. Use a stable project-relative path when invoking the
CLI, or pass `--source-id story/scene/input.twee` to keep the same IDs when the
checkout moves. The report's `file` and provenance use that source identity.
Renaming or rearranging an input can change generated IDs.
After adopting the source, edit that source and keep its anchors; import is not
an incremental synchronization command.

Pass `--schema path/to/schema.json` when checking against your game's schema.
Generated source goes through the native parser, compiler validation and compiler.
A complete report establishes the documented conversion, not compatibility with
another tool's runtime, save data or localisation database.

The readers deliberately exclude arbitrary scripts, runtime emulation and
implicit command mappings. Migrate conditions and effects manually into your
schema. No importer executes source-language commands.

The checked examples and expected reports live in
[`fixtures/import`](https://github.com/plethu/recite/tree/main/fixtures/import)
and the [import tests](https://github.com/plethu/recite/tree/main/crates/recite-import/tests).
