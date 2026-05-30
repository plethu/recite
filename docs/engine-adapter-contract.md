# Recite Engine Adapter Contract

This document defines the host-agnostic contract that Godot, Bevy, Unity, and
future Recite engine adapters must preserve. It is normative unless a section is
explicitly marked as illustrative.

This is a contract document, not an implementation plan. It does not add a
public Rust API, generated bindings, a shared `recite-adapter` crate, or a new
dependency. Adapters may call `recite-core` and `recite-runtime` directly until
real adapter MVPs prove which helper types deserve to be shared.

## 1. Contract Goals

Adapters exist to make Recite feel native in a host engine while preserving the
same dialogue semantics everywhere.

Every adapter must:

- load compiled Recite assets through the host asset system where practical;
- preserve deterministic runtime traversal;
- preserve choice selection by stable `ChoiceId`;
- preserve blocking-effect acknowledgement by stable `EffectRequestId`;
- emit dialogue output, choices, effects, endings, and errors as structured
  host-visible values;
- keep condition evaluation outside the runtime as pure host queries;
- emit effect requests without executing game-side mutation in the runtime;
- serialize and restore Recite session state without serializing game state;
- define and test its changed-asset behavior;
- document its authoring import or refresh loop.

Adapters must not:

- mutate an active runtime session by silently swapping its compiled asset;
- execute game-side effects from inside `recite-runtime`;
- require dialogue source to call directly into engine scripts;
- depend on prose parsing for lines, choices, metadata, effects, or errors;
- weaken schema validation to match a host engine convenience API.

## 2. Compiled Asset Identity and Freshness

This section covers two distinct questions that adapters must not conflate:

- **Compatibility identity** — "is this the same compiled asset a saved session
  was created against?" — used for save/load resume (see §9 and spec §8.6).
- **Freshness** — "is this compiled asset stale relative to the source and
  schema on disk?" — used for authoring import and diagnostics (spec §12.3).

A compiled Recite asset must have a stable **compatibility identity** that can
be stored in session state and compared during save/load. The identity should
include enough information to distinguish incompatible compiled assets, such as
a project asset ID, a compiled-asset version or fingerprint, the schema
fingerprint, and the compiler compatibility version. This identity answers
resume compatibility, not staleness.

**Freshness** is a separate, content-based comparison. Per spec §12.3 it is
computed over current source fingerprints, the current schema fingerprint, and
the current compiler compatibility version, compared against the values embedded
in the compiled asset. Adapters must not substitute the single compatibility
fingerprint for this source-level freshness comparison.

Locale catalogs are not part of compiled asset identity unless an adapter
explicitly bundles them into the host asset. If catalogs are bundled, the
adapter should track catalog identity as adapter content-bundle identity, not as
runtime compiled-asset compatibility.

Adapters must validate compiled asset compatibility before starting or resuming
a session. Loading or decoding failures must surface as structured adapter
errors. Stale or schema-incompatible assets must not start a session as if they
were current.

Adapters should reuse the same freshness semantics as the CLI `compile` and
`check-fresh` surface (spec §12.3) where possible. A host asset importer may
cache engine-native resources, but the cache must not hide stale compiled
content from session start, resume, or diagnostics when source and schema inputs
are available.

## 3. Session Ownership

The v1 adapter contract supports one active dialogue session per declared
adapter owner. Each adapter must document whether that owner is a singleton
service/resource, node, component, scene service, or equivalent host-native
object.

Starting a second session on the same owner while one is active must return or
emit a structured error. It must not panic, drop the previous session, overwrite
session state, or implicitly end the active session.

Future adapters may support multiple sessions keyed by entity, scene, or
session ID. That extension must preserve the same start/select/ack semantics per
session and must not change the single-session v1 contract.

## 4. Runtime Operations

Every adapter must expose host-native equivalents of these operations:

- start a session from a compiled asset, optional start block, and locale;
- select a prompt choice by `ChoiceId`;
- acknowledge a pending blocking effect by `EffectRequestId` and `EffectAck`;
- end or dispose the active session through an explicit host-visible operation;
- snapshot and restore Recite session state for game save/load integration.

After `start`, `select`, or `acknowledge`, the adapter must either expose the
core `next`/advance operation directly or drain traversal synchronously until
the next host-observable boundary: line output, prompt, immediate effect,
blocking effect, end, or structured error. The adapter must document which
shape it uses. If it drains synchronously, the returned or emitted output batch
must preserve runtime order and must stop at a prompt or blocking effect.

