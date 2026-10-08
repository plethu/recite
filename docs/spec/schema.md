# Schema

Part of the [production specification](../recite-production-spec.md). These are requirements;
implementation and release readiness require evidence from code, tests and the current GitHub
milestone. Section numbers remain stable.

## 10. Schema

### 10.1 Schema Scope

The schema must define:

- condition functions, including their parameter types and optional `returns` enum type for `:match`
  scrutinees;
- effect functions;
- effect modes;
- metadata keys;
- named metadata domains;
- inline markup tags;
- speaker IDs;
- optional actor registries;
- optional sound effect registries;
- optional cinematic cue registries;
- custom enum types;
- project-level content registries;
- presentation projection query functions;
- presentation projector definitions and label templates.

### 10.2 Schema Model and Producers

[`recite-core::schema`](../../crates/recite-core/src/schema/mod.rs) owns the canonical model and
validation. Producers own source declarations and export deterministic JSON manifests for compiler,
CLI and LSP consumption. Generated manifests are read-only derived artifacts. Dialogue validation
must not execute game code, reflect over host types or load engine resources.

Prefer native registrations for engine projects. Standalone projects use the source-owning TOML
producer below. Through the shared authoring kernel, Writer and editor actions edit that source or
open/regenerate through an engine producer, report stale output, and support structured failure and
retry. Unsupported producers remain explicitly read-only and do not count as schema editing. Writer
must expose the standalone path.

