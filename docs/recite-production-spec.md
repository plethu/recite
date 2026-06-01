# Recite - Production Specification

## 1. Purpose

Recite is an open-source deterministic dialogue compiler, runtime, editor, and tooling suite for narrative-heavy games.

Its primary audience is developers building games where dialogue must be:

- testable through programmatic fixtures and snapshot tests;
- deterministic across replay, save/load, and CI;
- integrated with explicit game state boundaries rather than ad hoc runtime callbacks;
- localisable through stable gettext-style workflows;
- portable across engine integrations without tying the dialogue model to one engine's scripting language;
- authorable through excellent text tooling, with a visual editor as a structured companion rather than the only workflow.

Recite should be a credible replacement for existing dialogue tools when their tradeoffs do not fit a project's narrative, tooling, or architecture needs. The motivating pain points are specific:

- localisation workflows that depend on unstable text or ad hoc IDs;
- editor tooling that cannot catch enough content mistakes before runtime;
- dialogue scripts that can call directly into engine scripting or mutate game state;
- one-off authoring languages whose concepts do not travel well outside that tool;
- runtime behaviour that is difficult to replay, test, save, load, or inspect deterministically;
- asset-store or engine-specific packaging that makes the dialogue model feel less portable than the game needs.

This is not a claim that ink, Yarn Spinner, Godot-native tools, or other dialogue systems are bad fits for all projects. Recite is specifically for projects that value portable narrative-system thinking, strict architectural boundaries, reproducible execution, schema-checked integration, and tool-assisted content validation.

## 2. Core Invariants

The following invariants define the project and must not be weakened for convenience:

1. Dialogue traversal is deterministic.
2. The runtime never performs game-side effects.
3. Game-side effects are emitted as typed, schema-checked effect requests.
4. Dialogue state is serialisable and deserialisable.
5. Dialogue files can be validated without running the game.
6. Localisable strings use stable IDs that survive nearby edits.
7. Runtime data surfaces speaker, line, choice, metadata, and effect information as structured values, not conventions parsed from prose.
8. Tooling is part of the product, not an optional afterthought.
9. Stable IDs are author-visible and never silently rewritten by tooling. Renames go through an explicit code action.

## 3. Terminology

- **Dialogue source**: Human-authored text file in the dialogue DSL.
- **Compiled dialogue asset**: Binary or structured compiled representation consumed by runtimes and adapters.
- **Block**: Named unit of dialogue execution, equivalent to an ink knot or Yarn node.
- **Line**: Atomic localisable dialogue/narration output.
- **Prompt**: A line, optional line, or UI state that presents choices.
- **Choice**: Player-selectable option with stable ID, localisable text, availability, metadata, and optional echo policy.
- **Condition**: Pure query against game state, evaluated through a caller-provided context.
- **Effect request**: Typed intent emitted by dialogue. The runtime does not execute it.
- **Deferred effect**: Effect collected and returned when the scene ends.
- **Immediate effect**: Effect yielded immediately while traversal may continue.
- **Blocking effect**: Effect yielded immediately and requiring explicit acknowledgement before traversal continues.
- **Metadata**: Ordered key/value annotations attached to lines, choices, blocks, or scenes.
- **Inline markup**: Markup embedded inside localisable text, such as `[slow]...[/slow]`.
- **Scene manifest**: Project-level mapping from scene IDs to dialogue assets, start blocks, participants, and presentation hints.

## 4. Product Shape

The reference implementation should be delivered as a Rust workspace. Rust is the implementation language for the core toolchain, not a requirement that users build Rust-first games.

The workspace should contain:

- `recite-core`: AST, identifiers, value model, diagnostics, schema model.
- `recite-parser`: DSL parser and source mapping.
- `recite-compiler`: compiler, validator, POT extractor, compiled asset writer.
- `recite-runtime`: deterministic runtime with no engine dependencies.
- `recite-cli`: project CLI, exposing the `recite` binary.
- `recite-lsp`: language server.
- engine adapter crates as integrations mature, such as `recite-godot`,
  `recite-bevy`, or `recite-unity`.
- `recite-vscode`: VS Code extension.

Visual-editor surfaces are deferred until text tooling is mature; the crate split (`recite-editor-core`, `recite-visual-editor`) will be designed when that work begins, not pre-declared here.

Neovim support ships as documented LSP-client config and an optional Tree-sitter grammar, not a Rust crate.

## 5. Source Format

### 5.1 Requirements

The format must be human-readable, line-oriented where practical, and formally specified with a grammar. Writers must not need to understand general programming beyond variables, function-style conditions, simple boolean logic, and structured annotations.

Recite has a small domain language because dialogue has structure that should be named directly: blocks, lines, choices, stable IDs, conditions, metadata, and effects. The format must teach a portable way of thinking about narrative systems, not a one-off bridge into a specific engine scripting language.

Dialogue prose must not be written as quoted string literals. Quoted prose creates the same awkward formatting pressure as long strings in source code. Recite source should treat dialogue text as indented body text owned by a structured statement header.

The source format should be indentation-first and must not mix one-line object literals, curly-brace blocks, and ad hoc nested styles. The concrete grammar should use a small, consistent statement vocabulary:

```text
:: block_name default      # block
> line_id                  # line
? choice_id                # choice
! mode effect(...)         # effect
-> target                  # goto
:if condition(...)         # conditional branch
:else                      # else branch
:match query(...)          # enum match
:case variant              # match arm
# comment                  # comment
```

Statement headers carry structured fields. Indented bodies carry prose and nested statements.

#### Indentation Rules

A `>` line's indented body holds prose. Prose continues until a sibling-indented line begins with one of `?`, `!`, `->`, `>`, `:if`, `:else`, or `::` — at that point the prose body ends and nested statements begin at the same indent column. Blank lines inside prose are preserved as paragraph breaks; blank lines do not by themselves terminate the prose body.

Nested statements inside a line body, conditional branch, or block share a single indent column. Mixing indent widths within a body is a parse error.

The format must support:

- named blocks;
- exactly one default block per file or project;
- block references within the same file;
- block references across files;
- localisable lines;
- localisable choices;
- structured speaker references;
- ordered metadata entries;
- conditional choices;
- conditional branches;
- effect declarations;
- comments;
- includes/imports;
- inline markup in text;
- stable source spans for diagnostics.

### 5.1.1 Parser Architecture

The production parser uses a rowan-style lossless syntax tree as the core parser foundation. Syntax parsing preserves source text, trivia, malformed regions, and recovery context, and reports syntax diagnostics with stable codes and spans.

Valid and partially valid syntax lowers into the `recite-core` source AST. That AST is the compiler-facing source model, not the parse tree. Parser responsibilities stop at syntax shape, source spans, trivia, malformed regions, recovery, and parse diagnostics.

Compiler-facing validation owns stable ID policy, references, schema checks, match exhaustiveness, semantic validation, and compiled output determinism. Runtime traversal must never depend on parser-only trivia or malformed syntax nodes.

Tree-sitter is not part of the v1 core parser. It remains a possible future editor integration for highlighting or structural editing after the rowan parser and lowering path are established.

Lossless syntax trees are heavier than AST-only parsing. Compiler and CLI flows should treat them as temporary parse artifacts and lower promptly. LSP flows may retain syntax trees and a live index for open or recently changed files.

### 5.2 Blocks

A dialogue file is organised into named blocks.

Each block may declare:

- `id`;
- optional metadata;
- optional default speaker context;
- a sequence of statements.

Example syntax:

```text
:: tavern_arrival default

> ta_001 speaker=innkeeper portrait=neutral
  Welcome to the Rusty Flagon. Haven't seen you in a while.
```

The concrete syntax should optimise for writer ergonomics and LSP implementation while preserving this structural shape.

### 5.3 Lines

A line is the atomic localisable output unit.

Each line must expose:

- `id`: stable string identifier;
- `speaker`: optional speaker identifier;
- `source_text`: localisable source text;
- `metadata`: ordered metadata entries;
- `inline_markup`: preserved in source text and validated separately;
- source location.

Speaker names must not be parsed from line text. The following is invalid as a speaker declaration:

```text
Rhea: Hello.
```

Instead, speaker must be structured in the line header:

```text
> rhea_001 speaker=rhea
  Hello.
```

Multiline prose is represented by the indented body:

```text
> rhea_014 speaker=rhea portrait=concerned
  I didn't know it was that bad.

  I mean, I knew it was bad.
  Just not... that bad.
```

Standalone effects (`!`) are top-level statements between dialogue events, not children of a line body. Per-line presentation cues belong in metadata. See §7.5.

### 5.4 Choices

Choices are first-class records.

Each choice must expose:

- `id`: stable localisable choice ID;
- `text`: source text;
- `metadata`: ordered metadata entries;
- optional `if` condition;
- `target`: block reference or `END`;
- `availability`: evaluated at runtime;
- `echo`: explicit echo policy.

Unavailable choices must be included in runtime output by default so callers can render disabled choices. The format may allow a choice to opt into hidden behaviour, but hiding must be explicit and visible to tooling.

Choice echo policies:

```text
echo = none
echo = selected_text
echo = line(choice_echo_001)
```

The default should be `none`. If a game wants the protagonist to repeat the selected choice, it should be an explicit authored output, not a runtime quirk.