Selection by index may be exposed as an engine convenience, but it must lower to
the stable `ChoiceId` from the current prompt. Selecting an unavailable,
unknown, or stale choice must produce a structured error.

Acknowledging an effect must require the exact pending `EffectRequestId`. The
adapter must reject acknowledgements when no blocking effect is pending or when
the ID does not match the pending effect.

Adapters must expose both acknowledgement outcomes from spec §7.4
(`EffectAck::Completed` and `EffectAck::Failed { reason }`), not only the success
path. A host that cannot complete a blocking effect must have a contract-blessed
way to report failure back into traversal.

Selecting a choice may emit an echoed line (`ChoiceEchoMode`, spec §8.5) as the
first output after `select`. Adapters must treat this as ordinary line output in
the drained batch or `next` sequence, not as unexpected content.

## 5. Structured Output

Adapters must surface runtime output as structured values, not host-formatted
strings. The host-visible shape must include equivalents for:

- line output with line ID, speaker, localized text, source text where useful,
  metadata, markup, and pending deferred effects;
- prompt output with optional line content and a list of structured choices,
  where each choice preserves its `ChoiceId`, localized text, source text,
  metadata, availability state, and unavailable reason (spec §8.5) so hosts can
  present and disable choices and so the §4 unavailable-choice error is
  satisfiable from emitted data alone;
- effect request output with effect request ID, effect name, mode, arguments,
  and source/debug identity where available;
- end output with deferred effects;
- structured errors.

Adapters may map those values to signals, events, messages, resources,
callbacks, or service responses. The mapping must preserve Recite IDs, effect
modes, line/choice metadata, locale, and error categories. Inline markup must be
preserved as part of runtime text/source text; adapters may add a later
presentation layer that interprets markup, but that layer is outside the core
adapter contract.

## 6. Conditions

Conditions are pure host queries. Adapters must register condition handlers
through host-native extension points and evaluate them through caller-provided
game context.

Condition handlers must not mutate game state, advance time, emit effects, or
depend on nondeterministic ordering. The adapter must surface a structured error
when:

- no handler can be found (`missing_condition_handler_error`);
- a handler receives invalid arguments or fails during evaluation
  (`condition_evaluation_error`);
- a handler returns a value outside the declared schema type
  (`invalid_condition_result_error`).

Schema-generated or hand-written typed condition bindings are allowed, but they
must lower into the same canonical schema manifest used by the compiler, CLI,
LSP, and runtime integration.

## 7. Schema Manifest Generation

Adapters should let game projects produce Recite schema manifests from typed
host code where practical. The host-specific authoring surface may be a Rust
builder or derive, Godot C#/GDScript registration, Unity C# attributes or
builders, editor-imported assets, data tables, or another native mechanism.

All producer surfaces must lower into the canonical Recite schema model and
generated manifest. The compiler, CLI, LSP, and adapter runtime integration
must agree on condition names, effect names, parameter types, enum variants,
registries, metadata keys, metadata domains, and documented handler
requirements.

Adapters must not introduce a second, host-only schema truth that can drift
from compiled dialogue validation. The generated manifest is the boundary: game
or adapter code may produce it, but Recite compiler, CLI, LSP, and runtime code
only consume it.

### 7.1 Producer Responsibilities

Schema producers are responsible for host discovery. A producer may be part of
an engine adapter or may be a standalone project tool, but it owns:

- scanning host resource directories, content folders, asset databases, and
  import metadata;
- reading typed registries, editor assets, data tables, or reflected host code;
- applying host-specific inclusion and exclusion rules;
- resolving resource-backed enum, registry, and metadata-domain values into a
  self-contained manifest snapshot;
- checking whether the previously generated manifest is stale relative to the
  host state it claims to represent.

Recite core validation must not scan engine resources, query an asset database,
load editor-only data, reflect over game code, or execute game code to validate
dialogue. Normal compiler, CLI, LSP, and runtime flows consume only the
generated manifest plus dialogue/project inputs.

### 7.2 Metadata-Domain Export Shape

The manifest must represent metadata domains using the schema model from spec
§10.2. Adapters and standalone producers must export domains by symbolic domain
name, not by hardcoded presentation keys. A metadata definition references a
domain by name; the key using that domain remains project schema data.

