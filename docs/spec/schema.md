# Schema

Part of the [production specification](../recite-production-spec.md). These are
requirements; implementation and release readiness require evidence from code,
tests and the current GitHub milestone. Section numbers remain stable.

## 10. Schema

### 10.1 Schema Scope

The schema must define:

- condition functions, including their parameter types and optional `returns` enum type for `:match` scrutinees;
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

The schema has three separate surfaces:

1. a canonical Rust model in `recite-core`;
2. a generated schema manifest consumed by `recite-compiler`, `recite-cli`, and
   `recite-lsp`;
3. producer-specific authoring surfaces that create the manifest.

For engine projects, the preferred producer is adapter or game code, not a
parallel hand-authored schema configuration. Game projects already define typed
handles, effect handlers, condition queries, enum state, speakers, and
registries near their adapter code. Presentation projection query functions,
projector definitions, and label templates may also originate in adapter or
game code. Standalone projects without an engine producer must instead have a
source-owning declarative producer path; the GUI edits that source and invokes
deterministic generation rather than editing the manifest.

The writer's explicit source-navigation and generation registration is specified
in [Schema producer registration](../schema-producer-registration.md). Registration
loading never executes a command; validated generated output is published only
after an author-requested generation succeeds.

#### 10.2.1 Standalone TOML source contract

The standalone source-owning producer uses a versioned, map-shaped TOML
document. Its root `schema_version` is the numeric marker `1`; named schema
declarations use TOML tables keyed by declaration name, while arrays are
ordered only where the canonical model gives them semantic order. A
`[producer]` table with a non-empty `id` is mandatory. Standalone TOML has the
fixed producer kind `standalone` (an explicit `kind` is validated, not chosen
by the author). For availability-reason arguments, TOML uses explicit tagged
values, for example `{ kind = "binding", name = "actor" }` or
`{ kind = "literal", value = "calm" }`; the JSON `$name` shorthand is not a
TOML form. For availability-reason mapping literals, generated JSON escapes a
leading `$` by doubling that character (`$name` remains a binding and `$$name`
is a literal `$name`); other JSON literal fields retain their existing exact
string semantics. This is a format boundary, not a tagged JSON form.

TOML duplicate keys or tables are syntax errors from the TOML frontend.
Declaration/table order is nonsemantic. The source owner retains the CST and
source spans: an unedited `source_text()` is the exact input, including line
endings and final-newline policy, and typed edits preserve that policy and
untouched trivia. TOML lowers directly through the canonical manifest raw and
validation path; it is not converted through JSON or a second mutable schema
model. Generated JSON is deterministic, read-only, retains provenance and
producer freshness metadata, and is accepted by the existing manifest loader.
The source fingerprint includes producer identity and source-owned semantic
content while ignoring trivia, map order, generated fields, and diagnostic-only
provenance; the semantic schema fingerprint remains distinct and provenance
safe. Producer IDs and fingerprints are the linkage used for stale-output
comparisons.

Producer APIs should be native to their host ecosystem. A Bevy adapter should
feel like Rust, Godot adapters should support Godot-facing C# and/or GDScript
surfaces, Unity should feel like C#, LÖVE should feel like Lua, and future
adapters should follow the language their users already write. Those producer
APIs may differ, but they must all export the same generated manifest and pass
the same Recite manifest validation suite.

Adapter registration should feel like ordinary typed game code. The Bevy/Rust
adapter should support a builder style for explicit central registration:

```rust
schema
    .condition("trust_gte")
    .param::<ActorId>("actor_a")
    .param::<ActorId>("actor_b")
    .param::<i32>("threshold")
    .returns_bool();

schema
    .condition("thread_stage")
    .param::<ThreadId>("thread_id")
    .returns_enum::<ThreadStageKind>();

schema
    .effect("play_sfx")
    .immediate()
    .param::<DialogueSoundEffectId>("sound_effect");
```

The Bevy/Rust adapter should also support derive or macro-based declarations
from the start. Builder registration and derive declarations serve different
ergonomic needs, and both lower into the same canonical model:

```rust
#[derive(ReciteEffect)]
#[recite(name = "play_sfx", mode = "immediate")]
struct PlaySfx {
    sound_effect: DialogueSoundEffectId,
}
```

The generated manifest is a deterministic, language-neutral data artifact.
It is the only schema surface the compiler and LSP must understand. Compiler
and editor tooling must not execute game code to validate dialogue.

