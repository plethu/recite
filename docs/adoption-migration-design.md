# Adoption and migration implementation

This integration covers #38, #57, #60 and #99–104. Product positioning (#56)
and publication remain outside this pass.

## Owners

`recite-import` owns migration reports, provenance, explicit mappings, bounded
format readers and source generation. It calls the existing parser/compiler
validation; it does not add runtime behavior or another compiled format.
Format readers are concrete modules. Shared source generation owns escaping,
generated identifiers and native validation. The CLI owns filesystem access,
report rendering and writing to a new destination directory.

Reports distinguish complete conversion, partial conversion requiring review,
and invalid generated output. They retain generated source for inspection even
when validation prevents writing a `.recite` file. Every generated record has
source provenance; unsupported constructs have a diagnostic and an explicit
action. A successful native validation is not a claim of source-runtime parity.

JSON/CSV mappings name fields explicitly and reject ambiguous mappings. Text
readers accept documented subsets; Ink and Yarn initially favor inspection
where control-flow or expression semantics cannot be preserved. Generated IDs
are deterministic and recorded, and existing valid IDs are retained. Once the
author edits imported source, its native IDs belong to that source: rerunning
an importer is not an incremental synchronization workflow.

## Evidence

The maintained realistic fixture pack is the complete-workflow template.
Clean-copy checks run the documented commands and verify deterministic traces,
localisation extraction and freshness. Existing engine checks own host-specific
refresh/restart and package behavior. Technical guides link these checks instead
of duplicating engine integration infrastructure.

Importer fixtures cover complete and partial conversion, malformed input,
ambiguous mappings, identifiers, provenance, unsupported constructs, invalid
targets and native validation. Distribution instructions name existing package
builders, upgrade checks, signing status and remaining platform acceptance.
Final release acceptance stays with #79/#81 and the serious-v1 gate.

## Implementation limits and review

The first readers intentionally accept flat JSON/CSV records; plain Twee
passages with trailing standalone links; Ink knots with static diverts and
sticky menu-only choices; and plain Yarn nodes with speaker prefixes, line IDs
and static jumps. Other constructs are inspection findings. Unsupported text
control flow holds back its entire block. No schema-mapped command conversion
is claimed in this subset; authors add native conditions/effects explicitly.

The source builder is above the 250-line review trigger. It remains one cohesive
owner for generated block/session shape, source IDs, source mappings and the
native validation gate; the independent format readers and diagnostic contract
are separate modules. Splitting its small state transitions across files would
make that invariant harder to inspect. The CLI dispatcher crossed the 400-line follow-up threshold with its new
arm, so benchmark commands moved to their own cohesive module. The argument,
help, error and structured-error inventories remain cohesive declarative
command contracts; migration adds entries without new policy branches.

The realistic project manifest now uses the current `content_set` field and
declares its scene. Its obsolete `[locales]` table was removed; catalogue
selection already belongs to `runtime-fixture.toml`. The checked fixture digest
was regenerated without changing authored dialogue, IDs or catalogue content.