### 5.4.1 ID Assignment Policy

Every line and choice must reach the compiler with a stable ID. The intended workflow:

- Authors may write line and choice headers without an ID. Example: `>` alone, or `> hazel_rhea.small_talk` with no suffix.
- The LSP inserts a deterministic-but-unique short suffix (4–6 base32 characters) into the source file on save, producing e.g. `> hazel_rhea.small_talk_WPUQ`. The suffix is selected to be unique within the project at insertion time.
- Once written to disk, IDs are **frozen**. The LSP never rewrites an existing ID. Renames go through the explicit `rename` code action and produce a structured rename record so translation tools can follow the change.
- The compiler errors if any line or choice lacks an ID. `recite check-ids` enforces the same.
- Because IDs do not encode content, translation files survive author edits to source text.

This policy keeps gettext-style translation stable: an edit to source text does not invalidate `msgctxt`. Auto-rewriting IDs based on content is an explicit non-goal.

### 5.5 Prompts

The runtime must be able to represent choices attached to a line. Many games present a prompt line and choices as one UI state.

The source format should support prompts as a line with nested choices:

```text
> ta_prompt_001 speaker=innkeeper portrait=neutral
  What do you need?

  ? ta_opt_room
    I need a room.
    -> get_room

  ? ta_opt_news if familiarity_gte(innkeeper, 3)
    What's the news?
    -> local_news
```

A prompt may also omit line text and present choices only:

```text
? ta_opt_room
  I need a room.
  -> get_room

? ta_opt_leave
  Never mind.
  -> END
```

### 5.6 Metadata

Metadata must be ordered and must allow repeated keys.

A plain string map is insufficient because existing production use cases include repeated cues such as multiple sound effects or ordered presentation hints.

Runtime representation:

```rust
pub struct MetadataEntry {
    pub key: String,
    pub value: Value,
    pub source_span: Option<SourceSpan>,
}
```

Source metadata values must distinguish author spelling from compiled/runtime
meaning. The source AST preserves this as:

```rust
pub enum SourceMetadataValue {
    Scalar(SourceMetadataScalar),
    Array(Vec<SourceMetadataScalar>),
}

pub enum SourceMetadataScalar {
    Symbol(String),
    StringLiteral(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
}
```

`SourceMetadataScalar` is the scalar subset: symbol, string literal, integer,
float, and bool. Nested arrays are not part of v1.

Metadata source spelling:

- bare values such as `portrait=grin` are symbols/reference tokens;
- quoted values such as `caption="Door closes"` are literal strings;
- integer, float, boolean, and array values remain typed literals;
- arrays validate each scalar element against the same metadata definition and
  domain rules as a single value;
- runtime-bound `$name` metadata values are reserved for explicit future
  support and must not be accepted silently as ordinary symbols; they are
  malformed until that support is added.

Compiled/runtime metadata semantics are schema-driven. Runtime consumers should
not infer meaning from whether a source value was bare or quoted; they consume
the compiled value after schema validation has assigned the allowed type and
domain.

Metadata values must support:

- string;
- integer;
- float;
- boolean;
- arrays of scalar values.

The core format must not hardcode keys such as `portrait`, `sfx`, `delay`, `shot`, `pose`, or `focus`. Those keys belong in project schema. The tooling must still make project-specific metadata validation excellent.

Migration note: existing examples, fixtures, and tests should leave
reference-like metadata values bare (`portrait=grin`, `sfx=chime`,
`speaker=rhea`). Literal display text or values that rely on spaces or
punctuation must be quoted. Existing generated fixtures that quote registry-like
presentation values are legacy inputs until the parser/schema implementation
issue updates them.

### 5.7 Inline Markup

Inline markup is allowed inside localisable text and must be preserved through extraction and runtime delivery.

Examples:

```text
> hazel_rhea.small_talk.005 speaker=rhea portrait=concerned
  [slow]I didn't know it was that bad.[/slow]

> hazel_rhea.small_talk.002 speaker=hazel portrait=flat
  [shake]Yeah, funny.[/shake]
```

The project must provide markup validation:

- balanced tags;
- known tag names from schema;
- required tags preserved in translation;
- no invalid nesting where a tag schema forbids nesting;
- source spans for invalid markup.

The runtime does not interpret inline markup. Presentation layers may interpret it.

The bracketed tag form `[name]...[/name]` is deliberately distinct from ink's `[choice text]` convention. The visual collision is acknowledged; the bracket form is chosen for parser simplicity and translator familiarity.

### 5.8 Diverts

Blocks may divert to:

- a block in the same file;
- a block in another file;
- `END`.

Unknown targets must be validation errors.

Runtime traversal of an unknown target must return an error and never silently end the scene.

### 5.9 Conditional Branches

Conditional branches gate a section of dialogue (lines, choices, effects, diverts, nested branches) on a condition expression.

```text
:if familiarity_gte(hazel, rhea, 3)
  > greet_warm_001 speaker=rhea
    You again. Good.
:else
  > greet_cold_001 speaker=rhea
    Do I know you?
```

Rules:

- `:if <condition>` opens a body of statements at the next indent level.
- An optional `:else` at the same indent attaches to the immediately preceding `:if`. Anything else at that indent terminates the conditional.
- No `:elif` in v1. Chained boolean conditions are a smell — they typically indicate that the dispatch is on an enum (use `:match`, see §5.9.1) or that the branches should be separate blocks. Adding `:elif` later is trivial if real authoring pain is reported; removing it once authors depend on it is not.
- Conditions reuse §6 grammar, semantics, and validation. The expression must be a boolean condition.
- Lines inside a branch must still carry stable IDs (§5.4.1) and are extracted to POT regardless of which branch evaluates true at runtime.
- Branches may be nested arbitrarily.

#### 5.9.1 Enum Match

Pattern matching is restricted, additive sugar over `:if` chains for the case where dispatch is on an enum. It is not general destructuring.

```text
:match thread_stage(rhea_job_response)
  :case tired
    > rhea_tired_001 speaker=rhea
      I'm exhausted. Let's keep it short.
  :case angry
    > rhea_angry_001 speaker=rhea
      Don't.
  :case fine
    > rhea_fine_001 speaker=rhea
      All right, what's up?
  :case _
    > rhea_default_001 speaker=rhea
      Hey.
```

Rules:

- The match scrutinee is a single condition-grammar query (§6.1) whose return type is declared in schema as an enum.
- Schema must declare the function as enum-returning. Boolean-returning queries are not valid scrutinees — use `:if` for those.
- `:case <variant>` arms must reference declared variants of that enum. Unknown variants are validation errors.
- `:case _` is the wildcard arm. It matches any variant not covered above and may appear at most once, as the last arm.
- Arms are evaluated top-to-bottom. The first matching arm runs; the rest are skipped.
- The compiler validates **exhaustiveness**: a match must either cover every declared variant of the enum or include `:case _`. Missing arms are an error, not a warning.
- Duplicate `:case <variant>` arms are validation errors.
- Each arm's body follows the same indentation rules as `:if` bodies and may contain lines, choices, effects, diverts, nested `:if`, or nested `:match`.
- Schema producers should mark a condition function as enum-returning in the
  canonical schema model. Adapter code should do this through typed bindings,
  and the generated manifest records the enum type for compiler and LSP use:

  ```rust
  schema
      .condition("thread_stage")
      .param::<ThreadId>("thread_id")
      .returns_enum::<ThreadStageKind>();
  ```

  ```json
  {
    "types": {
      "thread_stage_kind": {
        "kind": "enum",
        "values": ["fresh", "tired", "angry", "fine", "completed"]
      }
    },
    "conditions": {
      "thread_stage": {
        "params": [{ "name": "thread_id", "type": "registry:thread" }],
        "returns": "enum:thread_stage_kind"
      }
    }
  }
  ```

- Runtime evaluation extends `DialogueContext` with an enum-returning lookup or, equivalently, schema-generated bindings convert host return values to declared variants. Either path is acceptable; the runtime contract is that the scrutinee returns one declared variant of the schema enum or evaluation fails as a structured error.

The intent is narrow: schema-checked exhaustive dispatch on declared enum state. Writers who do not need it never see it; writers who do get compile-time coverage warnings when a new enum variant is added and an old `:match` was not updated.

### 5.10 Text Interpolation

Localisable text may interpolate named values supplied by the caller.

Placeholders use curly-brace syntax. Each placeholder is `{name}`, where `name` is a lowercase ASCII identifier (letters, digits, underscores; must start with a letter). Whitespace inside the braces is not permitted.

```text
> letters_001 speaker=narrator count=$letters_remaining
  You have {letters_remaining} letters.
```

Placeholders must be declared on the line header using `name=$value_name` attributes. Each attribute binds a placeholder name to a caller-supplied value at delivery time.

- An undeclared placeholder is a validation error.
- A declared attribute that is not referenced in the line's text is a warning; the caller must still provide the value.
- The `$` sigil distinguishes runtime-bound references from metadata symbols
  and literal strings (`portrait=flat`, `caption="Door closes"`). `$name`
  metadata values remain reserved until explicit runtime-bound metadata support
  is designed.

Interpolation rules:

- Placeholders are preserved verbatim through POT extraction; translators see `{name}` in `msgid` and must preserve the same names in `msgstr`.
- Translation validation must catch missing, renamed, or extra placeholders relative to the source.
- Placeholders may appear inside inline markup (`[slow]{name}[/slow]`) but must not span tag boundaries.
- The runtime substitutes placeholders after locale lookup, before delivering the line text on `DialogueLine.text`. `DialogueLine.source_text` retains the unsubstituted source for diagnostics and fallback.
- Literal `{` and `}` in source text must be escaped as `\{` and `\}`. Escapes are preserved through extraction; the runtime emits literal braces in `text` and `source_text`.

Caller-supplied values are threaded through `DialogueContext`. Missing values for declared attributes are a structured runtime error, not silent omission.

Determinism: same line id, same declared values, same locale → same delivered text.

### 5.11 Plural Lines

Lines whose text varies by count declare two source forms — singular and plural — using a continuation line prefixed with `|`. Selection between forms is governed by gettext/CLDR plural rules per locale, not by recite.

```text
> letters_001 speaker=narrator count=$letters_remaining
  You have one letter.
  | You have {letters_remaining} letters.
```

Plural line rules:

- The line header must include a `count=$<name>` attribute. The bound value must resolve to an integer.
- The singular form is the first body line. The plural form is the immediately following body line prefixed with `|`. Exactly two source forms are permitted; additional plural arms for translated locales live in `.po` (see §9.7).
- Both forms must be valid localisable text and may contain interpolation placeholders and inline markup.
- The placeholder bound by `count` may, but need not, appear in either form.
- POT extraction emits the line as a single entry with `msgid`, `msgid_plural`, and `msgstr[N]` arms (§9.7).
- The runtime resolves which form to deliver via the locale provider's plural lookup, supplying the count value. If the locale provider returns no translation, the runtime falls back to the source forms using English CLDR rules (`n == 1 → singular`, otherwise plural).
- Plurals compose with variants (§9.5): `id&formal` may be a plural line.

Multiline body prose is not permitted on plural lines in v1. If a plural line needs more than one paragraph, split it into separate adjacent lines.

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

Condition functions return either a boolean (the default, used by `:if` and choice `if` clauses) or a schema-declared enum variant (used by `:match` scrutinees, see §5.9.1). An enum-returning function declares its return type in the canonical schema model; the dialogue context exposes it through the same `evaluate_condition` path or a sibling enum-returning lookup, depending on adapter ergonomics.

### 6.3 Schema Validation

All condition functions must be declared in schema.

Validation must reject:

- unknown condition functions;
- wrong arity;
- wrong argument types;
- invalid literal values where a schema defines an enum or registry;
- non-boolean condition expressions in `:if` and choice `if` clauses;
- non-enum-returning scrutinees in `:match`;
- `:match` arms that reference variants not declared in the scrutinee's enum;
- non-exhaustive `:match` (no `:case _` and at least one declared variant uncovered);
- duplicate `:case <variant>` arms in a single `:match`.

## 7. Effects

### 7.1 Effect Model

The previous term “mutation” is too narrow. The production system should use **effects**.

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

- “Here, I’ll mark it on your map.”
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

## 8. Runtime

### 8.1 Core Requirements

The runtime must:

- be implemented in Rust;
- have no engine dependencies;
- be deterministic;
- be side-effect free;
- expose serialisable session state;
- support save/load while waiting on a blocking effect;
- support programmatic tests without engine runtime;
- return structured errors instead of panicking.

### 8.2 Runtime API

Illustrative API:

```rust
pub fn start_scene(
    asset: &CompiledDialogue,
    block: Option<&str>,
    locale: LocaleId,
) -> Result<DialogueSession, DialogueError>;

pub fn next(
    session: &mut DialogueSession,
    context: &dyn DialogueContext,
    locale: &dyn LocaleProvider,
) -> Result<DialogueEvent, DialogueError>;

pub fn choose(
    session: &mut DialogueSession,
    choice_id: ChoiceId,
    context: &dyn DialogueContext,
    locale: &dyn LocaleProvider,
) -> Result<DialogueEvent, DialogueError>;

pub fn acknowledge_effect(
    session: &mut DialogueSession,
    effect_id: EffectRequestId,
    ack: EffectAck,
) -> Result<(), DialogueError>;

pub fn end_scene(
    session: DialogueSession,
) -> Result<Vec<DialogueEffectRequest>, DialogueError>;
```

The concrete API may differ, but the semantics must hold.

### 8.3 Event Model

The event model must represent prompts directly.

```rust
pub enum DialogueEvent {
    Line(DialogueLine),
    Prompt {
        line: Option<DialogueLine>,
        choices: Vec<DialogueChoice>,
    },
    Effect(DialogueEffectRequest),
    End,
}
```

`Line` should be used when no choices are present.

`Prompt` should be used when choices are present, with or without prompt text.

`Effect` should be used for immediate and blocking effects. Deferred effects are collected and may optionally also be observable in trace/debug mode.

### 8.4 Line Model

```rust
pub struct DialogueLine {
    pub id: LineId,
    pub source_text: String,
    pub text: String,
    pub speaker: Option<SpeakerId>,
    pub metadata: Vec<MetadataEntry>,
    pub pending_deferred_effects: Vec<DialogueEffectRequest>,
}
```

`text` is the resolved localized text.

`source_text` is retained for diagnostics, fallback, tests, and gettext semantics.

### 8.5 Choice Model

```rust
pub struct DialogueChoice {
    pub id: ChoiceId,
    pub source_text: String,
    pub text: String,
    pub metadata: Vec<MetadataEntry>,
    pub is_available: bool,
    pub unavailable_reason: Option<String>,
    pub echo: ChoiceEchoMode,
}

pub enum ChoiceEchoMode {
    None,
    SelectedText,
    ExplicitLine(LineId),
}
```

Selection should prefer `ChoiceId` over index. Adapters may expose index-based APIs for engine ergonomics, but the core runtime should preserve stable choice identity.

### 8.6 Session State

`DialogueSession` must serialise enough information to resume exactly:

- compiled asset identity/version;
- current block;
- statement pointer;
- call/divert stack if applicable;
- collected deferred effects;
- pending blocking effect;
- previous prompt choices;
- locale;
- deterministic trace counters;
- selected choice history.

The session must not serialise game state.

#### Save/load while waiting on a blocking effect

If the session is saved while a blocking effect is pending, on resume the runtime re-emits the same effect with the same `EffectRequestId`. The runtime makes no claim about whether the game-side operation was partially executed before the save. The game decides whether to fast-forward, replay, or otherwise reconcile and then calls `acknowledge_effect`. The runtime contract is purely: same ID re-emitted, same acknowledgement expected.

### 8.7 Error Handling

Runtime errors must be structured.

Examples:

- unknown block;
- invalid choice;
- unavailable choice selected;
- missing blocking effect acknowledgement;
- wrong acknowledgement ID;
- malformed compiled asset;
- condition evaluation failure;
- locale provider failure;
- unsupported compiled format version.

The runtime must not panic on malformed project content.

## 9. Localisation

Recite has two localisation domains:

- dialogue content localisation, owned by compiled project content and runtime locale providers;
- Recite-owned tool UI text, owned by CLI/TUI catalog resources.

Dialogue content uses the gettext/POT workflow in this section. CLI/TUI helper text, labels, footer hints, status messages, and Recite-owned human error text use Fluent resources inside `recite-cli` so UI strings can carry variables, future plural/select rules, and deterministic fallback behavior. These catalogs must not be used as a substitute for translated dialogue text. Future content preview work must load explicit dialogue catalogs through the runtime/provider path rather than mixing dialogue content into the tool UI catalog.

### 9.1 Requirements

The project must support gettext/POT workflows as a first-class path.

Localisable strings:

- line text;
- choice text;
- speaker display names;
- optional project-defined localisable metadata values.

Each localisable string must have:

- stable ID;
- source text;
- source location;
- translator comments;
- block/scene context where available.

### 9.2 POT Extraction

The CLI must emit POT files.

For dialogue lines and choices:

```po
#. file: Dialogue/hazel_rhea/small_talk.recite
#. block: small_talk_start
#. speaker: rhea
msgctxt "hazel_rhea.small_talk.001"
msgid "Oh, hey! Didn't expect to see you here."
msgstr ""
```

Speaker names must be extracted separately:

```po
msgctxt "dialogue_speaker:rhea"
msgid "Rhea"
msgstr ""
```

### 9.3 Locale Provider

The runtime locale provider must receive both stable ID and source text.

```rust
pub trait LocaleProvider {
    fn lookup(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Option<String>;
}
```

This supports gettext-style lookup where `msgctxt` is the stable ID and `msgid` is the source text. The `variant` parameter carries the explicit selection from the caller (see §9.5).

### 9.4 Fallback

If no translation is found for the requested locale, the locale provider must attempt broader locales via BCP-47 region truncation before falling back to source text. Example: a lookup for `pt-BR` falls back to `pt`, then to `msgid`.

The chain is the responsibility of the locale provider implementation. The spec requires:

- The terminal fallback is always the source text (`msgid`, or for plural lines `msgid` / `msgid_plural` selected by English CLDR rules).
- Each step in the chain — including the terminal source fallback — must be observable in diagnostics or trace mode so missing translations and unintended fallbacks can be caught in tests.
- Fallback resolution must be deterministic for a given `(id, source, locale, variant, count)` tuple.