Flat domains must include:

- `kind: "flat"`;
- a deterministic list of symbol `values`;
- optional domain-level and value-level origin metadata when available;
- optional producer fingerprints for the host inputs used to create the domain.

Contextual domains must include:

- `kind: "contextual"`;
- a contextual `selector`, using the v1 selector forms defined by spec §10.2;
- deterministic `values_by_context`, keyed by the selector result symbol;
- a declared `missing_context` policy;
- optional domain-level, context-level, and value-level origin metadata when
  available;
- optional producer fingerprints for the host inputs used to create the domain.

`missing_context` must be one of the policies accepted by the schema model:
`diagnostic`, `empty`, or `fallback` to a named flat domain. Fallback targets
must be metadata-domain references, not copied value lists, so validation,
completion, fingerprinting, and diagnostics share one definition.

### 7.3 Deterministic Snapshots and Fingerprints

Generated schema manifests are snapshots. The same host state and producer
configuration must produce the same canonical schema model and schema
fingerprint.

Producers must use stable symbolic IDs for resource-backed values. Host object
addresses, transient import IDs, localized display names, filesystem traversal
order, wall-clock time, or editor session state must not affect symbol identity.

Manifest content must be ordered deterministically before fingerprinting and
diagnostics. At minimum, producers must make ordering stable for:

- domain names;
- flat-domain values;
- contextual-domain context keys;
- contextual-domain values within each context;
- metadata-domain references;
- registry names and values;
- included origin and producer-fingerprint records.

The schema fingerprint must change when any canonical domain definition changes,
including:

- added, removed, or renamed domains;
- domain kind changes;
- added, removed, renamed, or reordered canonical values;
- selector changes;
- `values_by_context` changes;
- `missing_context` policy or fallback target changes;
- metadata definitions that reference different domains;
- inclusion or exclusion policy changes that affect exported domain content;
- included origin or producer-fingerprint changes that are part of the
  canonical manifest model.

Producer metadata that is explicitly non-canonical for diagnostics only must be
marked or modeled so it cannot accidentally perturb schema fingerprints.

### 7.4 Provenance and Diagnostics

When the host can provide provenance, generated manifests should include origins
for domains, contexts, and values. Origins may name a resource path, asset GUID,
asset database key, script/type/member, data-table row, import source, or other
stable host identifier. Producers should also include fingerprints for input
sets when the host can compute them cheaply and repeatably.

Origins and fingerprints are for diagnostics, LSP hovers, stale-schema checks,
and adapter troubleshooting. Dialogue diagnostics must still work when origin
metadata is absent. A missing origin must not make a valid dialogue invalid or
make an invalid dialogue valid.

### 7.5 Stale-Schema Checks

Adapters and standalone producers should provide a command or editor action
that reports whether the generated schema manifest is stale relative to the
producer's current host inputs. Host-agnostic checks compare the manifest's
recorded producer fingerprints, inclusion policy, schema export version, and
canonical schema fingerprint against a fresh export plan.

Where the host cannot expose reliable file or asset fingerprints, the adapter
must document the weaker check it can perform. The adapter may require an
explicit regenerate action, but it must not hide stale schemas by silently
falling back to editor state that Recite compiler, CLI, and LSP cannot reproduce.

Recite diagnostics should distinguish dialogue-source validation failures from
malformed manifests and stale-schema reports. Stale-schema reporting belongs to
producer and adapter tooling; compiler and LSP validation may surface it only
when the manifest carries enough producer metadata to make the check
reproducible without host access.

### 7.6 Host-Agnostic Example

This example models item inspection states. The dialogue uses a project metadata
key named `item`, and a second metadata key references a contextual domain keyed
by `metadata:item`.

```json
{
  "schema_version": 1,
  "metadata_domains": {
    "inventory_item": {
      "kind": "flat",
      "values": ["brass_key", "field_journal"],
      "origin": "content/items"
    },
    "inspection_state_all": {
      "kind": "flat",
      "values": ["new", "noticed", "examined"]
    },
    "inspection_state_by_item": {
      "kind": "contextual",
      "selector": "metadata:item",
      "values_by_context": {
        "brass_key": ["new", "noted_teeth", "matched_to_lock"],
        "field_journal": ["new", "skimmed", "decoded_margin_notes"]
      },
      "missing_context": {
        "policy": "fallback",
        "domain": "inspection_state_all"
      },
      "origin": "content/items"
    }
  },
  "metadata": {
    "item": {
      "targets": ["line", "choice"],
      "type": "symbol",
      "domain": "inventory_item"
    },
    "inspection_state": {
      "targets": ["line", "choice"],
      "type": "symbol",
      "domain": "inspection_state_by_item"
    }
  }
}
```