Generated manifests are compiler and LSP truth, but they are read-only derived
artifacts: neither the GUI nor an editor may edit them directly. The shared
authoring kernel must expose a source-owning schema-authoring capability. It
must define at least one source-owning, kernel-editable declarative producer
path suitable for GUI integration for standalone projects and producer-backed
actions for engine-owned schemas. Those actions open the source declaration,
invoke/regenerate through the producer, report stale output, surface structured
failure and retry, and never write generated manifests directly. Unsupported
producers are explicitly read-only and must not be counted as schema editing.
The standalone source format is the TOML contract above. It lowers to the same
canonical model while preserving producer provenance and deterministic
regeneration. The GUI workbench must expose at least one standalone path.

The host-agnostic export contract for adapter-produced manifests, including
resource-backed metadata domains, presentation projection declarations,
snapshot determinism, provenance, and stale-schema checks, lives in
`docs/engine-adapter-contract.md` §7. This section defines the canonical schema
model that those producers must lower into.

The manifest format for v1 should be JSON unless implementation evidence shows
that another data format materially improves the toolchain. JSON is widely
generated by game tooling, easy for editor integrations to read, and adequate
because the manifest is produced by adapters rather than hand-authored as the
primary developer interface. The manifest is canonical only after parsing into
the typed Rust model and sorting map-like collections deterministically for
fingerprinting and diagnostics.

Recite should publish a JSON Schema for the generated manifest format. That
JSON Schema validates manifest document shape only: required fields, allowed
keys, scalar types, array/object structure, effect mode strings, and basic
version compatibility. It is a useful public contract for adapter authors,
CI checks, editor IntelliSense, and people inspecting generated manifests.

The JSON Schema and manifest loader must classify adapter-produced provenance
and producer metadata consistently with `docs/engine-adapter-contract.md` §7.
Optional fields such as domain origins, value origins, context origins,
producer fingerprints, schema export versions, and inclusion policies must be
accepted only in their documented shapes. The loader must either preserve them
for diagnostics, hovers, and stale-schema tooling or explicitly ignore
non-canonical producer metadata; it must not accidentally treat diagnostic-only
metadata as semantic validation input.

The v1 pre-1.0 contract deliberately resets producer-origin syntax: origins
are structured objects containing `kind` and `id` (and optional `label`), not
legacy strings. This intentional migration break is reported directly by the
loader so a producer can regenerate its export. Namespaced producer-origin
extension fields are retained as diagnostic-only JSON values and never affect
the semantic schema fingerprint. Generated contextual domains must carry
their resolved `missing_context` policy explicitly; a source-owning producer
may resolve an omitted authoring option to `diagnostic` before exporting JSON.

JSON Schema is not the authority for Recite semantics. After document-shape
validation, Recite must lower the manifest into the canonical Rust model and
run semantic validation there. Semantic validation owns duplicate definitions,
unknown type references, registry/value checks, condition return compatibility,
effect arity/type checks, metadata target policy, markup policy, projection
query function references, projector input/output references, presentation label
placeholders, diagnostics, and deterministic fingerprinting.

The Rust schema model should live in `recite-core::schema` and include:

- `ProjectSchema`;
- `ProducerMetadata`, including optional typed producer identity, overall
  content fingerprint, export version, inclusion policy, and content freshness
  fingerprints kept outside the semantic schema fingerprint;
- `SchemaTypeDefinition`, including enum definitions;
- `SchemaTypeRef`, covering built-in scalar types, speaker IDs, enum types, and
  registry-backed IDs, and the metadata-only `symbol` scalar;
- `ConditionDefinition`, including typed parameters and optional enum return
  type, and optional availability reason mapping;
- `AvailabilityReasonDefinition`, including localisable template text and
  typed parameters;
- `EffectDefinition`, including typed parameters and supported modes;
- `MetadataDefinition`, including targets, type, repeatability, and optional
  range constraints, and optional domain reference;
- `MetadataDomainDefinition`, including flat value sets, contextual value
  selectors, and optional origin/fingerprint metadata for adapter-produced
  manifests;
- `ProjectionQueryFunctionDefinition`, including typed parameters, return type,
  and optional per-event call bound;
- `SchemaPresentationProjectorDefinition`, including candidate selectors, typed
  inputs, query calls, output definitions, and label templates;
- `PresentationLabelDefinition`, including stable localisable template ID,
  source text, and typed placeholders;
- `MarkupDefinition`, including closing, translatability, and nesting policy;
- `SpeakerDefinition`;
- `RegistryDefinition`, including value snapshots and optional
  origin/fingerprint metadata.