The runtime never invents broader locales beyond BCP-47 truncation. Cross-locale fallback (e.g., `nb` → `nn`) is the caller's job, configured outside the provider.

### 9.5 Grammatical Variants

IDs may support variant suffixes:

```text
hazel_rhea.small_talk.001&formal
hazel_rhea.small_talk.001
```

Lookup priority:

1. `id&suffix`;
2. `id`;
3. source text.

Variant selection must be explicit and deterministic. The caller selects a variant either via a session-level setter (`session.set_variant("formal")`) or via a per-call override threaded through `next` / `choose`. The runtime never infers a variant. Lookup priority remains `id&variant` → `id` → source text.

Variants are recite's mechanism for grammatical or register selection (formal/informal, masculine/feminine, polite/casual). They deliberately do not overload `msgctxt` semantically; `msgctxt` carries the full `id&variant` string and remains the stable lookup key. Counts (plural forms, §9.7) are a separate axis resolved by CLDR rules, not by variant lookup.

### 9.6 Inline Markup in Translation

Translation validation must be able to detect:

- missing required inline tags;
- invalid new tags;
- unbalanced tags;
- changed tag attributes where schema forbids changes.

The runtime should not parse translated markup unless configured to validate in debug/test mode.

### 9.7 Plural Forms

Plural lines (§5.11) extract to standard gettext plural entries:

```po
#. file: Dialogue/town/inventory.recite
#. block: post_courier
#. speaker: narrator
msgctxt "town.inventory.letters_001"
msgid "You have one letter."
msgid_plural "You have {letters_remaining} letters."
msgstr[0] ""
msgstr[1] ""
```

The number of `msgstr[N]` arms per locale is determined by the locale's `nplurals` header in the `.po` file. Translators use standard po editors (poedit, weblate, crowdin) without recite-specific tooling.

The locale provider must expose a plural lookup alongside the singular one:

```rust
pub trait LocaleProvider {
    fn lookup(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Option<String>;

    fn lookup_plural(
        &self,
        id: &str,
        source_singular: &str,
        source_plural: &str,
        count: i64,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Option<String>;
}
```

Lookup priority for plurals mirrors §9.5:

1. `id&variant` plural arm matching the locale's CLDR rule for `count`;
2. `id` plural arm matching the locale's CLDR rule for `count`;
3. source singular (if `count == 1`) or source plural (otherwise).

The fallback chain in §9.4 applies between steps 1 and 2 and between step 2 and step 3.

Plural translation validation must additionally detect:

- missing required `msgstr[N]` arms for the locale's declared `nplurals`;
- placeholder mismatch between any `msgstr[N]` and the corresponding source form;
- locales missing the `Plural-Forms` header in their `.po`.

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
- project-level content registries.

### 10.2 Schema Model and Producers

The schema has three separate surfaces:

1. a canonical Rust model in `recite-core`;
2. a generated schema manifest consumed by `recite-compiler`, `recite-cli`, and
   `recite-lsp`;
3. producer-specific authoring surfaces that create the manifest.

The preferred producer is adapter or game code, not hand-authored schema
configuration. Game projects already define typed handles, effect handlers,
condition queries, enum state, speakers, and registries near their adapter
code. Recite should reuse that existing typed surface instead of asking
developers to maintain a parallel string-based schema file.

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

The host-agnostic export contract for adapter-produced manifests, including
resource-backed metadata domains, snapshot determinism, provenance, and
stale-schema checks, lives in `docs/engine-adapter-contract.md` §7. This section
defines the canonical schema model that those producers must lower into.

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

JSON Schema is not the authority for Recite semantics. After document-shape
validation, Recite must lower the manifest into the canonical Rust model and
run semantic validation there. Semantic validation owns duplicate definitions,
unknown type references, registry/value checks, condition return compatibility,
effect arity/type checks, metadata target policy, markup policy, diagnostics,
and deterministic fingerprinting.

The Rust schema model should live in `recite-core::schema` and include:

- `ProjectSchema`;
- `SchemaTypeDefinition`, including enum definitions;
- `SchemaTypeRef`, covering built-in scalar types, speaker IDs, enum types, and
  registry-backed IDs, and the metadata-only `symbol` scalar;
- `ConditionDefinition`, including typed parameters and optional enum return
  type;
- `EffectDefinition`, including typed parameters and supported modes;
- `MetadataDefinition`, including targets, type, repeatability, and optional
  range constraints, and optional domain reference;
- `MetadataDomainDefinition`, including flat value sets, contextual value
  selectors, and optional origin/fingerprint metadata for adapter-produced
  manifests;
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
      "origin": "data/content/dialogue-sound-effects.toml"
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
      "returns": "bool"
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
- inline markup tag names;
- speaker IDs;
- registry references and registry values;
- schema manifest fields when the manifest itself is malformed.

When a schema manifest is generated from adapter code, the manifest may include
producer origin metadata for definitions, metadata domains, metadata-domain
contexts, metadata-domain values, and registry values. Recite diagnostics may
surface that origin as related context, but dialogue-source diagnostics must
remain valid even when producer origins are unavailable.

## 11. Scene Manifest

### 11.1 Purpose

The project should include an optional scene manifest to connect dialogue assets to game concepts without embedding game-specific data in the dialogue DSL.

This mirrors the current need for scene IDs, presentation modes, participants, and cinematic paths.

### 11.2 Example

```toml
[project]
content_set = "base"
version = "0.1.0"

[[scenes]]
id = "scene.small-talk"
presentation = "portrait_dialogue"
asset = "Dialogue/Compiled/dialogue.recitec"
block = "small_talk_start"
participants = ["hazel", "rhea"]

[[scenes]]
id = "scene.heart-to-heart"
presentation = "cinematic_cutscene"
asset = "Dialogue/Compiled/dialogue.recitec"
block = "heart_to_heart_start"
participants = ["hazel", "rhea"]
cinematic_scene = "Scenes/Dialogue/Cutscenes/HeartToHeartCutscene.tscn"
```

### 11.3 Manifest Validation

Validation must check:

- duplicate scene IDs;
- missing compiled assets;
- missing source assets where configured;
- unknown start blocks;
- missing participants;
- unknown participants where a speaker/actor registry exists;
- presentation mode requirements;
- duplicate scene/block pairs if project policy disallows them;
- stale compiled assets.

## 12. Compiler

### 12.1 Compilation

The compiler must:

- parse dialogue sources;
- resolve imports/includes;
- validate block references;
- validate IDs;
- validate conditions;
- validate effects;
- validate metadata;
- validate markup;
- emit compiled assets;
- embed source fingerprints;
- embed schema fingerprint;
- embed compiler version;
- preserve source map information for diagnostics.

### 12.2 Compiled Format

The v0 compiled asset format is a deterministic MessagePack document with a
decoded compact JSON inspection form for fixtures, debugging, and CLI tooling.
The MessagePack bytes are the runtime-facing asset; the JSON form is
non-authoritative and must be produced from the same structured model.

v0 uses:

- `format_version = 0`;
- `compiler_compatibility_version = 0`;
- MessagePack as the primary `.recitec` encoding;
- compact JSON as a decoded inspection encoding, not as the shipped runtime
  asset;
- BLAKE3 as the default content fingerprint algorithm.

The compiler must serialize deterministic tables, not parser-shaped object
graphs. The v0 wire contract must preserve row order explicitly and must not
depend on unordered map iteration. Lookup data is encoded as sorted tables keyed
by stable IDs. Repeated metadata entries remain ordered rows, even when keys
repeat. Field ordering, table ordering, string encoding, numeric representation,
and fingerprint inputs must be stable across repeated compiles of identical
validated input.

#### v0 wire shape

Runtime assets encode all compound values as fixed-length MessagePack arrays,
not maps. The decoded compact JSON inspection form renders the same arrays as
objects with the field names below. JSON field names are for humans and tests;
MessagePack array positions are authoritative.

Scalar wire rules:

- IDs and paths encode as UTF-8 strings.
- Index newtypes encode as unsigned 32-bit integers.
- Ranges encode as `[start, len]`, where `start` is the table index's `u32`
  value and `len` is a `u32` count.
- Optional values encode as MessagePack nil or the present value.
- Fingerprints encode as `[algorithm, digest]`, where `digest` is binary bytes.
- Source spans encode as `[file, start_line, start_column, end_line,
  end_column]`; `end_line` and `end_column` are nil for point spans.
- `Value` encodes as `[tag, payload]`, with tags `0 = scalar` and `1 = array`.
- `ScalarValue` tags are `0 = string`, `1 = integer`, `2 = float`, and
  `3 = boolean`.

Top-level and row arrays use this field order:

- `CompiledDialogue`: `[header, default_block, sources, blocks, statements, match_arms,
  lines, choices, speakers, metadata, effects, source_maps, block_lookup,
  line_lookup, choice_lookup]`.
- `CompiledAssetHeader`: `[format_version, compiler_compatibility_version,
  primary_encoding, inspection_encoding, compiler_version, asset_id,
  source_map_id, schema_fingerprint]`.
- `CompiledSourceFile`: `[path, fingerprint]`.
- `CompiledBlock`: `[id, source_file, statements, metadata, default_speaker,
  source_map]`.
