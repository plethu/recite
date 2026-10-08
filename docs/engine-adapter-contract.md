# Recite Engine Adapter Contract

This document defines the host-agnostic contract that Godot, Bevy, Unity, and future Recite engine
adapters must preserve. It is normative unless a section is explicitly marked as illustrative.

This contract is independent of the host API shape. The `recite-adapter` crate now owns the shared
immutable asset, session driver, error categories, and owned catalogue used by the Rust engine
companions and the C ABI wrapper.

## 1. Contract Goals

Adapters integrate compiled dialogue with host assets, callbacks and structured events. They
preserve the [runtime contract](spec/runtime-localisation.md#8-runtime), stable IDs and effect
semantics; game mutations remain outside traversal. An adapter cannot weaken validation or require
callers to parse prose. Host-specific setup belongs in the package guides linked in §14.

## 2. Compiled Asset Identity and Freshness

This section covers two distinct questions that adapters must not conflate:

- **Compatibility identity** — "is this the same compiled asset a saved session was created
  against?" — used for save/load resume (see §9 and spec §8.6).
- **Freshness** — "is this compiled asset stale relative to the source and schema on disk?" — used
  for authoring import and diagnostics (spec §12.3).

A compiled Recite asset must have a stable **compatibility identity** that can be stored in session
state and compared during save/load. The identity should include enough information to distinguish
incompatible compiled assets, such as a project asset ID, a compiled-asset version or fingerprint,
the schema fingerprint, and the compiler compatibility version. This identity answers resume
compatibility, not staleness.

**Freshness** is a separate, content-based comparison. Per spec §12.3 it is computed over current
source fingerprints, the current schema fingerprint, and the current compiler compatibility version,
compared against the values embedded in the compiled asset. Adapters must not substitute the single
compatibility fingerprint for this source-level freshness comparison.

Locale catalogs are not part of compiled asset identity unless an adapter explicitly bundles them
into the host asset. If catalogs are bundled, the adapter should track catalog identity as adapter
content-bundle identity, not as runtime compiled-asset compatibility.

Adapters must validate compiled asset compatibility before starting or resuming a session. Loading
or decoding failures must surface as structured adapter errors. Stale or schema-incompatible assets
must not start a session as if they were current.

Adapters should reuse the same freshness semantics as the CLI `compile` and `check-fresh` surface
(spec §12.3) where possible. A host asset importer may cache engine-native resources, but the cache
must not hide stale compiled content from session start, resume, or diagnostics when source and
schema inputs are available.

## 3. Session Ownership

When the host owner exits, the adapter must deterministically release its active session, callback
registrations, native buffers and host subscriptions.

The v1 adapter contract supports one active dialogue session per declared adapter owner. Each
adapter must document whether that owner is a singleton service/resource, node, component, scene
service, or equivalent host-native object.

Starting a second session on the same owner while one is active must return or emit a structured
error. It must not panic, drop the previous session, overwrite session state, or implicitly end the
active session.

Future adapters may support multiple sessions keyed by entity, scene, or session ID. That extension
must preserve the same start/select/ack semantics per session and must not change the single-session
v1 contract.

## 4. Runtime Operations

Every adapter must expose host-native equivalents of these operations:

- start a session from a compiled asset, optional start block, and optional locale; an absent locale
  selects source-text-only mode;
- accept caller-owned typed interpolation values for line and choice bindings; values are separate
  from serialised session state and must be supplied again when a restored session needs them;
- select a prompt choice by `ChoiceId`;
- acknowledge a pending blocking effect by `EffectRequestId` and `EffectAck`;
- end or dispose the active session through an explicit host-visible operation;
- snapshot and restore Recite session state for game save/load integration.

After `start`, `select`, or `acknowledge`, the adapter must either expose the core `next`/advance
operation directly or drain traversal synchronously until the next host-observable boundary: line
output, prompt, immediate effect, blocking effect, end, or structured error. The adapter must
document which shape it uses. If it drains synchronously, the returned or emitted output batch must
preserve runtime order and must stop at a prompt or blocking effect.

The shared Bevy, Godot and FFI driver limits each operation to 10,000 output events. Exceeding that
bound produces a dialogue fault and rolls back the operation, including accumulated lines and
immediate effects. This bounds loops that escape the runtime's per-advance step limit. FFI hosts can
prepare a new or restored session, install conditions and locale configuration, then begin
traversal; a failed begin leaves the prepared session available for correction.

Selection by index may be exposed as an engine convenience, but it must lower to the stable
`ChoiceId` from the current prompt. Selecting an unavailable, unknown, or stale choice must produce
a structured error.

Acknowledging an effect must require the exact pending `EffectRequestId`. The adapter must reject
acknowledgements when no blocking effect is pending or when the ID does not match the pending
effect.

Adapters must expose both acknowledgement outcomes from spec §7.4 (`EffectAck::Completed` and
`EffectAck::Failed { reason }`), not only the success path. A host that cannot complete a blocking
effect must have a contract-blessed way to report failure back into traversal.

Selecting a choice may emit an echoed line (`ChoiceEchoMode`, spec §8.5) as the first output after
`select`. Adapters must treat this as ordinary line output in the drained batch or `next` sequence,
not as unexpected content.

## 5. Structured Output

Expose the [runtime event model](spec/runtime-localisation.md#83-event-model) through native
signals, events, messages or return values. Preserve IDs, order, metadata, markup, effect modes and
structured errors. Prompts retain optional line content and full choice availability, including
primary reasons, `all`/`any` groups, bound arguments and origins. A compact display reason does not
replace the structured tree, and rejected selections leave traversal unchanged.

Plural output preserves authored singular/plural forms, count, selected arm and lookup provenance.
`DialogueLine.source_text` is the selected decoded source form; authored plural forms are not
localized templates. Catalogue templates belong in trace/debug output, not normal prose.

Optional [presentation projection](spec/schema.md#1024-presentation-projection) adds structured
affordances without changing runtime output or session state. Adapters implementing it document
query timing, refresh and error handling and preserve the canonical declaration and output data for
tooling and conformance. Host rendering alone is not a portable projection result.

## 6. Conditions

Conditions are pure host queries. Adapters must register condition handlers through host-native
extension points and evaluate them through caller-provided game context.

Condition handlers must not mutate game state, advance time, emit effects, or depend on
nondeterministic ordering. The adapter must surface a structured error when:

- no handler can be found (`missing_condition_handler_error`);
- a handler receives invalid arguments or fails during evaluation (`condition_evaluation_error`);
- a handler returns a value outside the declared schema type (`invalid_condition_result_error`).

Schema-generated or hand-written typed condition bindings are allowed, but they must lower into the
same canonical schema manifest used by the compiler, CLI, LSP, and runtime integration.

## 7. Schema Manifest Generation

[Schema §10.2](spec/schema.md#102-schema-model-and-producers) owns the canonical model, generated
manifest and source-editing boundary. Adapters provide native authoring surfaces and the
host-specific export obligations below. [Producer registration](schema-producer-registration.md)
connects their explicit navigation and generation actions to Writer.

### 7.1 Producer Responsibilities

Producers own host-resource discovery, typed registrations, inclusion rules, provenance and stale
checks. They export a self-contained snapshot accepted by the canonical validator. Host handlers and
compiled dialogue must agree on the exported declarations; adapters cannot maintain a second schema
truth.

### 7.2 Metadata-Domain Export Shape

[Schema §10.2](spec/schema.md#102-schema-model-and-producers) owns metadata-domain fields,
selectors, missing-context policies, origins and producer fingerprints. Export domains by symbolic
name and reference them from metadata definitions; do not embed presentation keys or duplicate a
fallback domain's values. Host provenance stays diagnostic: it cannot change compiler acceptance.

### 7.3 Deterministic Snapshots and Fingerprints

The same host state and producer configuration produce the same canonical schema and fingerprint.
Symbols must not depend on addresses, transient import IDs, localized labels, filesystem order, time
or editor sessions. Canonical map ordering and ordered semantic collections belong to
[schema §10.2](spec/schema.md#102-schema-model-and-producers).

The semantic fingerprint covers validation and runtime-relevant declarations, including domains,
availability reasons, mappings, projection queries and labels. Diagnostic origins and producer
metadata remain outside it. A manifest never hashes its own recorded fingerprint. Producer input
freshness is a separate comparison channel.

### 7.4 Provenance and Diagnostics

When the host can provide provenance, generated manifests should include `origin`,
`context_origins`, and `value_origins` for domains, contexts, and values. Origins may name a
resource path, asset GUID, asset database key, script/type/member, data-table row, import source, or
other stable host identifier. Producers should also include fingerprints for input sets when the
host can compute them cheaply and repeatably.

Origins and fingerprints are for diagnostics, LSP hovers, stale-schema checks, and adapter
troubleshooting. Dialogue diagnostics must still work when origin metadata is absent. A missing
origin must not make a valid dialogue invalid or make an invalid dialogue valid.

### 7.5 Stale-Schema Checks

Producers expose an explicit regeneration or stale-check action. Re-exporting current inputs and
comparing canonical schema fingerprints establishes whether their semantics changed. Typed producer
and content fingerprints may also detect stale inputs; they do not replace the semantic fingerprint.

`recite check-schema-producer-freshness --expected OLD.json --actual NEW.json` compares exported
fingerprints without inspecting host resources. Detailed results distinguish manifest content,
producer inputs, registry inputs and metadata-domain inputs, including missing, duplicate,
unexpected and mismatched records. A digest in one scope cannot stand in for another.

Document weaker checks when host fingerprints are unavailable. Never conceal staleness by using
editor state that compiler and LSP cannot reproduce. Distinguish source errors, malformed manifests
and stale producer output in diagnostics.

### 7.6 Host-Agnostic Example

The [full manifest fixture](../fixtures/schema/valid/full_manifest.json) is a tested example of the
canonical format. Producer identity and fingerprint rules are defined above and in
[schema §10.2](spec/schema.md#102-schema-model-and-producers).

### 7.7 Engine Notes

Engine-specific producers and resource identities are described in the
[package guides](#14-per-engine-guidance). Each exports ordinary Recite symbols and a canonical
manifest, regardless of its discovery API.

## 8. Effects

Preserve [effect modes and order](spec/conditions-effects.md#7-effects): deferred requests collect
until the ending, immediate requests yield during traversal, and blocking requests require the
matching acknowledgement. Typed host wrappers may improve ergonomics but cannot hide the original
structured request or move game-side mutation into traversal.

## 9. Save and Load Handoff

Round-trip the runtime snapshot as one opaque unit. Hand-picking or reconstructing fields loses
identity, history or pending work needed for deterministic resume. Host game state stays separate.
Restore validates against the supplied compiled asset; stable line IDs alone do not establish
compatibility.

A restored blocking request retains its ID. The game decides whether its operation already happened,
should replay or should fast-forward, then acknowledges that same request. The runtime cannot
establish whether an external effect occurred before saving.

## 10. Localisation

The adapter's locale provider/catalog, interpolation values, and grammatical variant are
adapter-owned inputs and are not part of the runtime snapshot. A restore operation must re-supply
those inputs before traversal resumes; the snapshot preserves the runtime locale so the same lookup
context can be reconstructed.

Adapters must start sessions with an explicit locale, a stable project-configured locale, or
source-text fallback. Adapters must not silently derive the dialogue locale from the OS, editor, or
engine environment; those inputs may be used only when the project or author explicitly opts into
that policy. When no locale is selected, the adapter must preserve source-text-only mode and bypass
locale-provider lookup. Locale fallback must be deterministic and must preserve the same localized
text, source text, line IDs, choice IDs, metadata, and markup that the runtime exposes.

Adapters must expose grammatical variant selection (spec §9.5) as an explicit, caller-driven choice
— a session-level setter or a per-operation override that threads into traversal. The runtime never
infers a variant, so adapters must not derive it from host environment or locale; lookup priority
remains `id&variant` → `id` → source text, and resolution must stay deterministic for a given `(id,
source, locale, variant, count)` tuple. The resolved text exposed in §5 output reflects the selected
variant.

Interpolation values use the runtime's typed scalar model (`string`, `int`, `float`, and `bool`) and
are resolved through the same binding names as the core runtime. Adapters must not stringify an
absent or mismatched value, and must project the runtime's structured localisation error. Values may
be replaced between traversal operations when host state changes; they are never implicitly captured
in save data.

Changing locale for an active session is not part of the v1 contract unless an adapter documents and
tests the exact behavior. Restarting the session with a new locale is always acceptable. Variant
selection, by contrast, may change mid-session because it is an explicit per-lookup axis, not a
session-rebuild.

Missing translations may use the runtime/compiler documented deterministic fallback path. Malformed
catalogs must surface a structured loading or localisation error or diagnostic. Silent host-specific
fallback chains are not acceptable.

## 11. Changed Compiled Assets

Every adapter must choose, document, and test one of these changed-asset policies for v1:

- `reject_refresh_until_session_ends`: if a compiled asset changes while a session is active, the
  adapter rejects the import or refresh attempt until the active session ends.
- `reload_for_next_session_only`: the adapter accepts the new compiled asset into the host asset
  cache, but the active session continues using its original compiled asset identity. The next
  session uses the new asset.
- `restart_required`: the adapter reports that the active session must be ended and restarted before
  the new compiled asset can be used.

Silent mid-session asset mutation is forbidden. It can break deterministic traversal, save/load
identity, previous prompt choice validation, pending blocking-effect acknowledgement, and
replay/test traces.

Adapters may explore richer mid-session patch reload after v1, but that feature requires a separate
design covering identity migration, pending prompts, blocking effects, save/load, localization, and
deterministic replay.

## 12. Error Categories

Adapters must surface structured errors with stable machine categories. Host-specific error text may
be added for diagnostics, but callers must be able to match the category without parsing prose.

The
[operation/result schema](../fixtures/adapter-conformance/v1/adapter-conformance-operation-result-v1.schema.json)
owns the exhaustive category enum. Shared Rust classification and host conformance tests enforce its
mapping; documentation does not duplicate that table.

Adapters should preserve source-backed diagnostics from the compiler and should include host asset
paths or resource identifiers when available.

Projection error categories are capability-gated: adapters that do not expose presentation
projection do not emit them. `missing_projection_handler_error` means a declared projection query
function has no host handler. `projection_evaluation_error` means a handler failed while evaluating
a projection query. `invalid_projection_result_error` means a handler returned a value outside the
declared projection query function return type.

## 13. Adapter Conformance Fixtures

The shared host-agnostic conformance artifacts live under `fixtures/adapter-conformance/v1/`:

- `scenarios.json` (versioned scenario manifest);
- `adapter-conformance-manifest-v1.schema.json` (manifest schema);
- `adapter-conformance-operation-result-v1.schema.json` (stable operation/result schema).

These files are public adapter-consumable fixtures, not private Rust-only test helpers.

### 13.1 Source Fixture Boundary

The [fixture placement rules](../fixtures/adapter-conformance/README.md#source-fixture-rule) keep
shared `.recite` sources in the main corpus and adapter operation/capability expectations in the
conformance manifests.

### 13.2 `AdapterConformanceDriver` Operation Contract

The published operation/result schema defines the host test surface. Results are structured; callers
must not parse prose to determine success. The
[fixture observation rules](../fixtures/adapter-conformance/README.md#observation-modes) explain how
individual reference advances relate to transactional host batches, invariant-only checks and
unavailable capabilities.

Godot conversions require initialized engine bindings. Compile checks and reference/FFI tests do not
exercise `VarDictionary` or other Godot-native values; use the Godot-hosted runner.

### 13.3 Stable Category Table and Drift Checks

Reference-driver checks must reject drift between the §12 operation/result category enum, the
manifest schema and the scenario manifest.

### 13.4 Scenario Requirements

Each failure scenario names one expected stable error category, or an allowed set where this
contract explicitly permits alternatives. Success-only scenarios may omit `expected_error` and must
contain no error step.

The published scenarios must cover every category from §12. Projection cases apply only to adapters
exposing that capability; source/schema freshness cases require declared import visibility. Compiled
compatibility and save/load identity are mandatory. Host-dependent cases remain in the manifest as
`adapter_runner_required`, with the same operation/result shape and an explicit runner note.
Changed-asset cases must cover each declared policy, including successful next-session reload.

### 13.5 Required Host-Independent Coverage

Each adapter must run its applicable §13.4 scenarios through its own surface, asserting rollback,
structured fields and event order. Reference-runtime success does not establish host execution.

Adapters exposing projection must also test successful candidate order, repeated metadata, stable
affordance IDs and preservation of template, source, localized and structured label fields; the
published failure scenarios alone do not cover those behaviors.

Host tests may differ in mechanics, but their expected Recite observations remain
engine-independent. Conformance covers semantics; performance evidence must also meet spec §19.6,
including negligible cost when no session is active.

## 14. Per-Engine Guidance

Godot, Bevy and Unity are the v1 adapter targets. All three must meet this contract and the
[release gates](spec/release.md#23-acceptance-criteria-for-a-serious-v1). Their package READMEs own
native API, setup, upgrade and tested-support details:

- [Godot](../addons/recite/README.md): Resources, dialogue Nodes, callables and signals.
- [Bevy](../crates/recite-bevy/README.md): assets, ordered requests, output messages and game
  systems.
- [Unity](../Packages/com.recite.dialogue/README.md): managed service and GameObject runner over the
  shared native core. An optional DOTS facade must preserve the same semantics and declare its own
  tested support; it is not required by the base package.

Examples or documented tests must demonstrate load/start/select/end, pure conditions, all effect
modes, prompt and blocking-effect save/load, structured errors and the source/build/import/refresh
loop. One small sample plus focused tests can supply that evidence; separate sample projects are not
required for every operation. Unreal and GameMaker remain post-v1 evaluation targets.

## 15. Shared Crate Boundary

`recite-adapter` owns reusable host-independent integration behaviour: validated immutable loaded
assets, one-owner session lifecycle, transactional traversal batches, stable error categories, and
owned dialogue catalogue lookup. Engine companions still own asset import, callbacks, native events,
and UI projection. `recite-runtime` remains the sole authority for traversal and snapshot bytes.
