# recite-import

Migration helpers that return editable Recite source, source mappings and
structured reports. Native Recite validation gates generated output. See the
[migration guide](https://github.com/plethu/recite/blob/main/docs-site/src/content/docs/migration/importer-boundaries.md)
for the supported subsets and review workflow.

Licensed under MIT OR Apache-2.0.

## Maintainer ownership

Concrete format readers own their accepted subsets. Shared source generation
owns escaping, generated identifiers, source mappings and native validation.
The CLI owns filesystem access, report rendering and writes to a new destination.
Reports distinguish complete conversion, partial conversion requiring review and
invalid output; unsupported constructs retain provenance and an explicit action.
Validation success does not establish source-runtime parity.

Generated IDs are deterministic, and existing valid IDs are retained. After an
author edits imported source, native IDs belong to that source; rerunning an
importer is not incremental synchronization. The migration guide owns accepted
formats and limitations. [Historical delivery evidence](../../docs/archive/adoption-migration-design.md)
records the original implementation pass.