The producer owns the scan that turns host item definitions into
`inventory_item` and `inspection_state_by_item`. Recite validation only sees the
manifest snapshot. If a dialogue line uses `inspection_state=matched_to_lock`
without `item=brass_key`, validation follows the `missing_context` policy from
the manifest; it does not query the host item registry.

### 7.7 Engine Notes

Bevy producers may gather metadata domains from Rust builders, derives,
resources, asset collections, or editor-side export commands. They must emit the
same manifest shape whether the data came from reflected code, `AssetServer`
paths, or project data files.

Godot producers may gather metadata domains from imported resources, project
settings, C# registrations, GDScript registrations, or editor plugins. They must
snapshot Godot resource identities into stable Recite symbols rather than
requiring Recite compiler or LSP code to open the Godot project.

Unity producers may gather metadata domains from ScriptableObjects, importers,
GUID-addressed assets, C# attributes, Addressables, or editor tooling. They must
export stable symbols and fingerprints in the generated manifest; Recite
tooling must not depend on Unity editor APIs to validate dialogue.

## 8. Effects

Effects are typed requests emitted to the game. The runtime and adapter must not
execute game-side mutation as part of traversal.

Adapters must preserve effect mode semantics:

- deferred effects are collected and emitted at session end;
- immediate effects are emitted during traversal and do not require
  acknowledgement;
- blocking effects are emitted during traversal and pause the session until the
  host acknowledges the same `EffectRequestId`.

Adapters may offer generated typed effect events, signals, records, or message
wrappers. Those wrappers must preserve the original structured effect request
and must not hide unknown or schema-invalid effects.

## 9. Save and Load Handoff

Recite session state and host game state are separate. The adapter must provide
a host-native way to extract and restore the serialized Recite session state.

The runtime owns the session state shape (spec §8.6). The adapter must round-trip
the runtime's complete serialized session state as a single opaque unit. It must
not re-serialize a hand-picked subset of fields, because the snapshot includes
determinism-critical state — compiled asset identity, current block and statement
pointer, the call/divert stack, deterministic trace counters, previous prompt
choices, selected choice history, locale and variant, collected deferred effects,
and any pending blocking effect. Dropping any of these (for example, trace
counters or the divert stack) silently breaks deterministic resume, which the
core contract forbids. Treat the snapshot as opaque: serialize and restore what
the runtime produces, in full.

The adapter must not serialize arbitrary game state into Recite session state.
The host game owns its own save data and decides how to reconcile game-side
effects across save/load.

If a save occurs while a blocking effect is pending, restoring the Recite
session must preserve the pending effect identity. The runtime contract is that
the same effect ID is expected before traversal continues; the host game decides
whether the game-side operation should be replayed, fast-forwarded, or treated
as already complete.

## 10. Localisation

Adapters must start sessions with an explicit locale, a stable
project-configured locale, or source-text fallback. Adapters must not silently
derive the dialogue locale from the OS, editor, or engine environment; those
inputs may be used only when the project or author explicitly opts into that
policy. Locale fallback must be deterministic and must preserve the same
localized text, source text, line IDs, choice IDs, metadata, and markup that
the runtime exposes.

Adapters must expose grammatical variant selection (spec §9.5) as an explicit,
caller-driven choice — a session-level setter or a per-operation override that
threads into traversal. The runtime never infers a variant, so adapters must not
derive it from host environment or locale; lookup priority remains
`id&variant` → `id` → source text, and resolution must stay deterministic for a
given `(id, source, locale, variant, count)` tuple. The resolved text exposed in
§5 output reflects the selected variant.

Changing locale for an active session is not part of the v1 contract unless an
adapter documents and tests the exact behavior. Restarting the session with a
new locale is always acceptable. Variant selection, by contrast, may change
mid-session because it is an explicit per-lookup axis, not a session-rebuild.

Missing translations may use the runtime/compiler documented deterministic
fallback path. Malformed catalogs must surface a structured loading or
localisation error or diagnostic. Silent host-specific fallback chains are not
acceptable.