Metadata domains are named schema definitions. Metadata definitions reference
domains by name rather than hardcoding special keys such as `portrait`.

`symbol` is a metadata schema scalar, not a new runtime value kind. A metadata
definition with `"type": "symbol"` accepts source
`SourceMetadataValue::Scalar(SourceMetadataScalar::Symbol(_))` values, rejects
quoted string literals unless a different metadata type permits them, and
lowers the accepted symbol into the compiled/runtime metadata value model as a
string-like value with schema-validated domain semantics. Runtime consumers
must use the metadata key and schema contract to interpret that value; they
must not depend on source spelling.

Domain kinds:

- flat domains declare a deterministic set of valid symbol values;
- contextual domains select the valid symbol values from another source item
  field or metadata key.

V1 contextual selector scope is deliberately small:

- `field:speaker` resolves the line speaker first, then the inherited block
  default speaker;
- `metadata:<key>` resolves metadata with `<key>` on the same source item. It
  succeeds only when that key appears exactly once on the item and the value is
  a scalar symbol after source-value lowering. An absent key follows the
  domain's missing-context policy. Repeated keys, arrays, quoted strings, and
  non-symbol scalar values are selector-shape diagnostics because they would make
  compiler and LSP resolution ambiguous.

Block-wide and project-wide selectors are deferred until a concrete
implementation issue needs them.

Contextual domains must declare a missing-context policy. The default is
`diagnostic`, which reports that the selector could not be resolved. Other
allowed policy values are `empty`, which produces no valid values or
completions, and `fallback`, which falls back to a named flat domain declared in
the same `missing_context` object. Fallback targets must be flat domains so
diagnostics and completions remain deterministic.

Compiler validation, CLI validation, and LSP completions/diagnostics must
consume the same manifest-backed metadata domain rules. The compiler is the
authority for acceptance; LSP behavior is a live authoring projection of the
same domain resolution.

The generated manifest should be self-contained enough for validation without
running the game. Registry-backed values should therefore be emitted as stable
snapshots, optionally with source/origin metadata and fingerprints so tooling
can explain where a value came from. If an adapter needs to read game data to
build those snapshots, that happens during the explicit schema export command,
not during normal Recite compilation or editor diagnostics.

Adapter and standalone producer responsibilities for scanning host resources,
exporting flat and contextual metadata-domain snapshots, recording provenance,
and reporting stale manifests are normative in `docs/engine-adapter-contract.md`
§7 and should not be redefined differently by engine-specific adapters.

#### 10.2.3 Availability Reason Definitions

Availability reasons are schema-owned reusable templates for explaining visible-but-unavailable choices. They give adapters, CLI/TUI, LSP, tests, and localisation tools structured data without requiring the core runtime to invent prose.

Rules:

- `availability_reasons` is a schema-level map keyed by stable reason ID.
- Each reason declares localisable source template text and typed parameters.
- Generated schema manifests must include enough reason-template data, parameter types, and provenance for compiler, LSP, CLI/TUI, runtime, and adapter tooling to validate and present reasons without executing game code.
- Template text is dialogue/project content, not Recite-owned UI text. It follows
  the dialogue localisation path, not the shared Recite UI Fluent resource
  contract used by CLI/TUI, GUI, LSP, and editor extensions.
- Boolean condition definitions may declare an `availability_reason` mapping. Mapping values bind reason parameters from condition arguments using `$<condition_param>` references or literal values valid for the target parameter type. In generated JSON, a literal leading dollar is escaped as `$$`; TOML uses the explicit `literal` tag instead.
- The compiler validates that condition reason mappings reference existing reason IDs, bind every required reason parameter exactly once, do not bind unknown parameters, and produce values compatible with the reason parameter types.
- A choice-level `reason=<id>` primary reason override must reference an
  existing parameterless availability reason in v1. Referencing a parameterised
  reason is a compiler diagnostic until an explicit binding syntax is designed;
  the compiler must not guess bindings from condition arguments.
- Negated conditions and compound expressions do not synthesize new reason prose. They may carry leaf reasons for positive condition calls where the boolean grouping preserves meaning, or no leaf reason where the schema cannot explain the failure safely.

Example choice and schema pairing:

```text
? ask_news@c2bdeae1465bfa65bcf4 requires=(trust_gte(innkeeper, player, 3)) reason=innkeeper_trust_hint
  What's the real news?
  -> local_news_private
```