- `CompiledStatement`: `[kind, source_map]`.
- `CompiledMatchArm`: `[pattern, statements, source_map]`.
- `CompiledLine`: `[id, source_text, speaker, metadata, source_map]`.
- `CompiledChoice`: `[id, source_text, metadata, condition, target, echo,
  source_map]`.
- `CompiledSpeaker`: `[id]`.
- `CompiledMetadataEntry`: `[key, value, source_map]`.
- `CompiledEffect`: `[id, mode, function, args, source_map]`.
- `CompiledSourceMapEntry`: `[source_file, span]`.
- Lookup entries: `[id, index]`, sorted strictly ascending by ID.

Enum-like values encode as `[tag, payload]` unless the variant has no payload,
in which case the payload is nil. v0 tags are:

- asset encoding: `0 = MessagePack`;
- inspection encoding: `0 = CompactJson`;
- schema fingerprint: `0 = fingerprint`, `1 = no_schema`;
- statement kind: `0 = line`, `1 = prompt`, `2 = divert`, `3 = if`,
  `4 = match`, `5 = effect`, `6 = end`;
- match pattern: `0 = variant`, `1 = wildcard`;
- divert target: `0 = block`, `1 = end`;
- choice echo: `0 = none`, `1 = selected_text`, `2 = explicit_line`;
- effect mode: `0 = deferred`, `1 = immediate`, `2 = blocking`;
- condition expression: `0 = call`, `1 = and`, `2 = or`, `3 = not`;
- argument: `0 = identifier`, `1 = value`.

v0 fixed array arity is not append-compatible. The v0 shape may still be
corrected before the runtime reader and compatibility gate are implemented, but
once a v0 reader ships, field additions, removals, reordering, tag changes, or
semantic changes require a `format_version` or `compiler_compatibility_version`
change. A v0 reader must reject unexpected array lengths, unknown tags, invalid
indexes, malformed lookup order, and algorithm-specific fingerprint length
mismatches as malformed compiled assets.

Compiled assets must include:

- format version;
- compiler compatibility version;
- compiler version;
- primary encoding and inspection encoding identifiers;
- asset identity and source-map identity;
- source file table;
- source fingerprints;
- schema fingerprint, or an explicit no-schema marker;
- default block index;
- block table;
- statement table;
- match arm table;
- line table;
- choice table;
- speaker table;
- metadata table;
- effect table;
- source map table;
- sorted lookup tables for block IDs, line IDs, and choice IDs.

The runtime-facing contract must exclude rowan syntax nodes, parser recovery
state, malformed source state, comments that are not part of runtime semantics,
and traversal over the `recite-core` source AST. Syntax trees and source AST
values are compiler and tooling inputs only. Runtime traversal consumes compiled
tables, source maps, fingerprints, and compact lookup indexes.

Custom binary, FlatBuffers, Cap'n Proto, bincode, postcard, CBOR, and other
encodings remain possible future versions if benchmark evidence or adapter
requirements justify them. They must not be introduced as v0 alternatives after
assets exist without a format or compatibility version change.

### 12.3 Freshness

The compiler must embed enough data for tooling to detect stale compiled assets.

`recite check-fresh` must compare:

- current source fingerprints;
- current schema fingerprint;
- current compiler compatibility version;
- compiled asset embedded source fingerprints;
- compiled asset embedded schema fingerprint or no-schema marker;
- compiled asset embedded compiler compatibility version.

The v0 freshness comparison is content-based. Source and schema fingerprints are
algorithm-tagged binary digest values; the initial algorithm is BLAKE3. The
MessagePack asset stores digest bytes directly. The compact JSON inspection form
may render those bytes as stable lowercase hexadecimal text. A compiler version
change alone does not require recompilation unless the compiler compatibility
version changes or the writer changes any runtime-facing semantics.

## 13. CLI

The CLI is a core product surface.

Required commands:

```text
recite compile <path-or-project>
recite validate <path-or-project>
recite validate-project <project-root>
recite extract <path-or-project>
recite check-ids <path-or-project>
recite check-fresh <project-root>
recite check-markup <path-or-project>
recite check-metadata <path-or-project> --schema <schema>
recite watch <project-root>
recite run <asset> --block <block> --fixture <fixture>
recite trace <asset> --block <block> --fixture <fixture>
recite play <asset> --block <block> [--ui auto|tui|plain] [--keymap standard|vim]
  [--dialogue-locale <locale>] [--dialogue-catalog <locale=path>]...
```

`recite play` is an interactive REPL for writers (Milestone 5.5). Future commands include `recite generate-bindings --schema <schema> --lang <lang>` once the schema and adapter contracts stabilise; it is not part of the v1 CLI surface.

### 13.1 `compile`

Compiles source dialogue into a compiled asset.

Must fail on validation errors unless `--allow-warnings` only warnings are present.

### 13.2 `validate`

Validates dialogue source without writing compiled output.

Must report all recoverable diagnostics.

### 13.3 `extract`

Emits POT files.

Options:

- output path;
- domain split;
- include/exclude speaker names;
- include/exclude metadata localisable fields.

### 13.4 `check-ids`

Reports:

- missing IDs;
- duplicate IDs;
- IDs that do not match project naming policy;
- IDs present in translations but absent from source;
- source strings whose ID changed unexpectedly where history data is available.

### 13.5 `run`

Runs a dialogue scene headlessly with fixture data.

Useful for tests, CI, and writer review.

Must be able to:

- auto-select choices by ID or index;
- auto-acknowledge immediate/blocking effects;
- emit transcript;
- emit effect list;
- emit condition query trace.

`run` may preview translated dialogue content only when the fixture opts in with
`[dialogue].locale`. Dialogue catalog paths in fixtures are resolved relative to
the fixture file directory. Without `[dialogue].locale`, line and choice output
must remain source text. Catalogs without a dialogue locale are an error.

### 13.6 `trace`

Produces a deterministic execution trace including:

- lines;
- prompts;
- choices;
- conditions evaluated;
- condition results;
- effects emitted;
- blocking acknowledgements;
- final deferred effects.

Structured trace field names and machine values are stable English identifiers.
When fixture dialogue preview is configured, trace output includes the selected
dialogue locale and fallback chain as metadata, while line and choice records
keep both `source_text` and preview `text` fields so terminal source fallback
remains testable.

### 13.7 `play`

Interactive REPL for writers. Loads a compiled asset, starts a scene, prints or renders lines and prompts, accepts choice selections by ID or index, asks for condition results as `y`/`n`, and requires explicit acknowledgement of blocking effects. Useful for fast authoring iteration without standing up a game.

`play` is a live authoring surface, distinct from the deterministic fixture runner. `run` and `trace` remain scriptable commands driven by fixture data; `play` is allowed to prompt the author and maintain an interactive transcript.

The default `--ui auto` mode should use a TUI when stdin and stdout are interactive terminals, and should fall back to the line-oriented plain mode for pipes, CI, and accessibility tooling. `--ui tui` must fail clearly when no interactive terminal is available and suggest `--ui plain`. `--ui plain` must preserve the same runtime event flow as the TUI with line-oriented prompts and responses.

Interactive UI preferences are user preferences, not project content. The CLI may read `$RECITE_CONFIG`, then `$XDG_CONFIG_HOME/recite/config.toml`, then `~/.config/recite/config.toml`. Missing config uses defaults. UI preferences must not be stored in `recite.project.toml`. Malformed UI config must not affect `run` or `trace`.

Initial UI config:

```toml
[ui]
locale = "en-US"        # BCP-47 locale, or "system"
keymap = "standard"      # "standard" or "vim"
key_hints = "contextual" # "contextual", "compact", or "hidden"

[play]
show_unavailable_choices = true
```

The UI locale controls only Recite-owned CLI/TUI text: pane titles, transcript labels, footer hints, prompts, status messages, invalid input text, blocking-effect acknowledgement labels, and human CLI errors owned by `recite-cli`. It does not control dialogue line or choice translation for `play`, `run`, or `trace`; those remain runtime/provider concerns (§9). There is no `--ui-locale` flag.

Dialogue content preview for `play` is separately opt in:

```text
recite play <asset> --block <block> --dialogue-locale fr-FR \
  --dialogue-catalog fr-FR=locale/fr-FR.po
```

`--dialogue-catalog` is repeatable and accepts `LOCALE=PATH`. Catalog paths on
the `play` command line are resolved relative to the current working directory
unless absolute. Passing a dialogue catalog without `--dialogue-locale` is an
error. Missing or empty catalog translations fall back to source text through
the runtime locale-provider path; Recite-owned UI text remains on the Fluent UI
catalog path.

Locale fallback for CLI/TUI text is deterministic: requested locale, then language-only locale, then `en-US`. Missing or malformed non-default catalogs fall back to `en-US`. The default `en-US` catalog is a test-gated resource.

The default keymap is `standard`: arrows move choices, printable keys enter a choice ID/index, Enter submits typed input or the highlighted choice, and Ctrl-C/Esc/`:q`/`:quit` quit cleanly. Vim mode is opt-in: choices start in normal mode, `j`/`k` and arrows move, `i` enters text input, `:` opens command mode, and Esc leaves insert/command/help before quitting at the root prompt.

The TUI should include:

- a transcript pane for lines, selected choices, effects, acknowledgements, and end state;
- a current prompt pane with visible choice indexes and stable choice IDs;
- a status/footer area showing the compiled asset, block, and available controls;
- a condition prompt accepting `y`/`n`;
- a blocking-effect panel showing mode, runtime effect ID, function, args, and Enter/`ack` acknowledgement.

`play` must not execute game-side effects. Immediate and blocking effects remain typed runtime requests emitted to the authoring surface.

`play` is part of Milestone 5.5 (Authoring Polish) and is not on the v1 acceptance gate, but the runtime API must accommodate it.

### 13.8 `watch`

Authoring build loop for source, schema, and project changes:

```text
recite watch <project-root>
```

`watch` observes the project manifest, dialogue source files, generated schema
manifest, and other compile inputs. On change, it validates the project and
rebuilds compiled assets using the same deterministic whole-project compiler as
`compile`. It should reuse `check-fresh` fingerprint semantics so generated
assets can be compared against current source and schema without editor- or
engine-specific state.

The expected authoring loop is:

1. edit source or schema inputs;
2. LSP reports live diagnostics;
3. on save, LSP/editor code actions may insert missing stable IDs without
   rewriting existing IDs;
4. `recite watch` validates and rebuilds compiled assets;
5. the engine adapter imports or refreshes those assets and restarts the scene
   or applies the adapter's documented active-session policy.

`watch` must not imply mid-session patch reload for v1. It is a fast rebuild
surface for authoring, CI-adjacent local checks, and editor/engine integration.

### 13.9 Future: `generate-bindings`

Deferred past v1. Will generate typed host-language bindings (condition stubs, effect records/enums, runtime conversions, test helpers, optional engine event/signal wrappers) from schema once the schema and adapter contracts stabilise.

## 14. LSP

The LSP must be excellent enough that text authoring feels safe.

Required capabilities:

- syntax diagnostics;
- schema diagnostics;
- unknown block diagnostics;
- duplicate ID diagnostics;
- missing ID diagnostics;
- unknown speaker diagnostics;
- unknown metadata key diagnostics;
- invalid metadata value diagnostics;
- unknown condition/effect function diagnostics;
- wrong arity/type diagnostics;
- inline markup diagnostics;
- completion for block references;
- completion for speaker IDs;
- completion for metadata keys;
- completion for metadata values where schema provides registries;
- completion for condition/effect functions;
- hover documentation from schema;
- go-to block definition;
- find references for block IDs;
- rename block;
- code action to add missing ID;
- code action to create block stub;
- code action to add schema entry for unknown metadata/effect/condition where appropriate.

Metadata value completions and diagnostics must use the same domain resolution
rules as compiler validation (§10.2). In particular, contextual metadata
domains resolve `field:speaker` and `metadata:<key>` selectors the same way in
the LSP as in the compiler, including the schema-declared missing-context
policy. The LSP must not invent broader fallback behavior for convenience; if
the manifest says the result is diagnostic, empty, or fallback to a named flat
domain, editor completions and diagnostics must reflect that same result.

Nice-to-have:

- semantic tokens;
- inlay hints for parameter names;
- condition preview with fixture data;
- dialogue flow outline;
- graph preview export.

## 15. Editor Support

### 15.1 VS Code

The VS Code extension must provide:

- syntax highlighting;
- LSP client wiring;
- commands for compile/validate/extract/watch;
- problem matcher integration;
- block outline;
- quick run/trace command;
- optional graph preview.

### 15.2 Neovim

Neovim support must include:

- documented LSP setup;
- Tree-sitter grammar if feasible;
- filetype detection;
- command examples for validation and extraction.

### 15.3 Visual Editor

The visual editor should be built after the source format, compiler, runtime, and LSP are stable enough to avoid designing the project around a premature UI.

The visual editor should:

- operate on the same source or a lossless structured representation;
- never lock users out of text workflows;
- show block graphs;
- edit lines, choices, metadata, conditions, and effects;
- surface validation diagnostics;
- preview localized text;
- preview effect traces;
- integrate with schema completions.

The visual editor is part of the long-term value proposition, but v1 should prove the text-first deterministic workflow first.

## 16. Engine Adapters

The normative adapter contract lives in
`docs/engine-adapter-contract.md`. This section records the product-level
requirements that the contract expands.

Schema-manifest export, including resource-backed metadata-domain snapshots and
stale-schema checks, is part of that adapter contract (§7). Engine-specific
sections here must not require Recite compiler, CLI, LSP, or runtime code to
scan host assets or execute game code.

### 16.1 Goals

The core runtime is engine-independent. Engine adapters are integration layers that make Recite feel native in a host engine without changing the dialogue contract.

Adapters must:

- load compiled dialogue assets through the host's asset pipeline where possible;
- export or consume generated schema manifests through the shared contract in
  `docs/engine-adapter-contract.md` §7;
- define how compiled assets are imported or refreshed during authoring;
- store active dialogue session state in host-native resources, nodes, components, or services;
- expose dialogue lines, prompts, effects, endings, and errors through host-native events, messages, signals, or callbacks;
- preserve choice selection by stable `ChoiceId`;
- preserve blocking-effect acknowledgement semantics;
- let users drive dialogue UI and presentation however they want;
- avoid requiring dialogue files to call directly into engine scripts.

### 16.2 Adapter API Shape

Every adapter should expose host-native equivalents of these operations:

- start dialogue from a compiled asset, optional block, and locale;
- select a dialogue choice by `ChoiceId`;
- acknowledge a blocking effect by `EffectRequestId`;
- observe structured dialogue output:
  - line;
  - prompt with optional line and choices;
  - effect request;
  - end with deferred effects;
  - structured error.

The concrete API should feel idiomatic for the host engine. A Bevy adapter may
use resources and events/messages. A Godot adapter may use nodes, resources,
C# APIs, and signals. A Unity adapter may use C# packages, imported assets,
events, and editor import hooks. The semantics must stay equivalent.

### 16.3 Active Sessions

Initial adapter scope may maintain one active dialogue session per declared
adapter owner. Each adapter must document whether that owner is a singleton
service/resource, node, component, scene service, or equivalent host-native
object.

Attempting to start a second scene on the same owner while one is active must
emit an error, not panic.

Each adapter must document and test what happens when a compiled asset changes
while a session is active. The shared policy names are
`reject_refresh_until_session_ends`, `reload_for_next_session_only`, and
`restart_required`. Silent mid-session mutation is not acceptable because it can
break deterministic traversal, save/load identity, previous prompt choice
validation, and pending blocking-effect semantics.

Adapters should make the edit-source -> LSP diagnostics -> on-save IDs ->
`recite watch` rebuild -> engine import/refresh -> restart scene loop practical
for their host engine. A richer mid-session patch reload can be explored after
v1, but it is not required for the serious v1 gate.

Future versions may support multiple sessions keyed by entity/session ID.

### 16.4 Conditions and Effects

Adapters should support:

- registering condition handlers through the host's normal extension points;
- emitting generic effect requests;
- optional generated typed effect events, signals, or records from schema;
- test fixtures independent of the host engine runtime where possible.

Conditions must remain pure queries. Effects must remain typed requests emitted to the game. Adapter convenience APIs must not move game-side mutation into the Recite runtime.

### 16.5 Initial Adapter Targets

Godot, Bevy, and Unity are v1-facing adapter targets. This is a settled product
scope decision, not a ranking of engine value. The serious v1 gate requires all
three adapters to be production-quality and to pass the engine-independent
conformance coverage in `docs/engine-adapter-contract.md` §13, including
contract-aligned asset refresh and active-session behavior.

No adapter may weaken the engine-independent core contract.

Unreal and GameMaker remain post-v1 evaluation targets.

### 16.6 Shared Conformance Artifacts

Adapter conformance scenarios are published in
`fixtures/adapter-conformance/v1/` with:

- a versioned scenario manifest;
- a manifest schema;
- a stable operation/result schema.

Those fixtures are adapter-consumable contracts, not private Rust-only test
support. They define operation sequencing, capability gates, changed-asset
policy declarations, and expected structured outcomes/errors.

`.recite` source fixtures still belong under `fixtures/recite/`; conformance
manifests reference those sources instead of duplicating parser/compiler/runtime
snapshot expectations.

## 17. Testing

### 17.1 Core Test Philosophy

The project must make dialogue easy to test without a game engine.

Supported test patterns:

- transcript snapshot;
- effect snapshot;
- condition trace snapshot;
- localization fallback snapshot;
- unavailable choice assertion;
- blocking effect pause/resume assertion;
- save/load mid-scene assertion;
- save/load while waiting on blocking effect assertion;
- adapter conformance operation/result traces.

### 17.2 Example Rust Test

```rust
let asset = compile_fixture("small_talk.recite");
let mut session = start_scene(&asset, Some("small_talk_start"), locale!("en-GB"))?;
let mut fixture = DialogueFixture::default()
    .with_condition("trust_gte(hazel, rhea, 3)", true)
    .auto_ack_effects();

let trace = run_to_end(&mut session, &fixture)?;

assert_snapshot!(trace.transcript);
assert_eq!(
    trace.deferred_effects,
    vec![
        effect!("advance_thread", "rhea_job_response", "fine"),
    ],
);
```