## 11. Changed Compiled Assets

Every adapter must choose, document, and test one of these changed-asset
policies for v1:

- `reject_refresh_until_session_ends`: if a compiled asset changes while a
  session is active, the adapter rejects the import or refresh attempt until
  the active session ends.
- `reload_for_next_session_only`: the adapter accepts the new compiled asset
  into the host asset cache, but the active session continues using its original
  compiled asset identity. The next session uses the new asset.
- `restart_required`: the adapter reports that the active session must be ended
  and restarted before the new compiled asset can be used.

Silent mid-session asset mutation is forbidden. It can break deterministic
traversal, save/load identity, previous prompt choice validation, pending
blocking-effect acknowledgement, and replay/test traces.

Adapters may explore richer mid-session patch reload after v1, but that feature
requires a separate design covering identity migration, pending prompts,
blocking effects, save/load, localization, and deterministic replay.

## 12. Error Categories

Adapters must surface structured errors with stable machine categories.
Host-specific error text may be added for diagnostics, but callers must be able
to match the category without parsing prose.

Required categories:

- `validation_error`;
- `asset_load_or_decode_error`;
- `stale_or_incompatible_asset_error`;
- `schema_mismatch_error`;
- `no_active_session_error`;
- `session_already_active_error`;
- `unknown_start_block_error`;
- `invalid_choice_error`;
- `unavailable_choice_error`;
- `stale_choice_error`;
- `missing_condition_handler_error`;
- `condition_evaluation_error`;
- `invalid_condition_result_error`;
- `effect_acknowledgement_error`;
- `rejected_changed_asset_refresh_error`;
- `save_load_incompatibility_error`;
- `localisation_error`.

Adapters should preserve source-backed diagnostics from the compiler and should
include host asset paths or resource identifiers when available.

## 13. Adapter Conformance Scenarios

Each adapter must have automated conformance coverage for host-independent
semantics:

- load/decode failure;
- stale compiled asset or schema mismatch;
- one-active-session rejection;
- start from default and explicit block;
- prompt selection by `ChoiceId`;
- unavailable or stale choice rejection;
- blocking effect acknowledgement and wrong-ID rejection;
- blocking effect save/load with same pending effect ID;
- pure condition handler dispatch;
- missing condition handler error;
- immediate, blocking, and deferred effect emission;
- save/load with a pending prompt;
- locale fallback or localization failure behavior;
- the adapter's declared changed-asset policy.

Host-runtime tests may be adapter-specific, but the expected Recite trace should
remain engine-independent where practical. Manual checks are acceptable only for
editor/import UX that cannot reasonably be automated.

Conformance above covers semantics. Adapters must also meet the engine-adapter
performance expectations in spec §19.6, including negligible cost when no session
is active.

## 14. Per-Engine Guidance

Godot, Bevy, and Unity are the v1-facing adapter targets. This document treats
that set as settled product scope and defines the contract each target must
preserve. The serious v1 gate (spec §16.5) requires all three adapters to be
production-quality and to pass the §13 conformance coverage; one adapter does
not satisfy the gate on its own.

### 14.1 Godot

Godot adapters should expose compiled dialogue as resources or imported assets
where possible. A session owner may be a node, autoload service, or resource
manager, depending on the final adapter shape.

Godot-facing APIs should support C# and/or GDScript surfaces. Runtime output
should map naturally to signals, typed C# events, or pull-style methods without
requiring dialogue files to call engine scripts.

Authoring import should fit Godot's resource workflow. The adapter must declare
one changed-asset policy from this document and make rejected refresh or restart
requirements visible in the editor or runtime logs.

### 14.2 Bevy

Bevy adapters should feel like Rust and ECS. Compiled dialogue should fit the
asset system where practical. Active session ownership may use resources,
components, or a dedicated session resource.

Runtime output should map naturally to events, messages, resources, or systems.
Condition handlers should be ordinary typed Rust game code. Effect requests may
be emitted as generic Recite events and optionally as schema-generated typed
events.

The adapter should keep frame cost negligible when no dialogue session is
active and should make headless tests practical without a full rendered game.

### 14.3 Unity

Unity adapters should expose a C#-native package shape. Compiled dialogue may be
represented as imported assets, ScriptableObject-backed resources, or another
Unity-native asset form that preserves compiled asset identity and freshness.

