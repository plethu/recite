# Conditions and effects

Part of the [production specification](../recite-production-spec.md). These are
requirements; implementation and release readiness require evidence from code,
tests and the current GitHub milestone. Section numbers remain stable.

## 6. Conditions

### 6.1 Condition Language

Conditions must support:

- named external function calls;
- typed scalar arguments;
- `and`;
- `or`;
- `not`;
- parenthetical grouping;
- arbitrary nesting;
- clear precedence rules.

Example:

```text
familiarity_gte(hazel, rhea, 3)
and not thread_completed(rhea_job_response)
```

Identifiers such as actor IDs, thread IDs, and stage IDs should be accepted as bare tokens. Quoted string literals should be reserved for values that genuinely need spaces or punctuation beyond the identifier grammar. Dialogue prose itself must never require quotes.

The grammar must be formally specified and parsed into an AST.

### 6.2 Condition Semantics

Conditions are pure queries. They must not mutate dialogue state or game state.

The runtime evaluates conditions through a caller-provided context:

```rust
pub trait DialogueContext {
    fn evaluate_condition(
        &self,
        function: &str,
        args: &[Value],
    ) -> Result<bool, DialogueError>;
}
```

The core runtime should not know project-specific condition meanings.

Condition functions return either a boolean (the default, used by `:if` and choice `requires=(...)` clauses) or a schema-declared enum variant (used by `:match` scrutinees, see §5.9.1). An enum-returning function declares its return type in the canonical schema model; the dialogue context exposes it through the same `evaluate_condition` path or a sibling enum-returning lookup, depending on adapter ergonomics.

### 6.3 Schema Validation

All condition functions must be declared in schema.

Validation must reject:

- unknown condition functions;
- wrong arity;
- wrong argument types;
- invalid literal values where a schema defines an enum or registry;
- non-boolean condition expressions in `:if` and choice `requires=(...)` clauses;
- non-enum-returning scrutinees in `:match`;
- `:match` arms that reference variants not declared in the scrutinee's enum;
- non-exhaustive `:match` (no `:case _` and at least one declared variant uncovered);
- duplicate `:case <variant>` arms in a single `:match`.

## 7. Effects

### 7.1 Effect Model

The previous term "mutation" is too narrow. The production system should use **effects**.

Effects are typed intents emitted by dialogue. The runtime never executes them.

```rust
pub struct DialogueEffectRequest {
    pub id: EffectRequestId,
    pub mode: EffectMode,
    pub function: String,
    pub args: Vec<Value>,
    pub source: EffectSource,
}

pub enum EffectMode {
    Deferred,
    Immediate,
    Blocking,
}
```

### 7.2 Deferred Effects

Deferred effects are collected during traversal and returned when the scene ends.

Use cases:

- advance story thread;
- record relationship interaction;
- mark scene as seen;
- commit relationship deltas.

Example:

```text
! deferred advance_thread(rhea_job_response, tired)
! deferred record_relationship_interaction(hazel, rhea, incidental_encounter)
```

### 7.3 Immediate Effects

Immediate effects are yielded to the caller as soon as encountered. The runtime may continue after the caller observes the event.

Use cases:

- play sound cue;
- fire presentation-only analytics;
- trigger non-blocking animation cue.

Example:

```text
! immediate play_sfx(snap)
```

Metadata may cover many presentation cues, but immediate effects are useful when a cue has event semantics rather than descriptive line metadata.

### 7.4 Blocking Effects

Blocking effects are yielded immediately and pause dialogue traversal until explicitly acknowledged.

Use cases:

- "Here, I'll mark it on your map."
- wait for camera pan to complete;
- wait for item grant animation;
- open a UI overlay and resume after close.

Example:

```text
! blocking mark_map(old_watchtower)
```

Runtime API:

```rust
pub fn acknowledge_effect(
    session: &mut DialogueSession,
    effect_id: EffectRequestId,
    result: EffectAck,
) -> Result<(), DialogueError>;
```

Initial production scope should only require completion/failure acknowledgement:

```rust
pub enum EffectAck {
    Completed,
    Failed { reason: String },
}
```

Result-dependent branching should be deferred until there is a proven need. If dialogue needs to branch on the result of a game operation, the game should update state and later dialogue should query that state through conditions.

### 7.5 Effect Ordering

Effects must be emitted and collected in declaration order.

Normative placement rule:

- Effects are standalone statements (`!`) emitted in source order between dialogue events. Effects do not appear inside a line's prose body.
- Per-line presentation cues (portrait, pose, sfx, delay, focus, shot) use metadata on the line header.
- Deferred effects are appended to the session's deferred-effect list when traversal reaches their statement, and surface to the caller when the scene ends.
- Immediate and blocking effects emit as `DialogueEvent::Effect` in the source order they are encountered.

### 7.6 Effect Schema

All effects must be declared in schema.

Validation must reject:

- unknown effect functions;
- wrong arity;
- wrong argument types;
- unsupported mode for that effect;
- invalid enum/registry values.

Preferred adapter registration example:

```rust
schema
    .effect("advance_thread")
    .deferred()
    .param::<ThreadId>("thread_id")
    .param::<ThreadStageKind>("stage");

schema
    .effect("record_relationship_interaction")
    .deferred()
    .param::<ActorId>("actor_a")
    .param::<ActorId>("actor_b")
    .param::<RelationshipInteractionKind>("kind");

schema
    .effect("mark_map")
    .blocking()
    .param::<LocationId>("location_id");

schema
    .effect("play_sfx")
    .immediate()
    .param::<DialogueSoundEffectId>("sound_effect_id");
```

Generated manifest excerpt:

```json
{
  "effects": {
    "advance_thread": {
      "modes": ["deferred"],
      "params": [
        { "name": "thread_id", "type": "registry:thread" },
        { "name": "stage", "type": "enum:thread_stage_kind" }
      ]
    },
    "play_sfx": {
      "modes": ["immediate"],
      "params": [{ "name": "sound_effect_id", "type": "registry:dialogue_sound_effect" }]
    }
  }
}
```