### 17.3 Fixture Format

The CLI should support a fixture format for headless runs:

```toml
[conditions]
"trust_gte(hazel, rhea, 3)" = true

[choices]
small_talk_start = "small_talk_start_WPUQ"

[effects]
auto_ack_blocking = true

[dialogue]
locale = "fr-FR"

[dialogue.catalogs]
"fr-FR" = ["locale/fr-FR.po"]
fr = ["locale/fr.po"]
```

Condition keys use bare identifiers inside the call, matching the dialogue DSL. The TOML key is quoted only because TOML requires it for keys containing parentheses; the inner argument list does not requote identifiers.

The `[dialogue]` fixture table is optional. When present, `locale` selects the
runtime dialogue locale for preview, and `catalogs` maps locale IDs to gettext
PO files. Catalog entries use singular gettext records with `msgctxt` as the
stable line or choice ID, `msgid` as source text, and `msgstr` as translated
text. Variant-specific entries may use `id&variant` contexts and should fall
back to `id` before source text.

### 17.4 Adapter Conformance Fixtures

The normative adapter conformance fixture contract is in
`docs/engine-adapter-contract.md` §13 and is backed by
`fixtures/adapter-conformance/v1/`.

Testing policy:

- mandatory scenarios cover every stable adapter error category from contract
  §12;
- source/schema freshness scenarios are capability-gated by adapter-declared
  source/schema import visibility;
- compiled-asset compatibility and save/load identity scenarios are mandatory;
- scenarios that require concrete adapters remain in the manifest as
  `adapter_runner_required` with operation/result shape and runner notes;
- reference-driver checks must fail when §12 categories drift from fixture
  schema tables.

## 18. Diagnostics

Diagnostics must be stable and testable.

Each diagnostic should include:

- code;
- severity;
- message;
- file;
- line;
- column;
- optional end line/end column;
- optional related spans;
- optional help text.

Diagnostic codes should be namespaced, for example:

- `RECITE_PARSE001`;
- `RECITE_ID001`;
- `RECITE_SCHEMA001`;
- `RECITE_EFFECT001`;
- `RECITE_META001`;
- `RECITE_MARKUP001`;
- `RECITE_PROJECT001`;
- `RECITE_FRESH001`.

## 19. Performance and Benchmarks

Performance is part of the product contract. Recite is intended for games that validate dialogue in CI, run headless tests frequently, and may load large narrative projects during editor workflows. Benchmarks must cover authoring, compilation, runtime traversal, localization, and adapter overhead.

All numeric budgets in this section are **aspirational targets, not contracts**, until a baseline exists from realistic fixtures. They will be re-evaluated against measured numbers; failing to hit a target is a benchmark report, not an acceptance failure. Benchmarks are tracked under Milestone 6 and are not part of the §23 v1 acceptance gate.

#### Authoring refresh layers

The compiler is whole-project for v1. Recite has several distinct refresh
layers:

- LSP live refresh: the editor-facing index re-parses edited files, refreshes
  diagnostics, and resolves cross-file references incrementally.
- Watch/build refresh: `recite watch <project-root>` observes source, schema,
  and project inputs, then re-runs deterministic whole-project validation and
  asset compilation.
- Adapter import refresh: each engine adapter defines how rebuilt compiled
  assets enter the host asset pipeline and what authors do with active sessions.
- Mid-session patch reload: changing the compiled asset underneath an already
  running session without restarting it is a non-v1 feature.

The v1 requirement is a competitive edit/save/rebuild/import/restart authoring
loop, not arbitrary runtime patching of active dialogue sessions.

### 19.1 Benchmark Harness

The workspace must include repeatable benchmarks using Criterion or an equivalent Rust benchmark framework.

Benchmarks must be runnable through:

```text
cargo bench
recite bench <fixture-or-project>
```

The CLI benchmark command should support:

- JSON output for CI comparison;
- Markdown summary output for release notes;
- baseline comparison against a checked-in or downloaded benchmark snapshot;
- filtering by benchmark group;
- fixture scale selection.

### 19.2 Benchmark Fixtures

The repository must include synthetic and realistic fixtures. Synthetic fixtures
must be generated from named scale profiles so compiler, runtime, CLI, LSP, and
adapter benchmarks exercise the same deterministic project shapes.

Synthetic scale profiles:

| Profile | Blocks | Lines | Choices | Localizable entries | Generated words |
| --- | ---: | ---: | ---: | ---: | ---: |
| tiny | 10 | 100 | 20 | about 120 | about 1,000 |
| small | 100 | 1,000 | 200 | about 1,200 | about 10,000 |
| medium | 1,000 | 10,000 | 2,000 | about 12,000 | about 100,000 |
| large | 5,000 | 50,000 | 10,000 | about 60,000 | about 500,000 |
| epic | 10,000 | 80,000 | 20,000 | about 100,000 | about 1,000,000 |

Each synthetic profile must define deterministic structural complexity targets:

- conditions on a representative subset of lines and choices, including shared
  flags, counters, and relationship-style state;
- metadata on blocks, lines, choices, and project inputs;
- deferred, immediate, and blocking effects with schema-checked payload shapes;
- localization catalogs and POT extraction pressure proportional to the
  localizable entry target;
- cross-block references and branching fan-out sufficient to expose reference
  resolution and choice lookup costs;
- stable line and choice IDs that remain deterministic across generator runs.

Synthetic fixture generation must take structured inputs: the scale profile,
the deterministic seed, schema shape configuration, localization configuration,
and any runtime fixture configuration needed for headless traversal. The
generator must produce Recite sources, schema files, runtime fixtures, and a
compact deterministic summary containing counts and content hashes. Summary
hashes are the reviewable signal that regenerated large fixtures still match the
expected shape without checking all generated data into git.

Checked-in synthetic fixture policy:

- check in the generator seed and profile configuration for every profile;
- check in the generated tiny fixture so smoke tests and examples work without a
  generation step;
- check in compact deterministic summaries for small, medium, large, and epic;
- generate small, medium, large, and epic fixture data on demand for benchmarks,
  stress checks, and profiling runs;
- do not check in generated fixture data whose size would make ordinary source
  review or clone time materially worse.

Realistic fixtures:

- conversation-heavy branching scene;
- object interaction scene set;
- relationship scene set with many conditions;
- localization-heavy scene set;
- effect-heavy scene set with deferred, immediate, and blocking effects.

Realistic fixtures should be compact enough to review by hand and checked into
the repository when possible. Larger realistic fixtures may follow the generated
fixture policy when they are derived from public, MIT OR Apache-2.0-compatible source material
or fully synthetic project descriptions.

Measurement hygiene:

- portable suites on Windows, macOS, and Linux must prove generator
  determinism, CLI stress correctness, and benchmark buildability;
- Criterion or the equivalent primary timing harness should provide warmup,
  sampling, outlier handling, baseline comparison, and noise reporting;
- authoritative trend numbers should come from one stable Linux runner or
  documented local Linux profile, not mixed operating-system timing;
- instruction, cache, and heap profiles may use Linux-only external tooling such
  as Valgrind or `perf`, but GPL tooling must remain documented external tooling
  rather than linked or vendored project dependencies;
- benchmark and profiling crate dependencies must be compatible with Recite's
  MIT OR Apache-2.0 distribution policy before they are added to the workspace.

### 19.3 Compiler Benchmarks

Compiler benchmarks must measure:

- parse time;
- lowering time;
- parser syntax tree memory;
- source AST allocation volume;
- validation time;
- schema validation time;
- block reference resolution time;
- ID uniqueness check time;
- markup validation time;
- POT extraction time;
- compiled asset serialization time;
- compiled asset size.

Initial target budgets on a typical developer laptop:

- small project compile: under 100 ms;
- medium project compile: under 1 s;
- large project compile: under 5 s;
- no superlinear blowups for ID checks, block resolution, or schema validation.

These are targets, not hard promises. If targets are missed, the benchmark report must make the cost visible.

### 19.4 Runtime Benchmarks

Runtime benchmarks must measure:

- `start_scene`;
- `next` for line events;
- `next` for prompt events;
- choice selection by ID;
- condition evaluation dispatch overhead;
- deferred effect collection;
- immediate effect emission;
- blocking effect pause and acknowledgement;
- locale lookup overhead;
- session serialization;
- session deserialization;
- full scene traversal with fixture context.

Initial target budgets:

- `next` without condition evaluation: allocation-free or near allocation-free after asset load;
- line/prompt advancement: comfortably under 50 us per event in release builds;
- choice selection by ID: effectively O(1) or O(log n), never linear over all project choices;
- session save/load: proportional to session state, not compiled asset size;
- runtime traversal must not clone full compiled assets.

### 19.5 LSP and Editor Benchmarks

LSP performance must be measured because authoring quality is a core product goal.

Benchmarks must cover:

- initial project indexing;
- open file parse;
- incremental edit parse;
- diagnostics refresh;
- completion latency;
- go-to definition latency;
- rename block latency;
- memory usage for indexed projects.

Initial target budgets:

- completion response under 50 ms for small/medium projects;
- diagnostics update under 100 ms for typical single-file edits;
- large project indexing should be incremental and cancellable;
- editor operations must avoid reparsing the entire project when a file-local edit is sufficient.