The primary reason override above uses the reusable `innkeeper_trust_hint` template instead of repeating prose on every choice. The compiler may still preserve any schema-derived detailed reason tree for trace and adapter output.

Example generated manifest excerpt:

```json
{
  "schema_version": 1,
  "types": {
    "thread_stage_kind": {
      "kind": "enum",
      "values": ["fresh", "tired", "angry", "fine", "completed"]
    }
  },
  "registries": {
    "dialogue_sound_effect": {
      "values": ["snap", "door_close", "rain_window"],
      "origin": {
        "kind": "asset_path",
        "id": "data/content/dialogue-sound-effects.toml"
      }
    }
  },
  "speakers": {
    "rhea": {},
    "hazel": {}
  },
  "metadata_domains": {
    "portrait_all": {
      "kind": "flat",
      "values": ["flat", "concerned", "wry"]
    },
    "sound_effect": {
      "kind": "flat",
      "values": ["snap", "door_close", "rain_window"]
    },
    "portrait_by_speaker": {
      "kind": "contextual",
      "selector": "field:speaker",
      "values_by_context": {
        "rhea": ["flat", "concerned"],
        "hazel": ["flat", "wry"]
      },
      "missing_context": {
        "policy": "fallback",
        "domain": "portrait_all"
      }
    },
    "emotion_by_subject": {
      "kind": "contextual",
      "selector": "metadata:subject",
      "values_by_context": {
        "rhea": ["calm", "hurt", "angry"],
        "hazel": ["calm", "guarded", "wry"]
      },
      "missing_context": { "policy": "diagnostic" }
    }
  },
  "conditions": {
    "thread_stage": {
      "params": [{ "name": "thread_id", "type": "registry:thread" }],
      "returns": "enum:thread_stage_kind"
    },
    "trust_gte": {
      "params": [
        { "name": "actor_a", "type": "registry:actor" },
        { "name": "actor_b", "type": "registry:actor" },
        { "name": "threshold", "type": "int" }
      ],
      "returns": "bool",
      "availability_reason": {
        "reason": "trust_too_low",
        "args": {
          "subject": "$actor_a",
          "target": "$actor_b",
          "threshold": "$threshold"
        }
      }
    }
  },
  "availability_reasons": {
    "trust_too_low": {
      "template": "{subject} does not trust {target} enough.",
      "params": [
        { "name": "subject", "type": "registry:actor" },
        { "name": "target", "type": "registry:actor" },
        { "name": "threshold", "type": "int" }
      ]
    },
    "innkeeper_trust_hint": {
      "template": "The innkeeper is not ready to share that.",
      "params": []
    }
  },
  "effects": {
    "play_sfx": {
      "modes": ["immediate"],
      "params": [{ "name": "sound_effect", "type": "registry:dialogue_sound_effect" }]
    }
  },
  "metadata": {
    "portrait": {
      "targets": ["line"],
      "type": "symbol",
      "domain": "portrait_by_speaker"
    },
    "sfx": {
      "targets": ["line", "choice"],
      "type": "symbol",
      "domain": "sound_effect",
      "repeatable": true
    }
  },
  "markup": {
    "slow": { "requires_closing": true, "translatable": true },
    "shake": { "requires_closing": true, "translatable": true }
  }
}
```

Hand-authored schema configuration may exist as a fallback for standalone
experiments, tests, or projects without an adapter. That fallback must lower
into the same `ProjectSchema` model and must not become the primary integration
contract for typed game projects.

Schema freshness is part of the authoring contract:

- compiled assets compare against the current schema manifest fingerprint;
- adapter tooling should provide a command to regenerate the manifest;
- adapter tooling should provide a check that reports stale generated schema
  manifests where the host ecosystem can support it;
- Recite diagnostics should clearly distinguish dialogue errors from stale or
  malformed schema manifest errors.

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

Schema validation should use the same `Diagnostic` model as parser and compiler
validation. The compiler should expose shared diagnostic factories or a shared
diagnostic catalog for schema-related checks so CLI, LSP, and test fixtures use
the same stable codes and messages.

Source-backed diagnostics must point at the smallest useful value-specific span
available:

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

When a schema manifest is generated from adapter code, the manifest may include
producer origin metadata for definitions, metadata domains, metadata-domain
contexts, metadata-domain values, and registry values. Recite diagnostics may
surface that origin as related context, but dialogue-source diagnostics must
remain valid even when producer origins are unavailable.