[Producer registration](../schema-producer-registration.md) owns explicit navigation and generation
actions; loading a registration executes nothing.
[Adapter contract §7](../engine-adapter-contract.md#7-schema-manifest-generation) owns host-resource
discovery, inclusion, provenance and freshness obligations. All producers must export the same
manifest and pass its validation suite.

#### 10.2.1 Standalone TOML source contract

The standalone source-owning producer uses a versioned, map-shaped TOML document. Its root
`schema_version` is the numeric marker `1`; named schema declarations use TOML tables keyed by
declaration name, while arrays are ordered only where the canonical model gives them semantic order.
A `[producer]` table with a non-empty `id` is mandatory. Standalone TOML has the fixed producer kind
`standalone` (an explicit `kind` is validated, not chosen by the author). For availability-reason
arguments, TOML uses explicit tagged values, for example `{ kind = "binding", name = "actor" }` or
`{ kind = "literal", value = "calm" }`; the JSON `$name` shorthand is not a TOML form. For
availability-reason mapping literals, generated JSON escapes a leading `$` by doubling that
character (`$name` remains a binding and `$$name` is a literal `$name`); other JSON literal fields
retain their existing exact string semantics. This is a format boundary, not a tagged JSON form.

TOML duplicate keys or tables are syntax errors from the TOML frontend. Declaration/table order is
nonsemantic. The source owner retains the CST and source spans: an unedited `source_text()` is the
exact input, including line endings and final-newline policy, and typed edits preserve that policy
and untouched trivia. TOML lowers directly through the canonical manifest raw and validation path;
it is not converted through JSON or a second mutable schema model. Generated JSON is deterministic,
read-only, retains provenance and producer freshness metadata, and is accepted by the existing
manifest loader. The source fingerprint includes producer identity and source-owned semantic content
while ignoring trivia, map order, generated fields, and diagnostic-only provenance; the semantic
schema fingerprint remains distinct and provenance safe. Producer IDs and fingerprints are the
linkage used for stale-output comparisons.

A manifest becomes canonical after parsing into the typed model and sorting map-like collections for
deterministic fingerprints and diagnostics.

Recite should publish a JSON Schema for the generated manifest format. That JSON Schema validates
manifest document shape only: required fields, allowed keys, scalar types, array/object structure,
effect mode strings, and basic version compatibility. It is a useful public contract for adapter
authors, CI checks, editor IntelliSense, and people inspecting generated manifests.

The JSON Schema and manifest loader must classify provenance and producer metadata consistently with
adapter contract §7. Optional fields such as domain origins, value origins, context origins,
producer fingerprints, schema export versions, and inclusion policies must be accepted only in their
documented shapes. The loader must either preserve them for diagnostics, hovers, and stale-schema
tooling or explicitly ignore non-canonical producer metadata; it must not accidentally treat
diagnostic-only metadata as semantic validation input.

Origins are structured objects containing `kind` and `id`, with an optional `label`; the loader
rejects legacy strings. Namespaced producer-origin extension fields remain diagnostic-only JSON
values and never affect the semantic schema fingerprint. Generated contextual domains must carry
their resolved `missing_context` policy explicitly; a source-owning producer may resolve an omitted
authoring option to `diagnostic` before exporting JSON.

JSON Schema is not the authority for Recite semantics. After document-shape validation, Recite must
lower the manifest into the canonical Rust model and run semantic validation there. Semantic
validation owns duplicate definitions, unknown type references, registry/value checks, condition
return compatibility, effect arity/type checks, metadata target policy, markup policy, projection
query function references, projector input/output references, presentation label placeholders,
diagnostics, and deterministic fingerprinting.

The public types and their fields are documented in `recite-core::schema`. All schema ingress,
including native callers constructing a `ProjectSchema`, must use the same integrity validation;
acceptance must not depend on whether the input arrived as JSON, TOML or Rust values.

Metadata domains are named schema definitions. Metadata definitions reference domains by name rather
than hardcoding special keys such as `portrait`.

`symbol` is a metadata schema scalar, not a new runtime value kind. A metadata definition with
`"type": "symbol"` accepts source `SourceMetadataValue::Scalar(SourceMetadataScalar::Symbol(_))`
values, rejects quoted string literals unless a different metadata type permits them, and lowers the
accepted symbol into the compiled/runtime metadata value model as a string-like value with
schema-validated domain semantics. Runtime consumers must use the metadata key and schema contract
to interpret that value; they must not depend on source spelling.

Domain kinds:

- flat domains declare a deterministic set of valid symbol values;
- contextual domains select the valid symbol values from another source item field or metadata key.

V1 contextual selector scope is deliberately small:

- `field:speaker` resolves the line speaker first, then the inherited block default speaker;
- `metadata:<key>` resolves metadata with `<key>` on the same source item. It succeeds only when
  that key appears exactly once on the item and the value is a scalar symbol after source-value
  lowering. An absent key follows the domain's missing-context policy. Repeated keys, arrays, quoted
  strings, and non-symbol scalar values are selector-shape diagnostics because they would make
  compiler and LSP resolution ambiguous.

Block-wide and project-wide selectors are deferred until a concrete implementation issue needs them.

Contextual domains must declare a missing-context policy. The default is `diagnostic`, which reports
that the selector could not be resolved. Other allowed policy values are `empty`, which produces no
valid values or completions, and `fallback`, which falls back to a named flat domain declared in the
same `missing_context` object. Fallback targets must be flat domains so diagnostics and completions
remain deterministic.

Compiler validation, CLI validation, and LSP completions/diagnostics must consume the same
manifest-backed metadata domain rules. The compiler is the authority for acceptance; LSP behavior is
a live authoring projection of the same domain resolution.

Registry-backed values are exported as self-contained snapshots, with optional origins and input
fingerprints for the provenance and freshness checks in adapter contract §7.

#### 10.2.3 Availability Reason Definitions

Availability reasons are schema-owned reusable templates for explaining visible-but-unavailable
choices. They give adapters, CLI/TUI, LSP, tests, and localisation tools structured data without
requiring the core runtime to invent prose.

Rules:

- `availability_reasons` is a schema-level map keyed by stable reason ID.
- Each reason declares localisable source template text and typed parameters.
- Generated schema manifests must include enough reason-template data, parameter types, and
  provenance for compiler, LSP, CLI/TUI, runtime, and adapter tooling to validate and present
  reasons without executing game code.
- Template text is dialogue/project content, not Recite-owned UI text. It follows the dialogue
  localisation path, not the shared Recite UI Fluent resource contract used by CLI/TUI, GUI, LSP,
  and editor extensions.
- Boolean condition definitions may declare an `availability_reason` mapping. Mapping values bind
  reason parameters from condition arguments using `$<condition_param>` references or literal values
  valid for the target parameter type. In generated JSON, a literal leading dollar is escaped as
  `$$`; TOML uses the explicit `literal` tag instead.
- The compiler validates that condition reason mappings reference existing reason IDs, bind every
  required reason parameter exactly once, do not bind unknown parameters, and produce values
  compatible with the reason parameter types.
- A choice-level `reason=<id>` primary reason override must reference an existing parameterless
  availability reason in v1. Referencing a parameterised reason is a compiler diagnostic until an
  explicit binding syntax is designed; the compiler must not guess bindings from condition
  arguments.
- Negated conditions and compound expressions do not synthesize new reason prose. They may carry
  leaf reasons for positive condition calls where the boolean grouping preserves meaning, or no leaf
  reason where the schema cannot explain the failure safely.

Example choice and schema pairing:

```text
? ask_news@c2bdeae1465bfa65bcf4 requires=(trust_gte(innkeeper, player, 3)) reason=innkeeper_trust_hint
  What's the real news?
  -> local_news_private
```

The primary reason override above uses the reusable `innkeeper_trust_hint` template instead of
repeating prose on every choice. The compiler may still preserve any schema-derived detailed reason
tree for trace and adapter output.

The [full manifest fixture](../../fixtures/schema/valid/full_manifest.json) is an executable example
of the generated format. Host setup and schema-producing APIs belong in each adapter's guide.

#### 10.2.4 Presentation Projection

Projection declarations describe pure presentation over runtime output. They are inspectable schema
data; validation does not execute host queries. The canonical types live in
[`recite-core::schema`](../../crates/recite-core/src/schema/mod.rs).

Selectors address an event, metadata key/set on a declared target, or an availability reason. Inputs
bind stable candidate identities, ordered metadata occurrences, reason arguments or literals. A
candidate-relative input must apply to its selector: a choice ID cannot bind a line candidate.
Project identity requires a declared stable project/content-set ID.

Repeated metadata is explicit: `Only` requires exactly one value; `First`, `Last` and `Index` select
in source order; `All` produces an array and requires an array-compatible input type. Validation
checks metadata targets and domains, query names, argument/result types, input references, output
fields and template bindings. Query functions are schema-global declarations separate from condition
functions; calls take their return type from the declaration.

Each projector and output has a stable ID. Affordance identity derives from projector ID, output ID,
target identity and relevant metadata occurrence, never an address, counter or display label.
Projectors, queries and outputs are ordered by their canonical sorted IDs. Query-result references
may name only earlier queries in that order; forward dependencies are invalid. Within an event,
candidate order is event, prompt container, prompt line, choices in runtime order, effect request,
current block, then project. Repeated values retain authored metadata order. Query results retain
request order even if a host batches or caches equivalent queries.

Labels have stable extraction IDs, source templates and named typed bindings to declared inputs or
query results. Translations preserve those placeholders. Structured output retains target, kind,
slot, provenance and bound fields alongside the localized label; a rendered string alone is not a
portable projection result.

V1 does not require a core projection executor. A host that implements projection must document when
queries run, how displayed projections refresh and how structured failures surface. Work is bounded
by the event, reachable compiled metadata and declared projectors; resource discovery belongs to
schema export. Projection cannot alter runtime text, choice order or availability, IDs, targets,
effects or session state; execute game mutations or random rolls; or become a save/load dependency.
The same event and host context produce the same affordances. Refreshing labels after host state
changes does not reevaluate a frozen prompt's availability.

### 10.3 Validation Reporting

Schema validation must report all violations in one run where possible.

Diagnostics must include:

- file path;
- line;
- column;
- severity;
- code;
- message;
- optional fix suggestion.

Schema validation should use the same `Diagnostic` model as parser and compiler validation. The
compiler should expose shared diagnostic factories or a shared diagnostic catalog for schema-related
checks so CLI, LSP, and test fixtures use the same stable codes and messages.

Source-backed diagnostics must point at the smallest useful value-specific span available:

- condition function names;
- condition arguments;
- effect names;
- effect modes;
- effect arguments;
- metadata keys;
- metadata values;
- projection projector IDs;
- projection query function names;
- projection query arguments;
- projection output IDs;
- projection label template IDs and placeholders;
- inline markup tag names;
- speaker IDs;
- registry references and registry values;
- schema manifest fields when the manifest itself is malformed.

When a schema manifest is generated from adapter code, the manifest may include producer origin
metadata for definitions, metadata domains, metadata-domain contexts, metadata-domain values, and
registry values. Recite diagnostics may surface that origin as related context, but dialogue-source
diagnostics must remain valid even when producer origins are unavailable.