### 19.6 Engine Adapter Benchmarks

Engine adapters must measure:

- asset loading and conversion overhead;
- event emission overhead;
- active session update overhead per frame or tick;
- condition handler dispatch overhead through the host engine;
- generated typed effect event/signal conversion overhead.

An adapter should add negligible frame cost when no dialogue session is active.

### 19.7 Memory Metrics

Benchmarks must report memory-sensitive metrics where practical:

- syntax tree size during parser-heavy flows;
- source AST allocation volume;
- compiled asset size;
- peak compiler memory;
- runtime session size;
- LSP project index size;
- number of allocations during hot runtime traversal;
- number of clones of large strings or metadata vectors.

The runtime should prefer shared immutable compiled data plus compact session state.

### 19.8 Regression Policy

CI should run a fast benchmark smoke suite on every pull request and a fuller benchmark suite on release branches or scheduled jobs.

Performance regressions should be treated like test failures when they exceed configured thresholds:

- more than 10% regression in hot runtime paths;
- more than 20% regression in compiler/LSP paths;
- any accidental O(n^2) behaviour on medium or large fixtures;
- unexpected allocation increases in allocation-sensitive runtime benchmarks.

Benchmark thresholds must be adjustable as the implementation matures, but changes to thresholds should be reviewed explicitly.

### 19.9 Trace Metrics

`recite trace` should optionally emit performance counters:

- event count;
- line count;
- prompt count;
- choice count;
- condition evaluation count;
- effect count by mode;
- localization lookup count;
- elapsed traversal time;
- maximum serialized session size.

This makes real project dialogue scenes measurable without requiring users to write Rust benchmarks.

## 20. Import and Migration

Full ink/Yarn/Clyde import compatibility is not a v1 goal.

However, because the project is motivated by practical limitations across existing dialogue tooling, a limited migration helper may be valuable later:

- convert `Speaker: text #id:x #portrait:y` into structured line records;
- convert `+ Choice #id:x` into structured choice records;
- convert external function calls or engine-script hooks into effect declarations;
- convert tags into metadata entries.

This should be explicitly best-effort and should not constrain the native format.

## 21. Non-Goals

Initial non-goals:

- executing game state mutations inside the runtime;
- embedding variable storage in the dialogue runtime;
- implementing a full scripting language;
- hidden arbitrary code execution;
- result-dependent branching from blocking effects;
- full CLDR/pluralization engine in core runtime;
- mandatory visual node editor for v1;
- tying the authoring model to one engine's scripting language;
- engine adapters that move game logic into the Recite runtime;
- network/cloud collaboration;
- AI-authored dialogue features;
- automatic ID renaming based on content changes;
- implicit localization variant selection by the runtime;
- `:elif` / `else if` sugar (deferred until real authoring pain is reported; nested `:else` + `:if` and `:match` cover the use cases);
- general pattern matching beyond schema-declared enum dispatch (no destructuring, no tuples, no guards).

## 22. Recommended Milestones

### Milestone 1: Core Language Spike

- AST;
- rowan syntax parser and source AST lowering;
- line/choice/block syntax;
- source spans;
- simple compiler;
- basic validation.

### Milestone 2: Runtime MVP

- compiled asset;
- deterministic session;
- line/prompt/end events;
- choice selection by ID;
- deferred effects;
- condition evaluation through context;
- serialisable session state.

### Milestone 3: Production Effect Model

- immediate effects;
- blocking effects;
- acknowledgements;
- save/load while blocked;
- effect schema validation;
- effect trace tests.

### Milestone 4: Localisation and IDs

- stable ID checks;
- POT extraction;
- gettext-compatible lookup API;
- speaker extraction;
- markup preservation checks.

### Milestone 5: CLI and Test Harness

- `compile`;
- `validate`;
- `extract`;
- `check-ids`;
- `check-fresh`;
- `run`;
- `trace`;
- fixture support.

### Milestone 5.5: Authoring Polish

- `recite play` interactive REPL;
- `recite watch` authoring build loop for source/schema/project changes;
- LSP code action that auto-fills missing IDs on save;
- documented editor and engine authoring refresh loop.

### Milestone 6: Scale and Performance Proof

- deterministic large-project fixture generator;
- compiler and runtime benchmark suite;
- trace performance counters;
- validation, compile, run, trace, and snapshot stress tests;
- memory/profile notes for realistic project shapes;
- documented scale limits and regression policy;
- CI benchmark smoke suite.

### Milestone 7: LSP and Text Authoring Readiness

- diagnostics;
- completions;
- go-to definition;
- rename block;
- hover from schema;
- code action for missing IDs;
- saved/live project indexing that remains useful on large projects.

### Milestone 8: Engine Adapter Contract

- host-agnostic adapter contract;
- compiled asset loading boundary;
- start/select/ack integration shape;
- structured output events, messages, signals, or callbacks;
- condition handler integration;
- save/load handoff rules;
- adapter asset refresh and active-session reload policy;
- adapter conformance tests.

### Milestone 9: First Production Adapters

- at least one engine adapter, selected from active project pressure;
- credible adapter stories for the v1 target engines;
- compiled asset loading;
- native authoring asset refresh loops for Godot, Bevy, and Unity;
- start/select/ack integration;
- condition handler integration;
- example project;
- engine integration tests.

### Milestone 10: v1 Adoption Documentation and Release Readiness

- public documentation site;
- Rustdoc crate examples;
- complete workflow demo project;
- install, publishing, compatibility, and release policy;
- game-developer guides for core CLI, LSP, localisation, testing, and adapter workflows;
- engine-facing authoring refresh docs and examples, including known reload limits;
- alternatives and adoption guide grounded in the shipped v1 shape.

### Milestone 11: Migration and Interop

- transition guides from established dialogue systems;
- best-effort importer boundary design;
- unsupported-construct inspection prototype;
- honest compatibility notes for Ink, Yarn Spinner, Dialogic, Dialogue Manager, Dialogue System for Unity, and adjacent tools.

### Milestone 12: Editor Extensions

- VS Code extension;
- Neovim setup;
- syntax highlighting;
- command integration, including `recite watch`.

### Milestone 13: Visual Editor

- block graph;
- structured editing;
- schema-backed controls;
- trace preview;
- localization preview.

### Milestone 14: v1 Release Hardening

- release candidate checklist;
- compatibility audit across compiled assets, runtime snapshots, schema manifests, and CLI output;
- packaging and installation smoke tests;
- final documentation review against shipped commands and adapter workflows;
- final verification that docs, examples, adapter behavior, and known limits
  describe the same shipped authoring refresh workflow;
- known-limits document for scale, migration, editor support, engine integration,
  and active-session reload behavior.

## 23. Acceptance Criteria for a Serious v1

The project is not production-credible until all of the following are true:

- A dialogue scene can be compiled, validated, run, snapshotted, localized, and replayed headlessly.
- All runtime outputs are structured and deterministic.
- Effects are schema-checked and never executed by the runtime.
- Blocking effects can pause and resume across save/load, including re-emission of the pending effect with the same `EffectRequestId`.
- Choice IDs are stable and selection by ID is supported.
- Stable IDs survive author edits to source text. Renames happen only via the explicit code action.
- Metadata supports repeated ordered keys.
- Inline markup is preserved and validated.
- POT extraction produces translator-usable context.
- The LSP catches common mistakes before runtime, including auto-filling missing IDs on save.
- CI can verify compiled assets are fresh relative to source and schema.
- Authors have a fast documented loop from source edit to LSP diagnostics,
  on-save stable ID insertion, `recite watch` rebuild, engine adapter
  import/refresh, and scene restart or documented active-session behavior.
- Large-project fixtures exercise compile, validate, run, trace, localisation
  extraction, and snapshot restore at narrative scale comparable to serious
  dialogue-heavy games.
- Performance and memory characteristics are measured, documented, and protected by regression smoke checks.
- At least one production-quality engine adapter can load compiled assets, traverse dialogue, evaluate conditions, emit effects without executing them, and participate in save/load workflows.
- Each v1 adapter has a documented asset refresh/import workflow and an
  explicit active-session behavior for changed compiled assets.
- The adapter contract is stable enough that additional engines can be implemented without changing core runtime semantics.
- Public docs and examples demonstrate both headless CLI workflows and at least one real engine integration path.
- Adoption and migration guidance makes a credible case for teams evaluating Recite against established tools such as Dialogue System for Unity, Dialogue Manager, Dialogic, Yarn Spinner, and Ink.

Shipping a credible v1 means more than proving the core can run headlessly. The core runtime, CLI, LSP, scale proof, adapter contract, at least one production adapter, and adoption documentation must work together well enough for a serious narrative-heavy game team to evaluate Recite as a practical replacement for established dialogue tooling.

## 24. Design Summary

The core value is not “branching dialogue.” Many tools already do that.

The core value is a deterministic dialogue/effect protocol:

- authored in a small domain language that names narrative structure directly;
- validated before runtime;
- compiled into deterministic assets;
- run as a pure state machine;
- integrated with games through explicit typed effects;
- tested with normal programmatic assertions.

The source format is small. It is a way to describe dialogue structure, not a second general-purpose scripting layer. Conditions are pure queries. Effects are typed requests. Game logic stays in the game.

That is the standard the project should optimise for.
