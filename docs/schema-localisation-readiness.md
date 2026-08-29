# Schema and localisation readiness

This is the checked-in evidence record for [#166](https://github.com/plethu/recite/issues/166),
audited against `f72cb819` on 2026-08-27. It records the current implementation
boundary and the work still needed for Milestone 2. The [production
specification](recite-production-spec.md) remains normative; this document is
the cross-surface audit, not a second language or schema specification.

## Authority and settled direction

The source, canonical schema model, compiler, runtime, and client boundaries
remain separate. The following decisions are settled for this readiness pass:

- A versioned TOML document is the authoritative standalone schema source. It
  lowers directly to `recite-core::schema::ProjectSchema`.
- JSON is optional generated, read-only interchange/export for external or
  engine producers. It is never a mandatory intermediate or a hand-maintained
  shadow authority.
- A producer-content freshness fingerprint answers whether generated content is
  stale. It is distinct from the semantic schema fingerprint.
- Producer provenance is useful diagnostic context, but diagnostic-only
  provenance is excluded from the semantic schema fingerprint.
- Named interpolation and plural lines are core v1 behaviour, across source,
  extraction, catalogues, runtime delivery, diagnostics, and clients.
- Gettext PO is the required dialogue-catalogue editing path and must be
  lossless for untouched structure and metadata. It remains separate from the
  Fluent UI path.
- All first-party Recite-owned UI text uses one shared Fluent resource contract
  across CLI/TUI, LSP, editor extensions, and the GUI. Host-required metadata
  remains host-owned.
- `en-US` is the sole supported launch UI locale. A checked-in or embedded
  resource for another locale is not a launch-support claim without human
  authorship, review, and the #181 completeness gate.
- Core structured diagnostics remain locale-neutral. A client may localise
  presentation at its boundary without changing codes, fields, spans, values,
  trace records, or machine-readable output.

Project and user configuration ownership belongs to [#167](https://github.com/plethu/recite/issues/167);
the UI-free shared authoring and preview boundary belongs to [#168](https://github.com/plethu/recite/issues/168).
Neither issue changes the semantic authorities recorded here.

Readiness uses three terms below: **baseline** means there is implementation
and focused evidence; **partial** means a useful slice exists but the v1
contract is incomplete; **gap** means the owner issue still needs to establish
the capability.

## Schema and generated-artifact matrix

| Contract slice | Evidence at `f72cb819` | Readiness and closure owner |
| --- | --- | --- |
| Canonical model | `crates/recite-core/src/schema/model/` defines `ProjectSchema`, typed declarations, metadata domains, availability reasons, and presentation projection data. `BTreeMap`/`BTreeSet` collection ownership and `schema_manifest/canonical_order.rs` plus `fingerprint.rs` provide deterministic ordering evidence. | **Baseline.** Keep semantic validation and canonical fingerprinting in `recite-core`; complete the cross-format agreement in #176. |
| Raw manifest and public JSON shape | `RawManifest` uses named-entry deserialisation that preserves declaration order for diagnostics and `deny_unknown_fields`; loader diagnostics carry source spans. `fixtures/schema/valid/`, `fixtures/schema/invalid/`, and `recite-core/tests/schema_manifest/**` cover the loader's current JSON inputs. `schemas/recite-schema-manifest-v1.schema.json` drifts from the loader: it omits availability reasons and projection sections and does not yet describe producer metadata beyond an untyped `origin` string. No test currently validates the fixtures against the published JSON Schema. | **Partial, with visible drift.** Reconcile required/optional fields, unknown-field policy, versions, and generated JSON shape in #176, and add explicit public JSON-Schema validation coverage. |
| Semantic lowering and validation | `load_schema_manifest_str` lowers JSON into the canonical model; schema diagnostics cover malformed shape, versions, duplicate definitions, type/domain references, metadata domains, availability reasons, and projection declarations. Compiler, CLI, and LSP consume `ProjectSchema`, not JSON directly. | **Baseline for existing JSON.** Add the reconciliation and mismatch fixtures owned by #176; do not move semantic authority into JSON Schema or a client. |
| Provenance | Registry definitions currently carry an optional string `origin`, and LSP summaries surface registry origins. Availability-reason origins load into `ProjectSchema` but are not surfaced by the LSP summary. `docs/engine-adapter-contract.md` describes richer origins, context/value origins, and producer fingerprints, but those types are not in `RawManifest` or `ProjectSchema`. | **Gap.** #176 defines typed producer provenance and its diagnostic/stale-output use. Missing provenance must not make valid dialogue invalid. |
| Semantic fingerprint | `ProjectSchema::canonical_fingerprint` is deterministic and BLAKE3-backed. Existing tests prove semantic changes alter it, but the current canonical bytes include the available `origin` fields. | **Partial and currently contrary to the settled direction.** #176 must exclude diagnostic-only provenance while retaining every semantic declaration in the fingerprint. Add a provenance-only-change stability fixture. |
| Standalone producer | `recite-cli` project loading currently points at a generated JSON schema manifest. There is no versioned standalone schema-source loader/editor or direct TOML-to-`ProjectSchema` path. | **Gap.** #177 owns the source-owning TOML producer, source spans, version/validation diagnostics, and optional deterministic JSON export. No issue may introduce a second schema model. |
| Engine/adapter producer | The host-agnostic producer and freshness contract is documented in `docs/engine-adapter-contract.md` §7; adapter conformance schemas exist under `fixtures/adapter-conformance/v1/`. There is no Recite-owned engine producer implementation in this snapshot. | **Partial contract baseline.** #176 establishes the shared manifest/provenance contract; #177 covers the standalone producer. Concrete engine integrations remain adapter work. |
| Freshness | `recite check-fresh`, `ProjectFreshnessInput`, and `validate_project_freshness` compare embedded source fingerprints, the canonical schema fingerprint, and compiler compatibility. Watch tests cover source and schema changes. | **Partial.** This is compiled-asset freshness, not producer-content freshness. #176 must define producer fingerprints and stale generated-schema reporting without conflating either fingerprint with semantic compatibility. |
| Consumers | Compiler validation/compilation, CLI project validation, LSP `SchemaIndex`, schema summaries, and runtime asset headers all use the canonical model or its semantic fingerprint. | **Baseline with missing metadata propagation.** #176 must ensure compiler, LSP, CLI, and adapter-facing summaries preserve producer identity and structured freshness state. |

## Dialogue-content localisation matrix

| Contract slice | Evidence at `f72cb819` | Readiness and closure owner |
| --- | --- | --- |
| Stable IDs, context, and POT | Source IDs and spans are retained in the core AST and compiled rows. `recite-compiler/src/pot.rs` emits deterministic line/choice entries, source IDs, file/block/speaker comments, references, speaker display names, availability-reason templates, and presentation-label templates. `tests/pot_extraction.rs` and snapshots cover the current entries and escaping. | **Partial.** The extraction path is useful and source-preserving for its current surface; #178–#180 must extend it to lossless PO-compatible variants, interpolation, and plural entries. |
| Source and rendered text | `SourceText` retains text and span; compiled lines/choices and runtime `DialogueLine`/`DialogueChoice` retain `source_text` separately from resolved `text`. Availability-reason output retains source and rendered forms. | **Baseline for existing singular text.** Keep source text unsubstituted and independently observable as interpolation and plural work lands in #179–#180. |
| PO parsing and editing | `recite-core::PoDocument` owns lossless parsing/editing; the CLI projects active singular and plural entries, validates headers/arms/placeholders, and reports malformed input with a path and line. Comments and unsupported entries remain excluded from runtime lookup without altering the source document. | **Baseline plus #180 plural projection.** #178 owns lossless parsing, targeted edits, source structure/metadata preservation, external-change detection, structured conflicts, and atomic writes. |
| Variants | Runtime `LocaleResolution` carries an explicit variant; runtime tests cover variant-first then base-ID lookup. The CLI provider implements `id&variant` then `id` lookup over its locale chain. There is no source/catalog extraction model for variant-bearing entries or shared client contract for selecting them. | **Partial.** Preserve explicit selection and deterministic priority while #178–#180 make catalogue and client records complete; no client may infer a variant. |
| Interpolation | `recite-core` re-exports the named-placeholder helpers `extract_placeholder_names` and `validate_translation_placeholders` (implemented in `crates/recite-core/src/text.rs`). Schema availability-reason templates validate placeholders, and the runtime renders typed availability-reason arguments. Ordinary line/choice source text has no complete binding/substitution path through AST, compiled asset, POT/PO, and runtime. | **Partial.** #179 owns the complete named interpolation pipeline, including escapes, repeated names, typed bindings, translation diagnostics, and separate source/rendered values. |
| Plural lines | Parser and compiled rows preserve exactly two source forms. POT emits locale-neutral `msgid`/`msgid_plural` templates with empty arms; translated PO loading validates its own `Plural-Forms` and all declared arms; runtime uses one structured provider resolution over the shared bounded gettext evaluator and composes locale, variant, and English source fallback. Runtime output, FFI, Godot, and CLI trace preserve source forms, count, matched entry, ordered attempts, selected arm, and terminal fallback provenance. | **Implemented in #180 with focused parser, POT/PO, provider, runtime, trace, and adapter-conversion evidence.** Keep the explicit POT-versus-PO contract and one-call resolution boundary stable as clients mature. |
| Inline markup | Source text retains markup, and the shared `recite-core` markup policy now drives compiler validation and lossless PO translation validation. PO entries preserve source structure while rejecting missing required tags, newly introduced tags, unbalanced translated tags, and changed tag attributes; compiler diagnostics retain source spans and PO diagnostics retain translated-field spans. POT preserves the source string. Core and CLI tests cover valid reordered prose, each failure class, lossless metadata retention, and runtime delivery. | **Partial.** The translated catalogue boundary is covered; #179–#180 still need to keep markup valid through interpolation and plural-form projection. |
| Locale fallback | `DialogueCatalogProvider` uses deterministic BCP-47 region truncation, then source text; CLI trace records the locale candidate chain only. It does not yet identify the matched catalogue entry or terminal source fallback. CLI tests cover region and intermediate-language fallback, source-only mode, and distinct line/choice/reason domains. | **Partial.** The CLI baseline is not yet a shared catalogue/provider contract with complete observability across clients. #178–#180 must keep fallback deterministic and locale-neutral in core records while exposing the matched entry and terminal source fallback. |
| Availability reasons and labels | Schema-owned reason templates and presentation-label templates are extracted to POT; compiler and runtime tests cover reason IDs, arguments, source text, rendered text, and variant lookup. | **Partial.** #176 reconciles their schema/provenance shape; #179 extends typed placeholder validation; #178 preserves their PO entries. |

## First-party client matrix

| Client or boundary | Current path | Readiness and closure owner |
| --- | --- | --- |
| Compiler and core diagnostics | Compiler/core diagnostics expose structured `Diagnostic` fields: stable codes, severities, spans, related spans, and help. Runtime and trace outputs are separate structured records, not fields on `Diagnostic`. Neither needs Fluent to remain machine-readable. | **Baseline boundary.** #181 must preserve locale-neutral records while defining presentation at client edges. |
| CLI | `recite-cli/src/i18n/messages.rs` loads embedded Fluent resources, has a typed `MsgId` inventory, checks that the default resource contains every listed ID, and localises command/help/error presentation. JSON trace and other machine-oriented records remain structured. | **Partial.** #181 owns one shared resource contract, extraction/completeness, argument checks, and the launch-locale policy. |
| TUI | TUI rendering uses the same `Messages`/`MsgId` path for labels, prompts, help, transcript, and errors. | **Partial.** It is the strongest current client slice, but it is still CLI-owned rather than a resource contract shared with other clients; close under #181. |
| LSP | `recite-lsp/src/diagnostics.rs` maps core diagnostics to LSP codes, ranges, and messages; schema loading and summaries use the canonical model. No Fluent resource inventory or presentation resolver is present. | **Partial.** Keep semantic diagnostics shared and locale-neutral; add the shared UI-resource boundary in #181. |
| Editor extensions | The repository has no shipped VS Code/VSCodium, Neovim, or Zed extension resource inventory in this snapshot. Their future commands and presentation consume the LSP/kernel contracts. | **Gap for shared UI completeness.** #181 defines their resource ownership; editor implementation and parity are later #169 work. |
| Standalone GUI | No GUI workbench crate exists in this snapshot. The production spec requires a source-first GUI and editable PO path, but no client can yet consume a shared Fluent catalogue. | **Gap for client implementation.** #181 defines the contract; the workbench is later #170 after the GUI strategy gate. |
| Engine adapters | Adapter and conformance documents preserve structured dialogue, effects, source/schema freshness, and localisation boundaries; adapters do not own Recite UI text. | **Partial contract baseline.** #176/#177 cover producer schema inputs; concrete adapter refresh and client work remain separate. |

## Fixtures, diagnostics, and proof coverage

| Evidence layer | Existing fixtures/tests | Missing evidence required for Milestone 2 |
| --- | --- | --- |
| Schema shape and semantics | `fixtures/schema/valid/`, `fixtures/schema/invalid/`, `recite-core/tests/schema_manifest/**`, compiler schema-validation tests, and LSP schema-index tests cover valid/invalid JSON, spans, duplicates, types, domains, reasons, markup, and projection declarations. | #176: public-schema/`RawManifest` drift, typed provenance, producer-content freshness, malformed/semantic mismatch, and provenance-only fingerprint stability. |
| Standalone schema source | No schema TOML fixture or source-owned edit round-trip exists. | #177: versioned TOML covering every supported declaration, direct canonical lowering, source spans/version failures, byte-stable optional JSON export, and read-only generated output. |
| POT and source localisation | `recite-compiler/tests/pot_extraction.rs` and snapshots cover deterministic entries, context/comments/references, speaker/reason/label templates, and string escaping. | #178–#180: variant-bearing entries, interpolation, both plural source forms, translated markup, and shared fixture reuse across compiler/CLI/runtime. |
| PO/catalogue boundary | CLI dialogue-locale tests cover singular lookup, source fallback, locale truncation, malformed narrow syntax, conflicts between loaded catalogues, and singular placeholder mismatch. | #178: lossless round-trip, targeted edit, unknown/previous/fuzzy/obsolete/header/flag data, atomic write and conflict fixtures; #179/#180: translation and plural-arm validation. |
| Runtime delivery | Runtime localisation tests cover explicit locale/provider use, source-only sessions, line/choice/reason domains, variant priority, plural counts, source text retention, and structured availability reasons. CLI trace records the selected plural arm and both source forms. | #179/#180: ordinary substitution, plural lookup and counts, bounded gettext arm selection, fallback priority, and stable source/rendered records. |
| Fluent UI | `crates/recite-cli/i18n/en-US.ftl` is the complete embedded default inventory for the typed CLI/TUI IDs; `en-GB.ftl` is a smaller embedded resource and tests cover default completeness, formatting, and fallback. | #181: one inventory across CLI/TUI, LSP, editor, and GUI; extraction/unused/malformed/argument completeness gates; human-review boundary; `en-US` as the only launch support claim. |
| Diagnostics and clients | Core diagnostics are stable and LSP preserves codes/ranges/messages; CLI formats human-facing text through Fluent. There is no cross-client locale-neutrality/completeness fixture. | #181: prove UI locale changes do not change machine-readable diagnostics or trace/JSON, and that each client resolves the same resource IDs and arguments. |
| Adapter evidence | `fixtures/adapter-conformance/v1/` and `docs/engine-adapter-contract.md` cover structured adapter output, schema fingerprint fields, and capability-gated source/schema freshness scenarios. | #176/#177: generated manifest provenance/freshness and standalone producer fixtures; adapter implementation remains outside this audit. |

## Work routing and dependency boundary

The readiness gaps are intentionally split into six bounded follow-ups:

- [#176](https://github.com/plethu/recite/issues/176) reconciles the public
  JSON Schema, `RawManifest`, canonical lowering, typed provenance, producer
  fingerprints, and semantic-fingerprint exclusion rules.
- [#177](https://github.com/plethu/recite/issues/177) adds the standalone
  versioned TOML source that lowers directly to `ProjectSchema`, with optional
  deterministic read-only JSON export.
- [#178](https://github.com/plethu/recite/issues/178) adds the lossless PO
  document/edit layer and safe conflict-aware writes.
- [#179](https://github.com/plethu/recite/issues/179) completes named
  interpolation through parser, compiler, extraction, runtime, and clients.
- [#180](https://github.com/plethu/recite/issues/180) completes plural source,
  gettext, fallback, and runtime delivery.
- [#181](https://github.com/plethu/recite/issues/181) defines the shared Fluent
  UI resource contract and completeness gate; it does not translate dialogue
  or implement the GUI/editor clients.

These are distinct from the later Milestone 3 client-enabling work:

- [#167](https://github.com/plethu/recite/issues/167) owns project/user
  configuration, platform path resolution, locale preference precedence, and
  capability discovery. It consumes the schema/localisation contracts; it does
  not define schema semantics or catalogue formats.
- [#168](https://github.com/plethu/recite/issues/168) owns the UI-free shared
  authoring kernel and structured preview boundary, including source-preserving
  edits and producer-backed actions. It consumes the canonical model and
  localisation records; it does not become a second parser, schema authority,
  PO implementation, or Fluent catalogue.

Editor parity and GUI implementation follow their own issue gates after those
contracts exist. No readiness row above treats a future client implementation
as evidence that the underlying semantic or catalogue contract is complete.

## Milestone 2 exit evidence

The outcome is ready to close only when a representative project can compile,
validate, extract, load schema, check IDs and localisation, and produce
deterministic assets and diagnostics with all of the following visible in
fixtures and client-facing records:

- standalone versioned TOML lowers directly to `ProjectSchema`; generated JSON
  is optional, deterministic, producer-linked, and read-only;
- producer content freshness is independently checkable, while provenance-only
  changes leave the semantic schema fingerprint unchanged;
- POT/PO preserve stable IDs, source/context/comments, interpolation,
  variants, plural forms, markup, and headers without flattening source
  structure;
- runtime records the locale candidate chain, matched catalogue entry, and
  terminal source fallback;
- line, choice, reason, and label text preserve source and rendered values, and
  interpolation/plural diagnostics are structured and deterministic;
- shipped compiler, CLI/TUI, LSP, and adapter-facing surfaces use the canonical
  semantic model, with core diagnostics unchanged by UI locale; the shared
  Fluent UI contract is ready for future editor and GUI clients, whose adoption
  and evidence belong to #169/#170 and the later GUI milestone;
- `en-US` is complete and human-reviewed as the only supported launch UI
  locale; no machine-generated translation is counted as support.

See [production spec §9–10](recite-production-spec.md), [§13–15](recite-production-spec.md),
[§17–18](recite-production-spec.md), and [engine adapter contract §7](engine-adapter-contract.md)
for the normative details this matrix audits.
