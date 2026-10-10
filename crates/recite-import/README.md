# recite-import

Migration helpers that return editable Recite source, source mappings and structured reports. Native
Recite validation gates generated output. See the
[migration guide](https://github.com/plethu/recite/blob/main/docs-site/src/content/docs/migration/importer-boundaries.md)
for the supported subsets and review workflow.

Licensed under MIT OR Apache-2.0.

## Maintainer ownership

Concrete format readers own their accepted subsets. Shared source generation owns escaping,
generated identifiers, source mappings and native validation. The CLI owns filesystem access, report
rendering and writes to a new destination. The migration guide owns result states, ID adoption and
supported subsets. Import is a one-time source conversion, not a source-runtime compatibility layer
or an incremental synchronization protocol.
