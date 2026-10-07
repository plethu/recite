---
title: JSON and CSV imports
description: Map explicit fields from flat records to validated Recite source.
template: splash
---

The JSON reader accepts an array of flat objects. The CSV reader accepts a header row followed by
records. Both require a JSON mapping file; names are exact and case-sensitive. This mapping matches
the checked examples:

```json
{ "block": "node", "text": "text", "id": "id", "speaker": "speaker", "target": "next" }
```

Only `block` and `text` mappings are required. Omit optional mappings when your input lacks those
fields. Every configured field must occur in each record and contain a string. Empty optional values
mean no ID, speaker or jump.

```sh
recite import fixtures/import/lines.json --from json --mapping fixtures/import/mapping.json
recite import fixtures/import/lines.csv --from csv --mapping fixtures/import/mapping.json
```

Each row/object emits a plain dialogue line. Consecutive records with the same block belong
together. A nonempty target ends that block with a static jump; `END` means termination. A block
cannot be reopened later in the input. Speakers must be native identifiers. Native validation
catches duplicate anchors and missing targets; `--schema` also checks the game's declarations.

Unmapped fields are reported, including their JSON Pointer or CSV column/header. Repeated JSON keys,
repeated CSV headers, multiple mappings to one field, missing fields and non-string mapped values
are rejected. Embedded scripts, nested choice arrays, conditions, effects, metadata, multiline text,
markup and interpolation need manual migration. Quoted CSV commas are supported by the CSV parser.
The report distinguishes record locations from physical text lines.

Use the [inspection and write workflow](/migration/importer-boundaries/) to review a partial result.
Neither reader infers semantics from familiar-looking names. Engine-native resources, spreadsheet
formatting/formulas, scenes and runtime save files are outside this input shape. Export a bounded
table explicitly or migrate those parts manually.