Runtime output should map naturally to C# events, UnityEvent-style hooks where
appropriate, or service responses. Condition and effect bindings should feel
like ordinary typed C# game code and must still export or consume the canonical
Recite schema manifest.

GameObject-facing and DOTS-facing Unity facades should share the same adapter
core. The DOTS surface may use Entities components, systems, buffers, or baked
data, but it must not fork Recite traversal, asset identity, save/load,
localisation, error, or changed-asset semantics away from the non-DOTS Unity
surface.

Editor import should make the edit/save/build/import/restart loop explicit.
The adapter must declare one changed-asset policy from this document before it
is treated as v1-ready.

### 14.4 Post-v1 Evaluation Targets

Unreal and GameMaker are post-v1 evaluation targets only. Their future adapters
must preserve this same contract, but they must not expand the v1 adapter scope.

## 15. Illustrative API Sketches

The snippets below are illustrative. They name contract concepts that adapter
implementations may share later, but they are not public API commitments.

### 15.1 Host-Agnostic Concepts

```text
CompiledAssetIdentity
  project_id
  asset_id
  compiled_fingerprint
  schema_fingerprint
  compiler_compatibility_version

AdapterSessionOwner
  start(asset, block?, locale) -> OutputBatch | AdapterError
  select(choice_id) -> OutputBatch | AdapterError
  acknowledge(effect_request_id, effect_ack) -> OutputBatch | AdapterError
  snapshot() -> SessionSnapshot | AdapterError
  restore(snapshot, asset) -> OutputBatch | AdapterError
```

### 15.2 Rust/Bevy-Flavoured Sketch

```rust
// Illustrative only: not a committed API.
fn start_dialogue(
    mut session: ResMut<ReciteSessionOwner>,
    asset: Res<Assets<ReciteDialogueAsset>>,
    mut output: EventWriter<ReciteOutput>,
) -> Result<(), ReciteAdapterError> {
    let events = session.start(asset.id(), Some(BlockId::new("intro")), Locale::new("en-US"))?;
    output.send_batch(events);
    Ok(())
}

fn select_choice(
    mut session: ResMut<ReciteSessionOwner>,
    mut selected: EventReader<ReciteChoiceSelected>,
    mut output: EventWriter<ReciteOutput>,
) -> Result<(), ReciteAdapterError> {
    for event in selected.read() {
        output.send_batch(session.select(event.choice_id.clone())?);
    }
    Ok(())
}
```

### 15.3 Godot-Flavoured Sketch

```csharp
// Illustrative only: not a committed API.
public partial class ReciteDialogueNode : Node
{
    [Signal] public delegate void OutputEventHandler(ReciteOutput output);
    [Signal] public delegate void AdapterErrorEventHandler(ReciteAdapterError error);

    public Error Start(ReciteDialogueResource asset, string block, string locale);
    public Error SelectChoice(string choiceId);
    public Error AcknowledgeEffect(string effectRequestId, ReciteEffectAck ack);
    public ReciteSessionSnapshot Snapshot();
}
```

### 15.4 Unity-Flavoured Sketch

```csharp
// Illustrative only: not a committed API.
public sealed class ReciteDialogueService
{
    public event Action<ReciteOutput> Output;
    public event Action<ReciteAdapterError> Error;

    public Result Start(ReciteDialogueAsset asset, string block, CultureInfo locale);
    public Result SelectChoice(ChoiceId choiceId);
    public Result AcknowledgeEffect(EffectRequestId effectRequestId, EffectAck ack);
    public ReciteSessionSnapshot Snapshot();
}
```

## 16. Shared Crate Boundary

The contract does not require a shared adapter crate for v1 design work.
Adapters may initially depend on `recite-core` and `recite-runtime` directly.

A future shared helper crate may be justified if Godot, Bevy, and Unity adapter
MVPs repeat the same stable concepts, such as compiled asset identity,
freshness checks, adapter error categories, changed-asset policy names, or
session snapshot handoff helpers. That decision belongs in follow-up adapter
implementation work, not this contract document.

## 17. Follow-up Prerequisites

This contract unblocks adapter implementation and refresh planning for #79,
#80, #82, #107, #108, #120, #121, #122, #123, and #94. Follow-up issues should
reference this document when choosing:

- their host asset import and freshness behavior;
- their active-session owner shape;
- their changed-asset policy;
- their start/select/ack API;
- their condition and effect binding surface;
- their save/load handoff;
- their conformance tests.
