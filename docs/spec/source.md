# Source format

Part of the [production specification](../recite-production-spec.md). These are requirements;
implementation and release readiness require evidence from code, tests and the current GitHub
milestone. Section numbers remain stable.

## 5. Source Format

### 5.1 Requirements

The format must be human-readable, line-oriented where practical, and formally specified with a
grammar. Writers must not need to understand general programming beyond variables, function-style
conditions, simple boolean logic, and structured annotations.

Recite has a small domain language because dialogue has structure that should be named directly:
blocks, lines, choices, stable IDs, conditions, metadata, and effects. The format must teach a
portable way of thinking about narrative systems, not a one-off bridge into a specific engine
scripting language.

Dialogue prose must not be written as quoted string literals. Quoted prose creates the same awkward
formatting pressure as long strings in source code. Recite source should treat dialogue text as
indented body text owned by a structured statement header.

The source format should be indentation-first and must not mix one-line object literals, curly-brace
blocks, and ad hoc nested styles. The concrete grammar should use a small, consistent statement
vocabulary:

```text
:: block_name default      # block
> line_id@94d299352de485ec0b23                  # line
? choice_id@c9f4c6bbcb7103982051                # choice
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

A `>` line's indented body holds prose. Prose continues until a sibling-indented line begins with
one of `?`, `!`, `->`, `>`, `:if`, `:else`, or `::` — at that point the prose body ends and nested
statements begin at the same indent column. Blank lines inside prose are preserved as paragraph
breaks; blank lines do not by themselves terminate the prose body.

Nested statements inside a line body, conditional branch, or block share a single indent column.
Mixing indent widths within a body is a parse error.

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

The production parser uses a rowan-style lossless syntax tree as the core parser foundation. Syntax
parsing preserves source text, trivia, malformed regions, and recovery context, and reports syntax
diagnostics with stable codes and spans.

Valid and partially valid syntax lowers into the `recite-core` source AST. That AST is the
compiler-facing source model, not the parse tree. Parser responsibilities stop at syntax shape,
source spans, trivia, malformed regions, recovery, and parse diagnostics.

Compiler-facing validation owns stable ID policy, references, schema checks, match exhaustiveness,
semantic validation, and compiled output determinism. Runtime traversal must never depend on
parser-only trivia or malformed syntax nodes.

Tree-sitter is not part of the v1 core parser. It remains a possible future editor integration for
highlighting or structural editing after the rowan parser and lowering path are established.

Lossless syntax trees are heavier than AST-only parsing. Compiler and CLI flows should treat them as
temporary parse artifacts and lower promptly. LSP flows may retain syntax trees and a live index for
open or recently changed files.

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

> ta_001@b5960ef350446cba696b speaker=innkeeper portrait=neutral
  Welcome to the Rusty Flagon. Haven't seen you in a while.
```

The concrete syntax should optimise for writer ergonomics and LSP implementation while preserving
this structural shape.

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
> rhea_001@44b166e10a429205d903 speaker=rhea
  Hello.
```

Multiline prose is represented by the indented body:

```text
> rhea_014@f4dc069011f35d4c1ce5 speaker=rhea portrait=concerned
  I didn't know it was that bad.

  I mean, I knew it was bad.
  Just not... that bad.
```

Standalone effects (`!`) are top-level statements between dialogue events, not children of a line
body. Per-line presentation cues belong in metadata. See §7.5.

### 5.4 Choices

Choices are first-class records.

Each choice must expose:

- `id`: stable localisable choice ID;
- `text`: source text;
- `metadata`: ordered metadata entries;
- optional `requires=(<condition expression>)` availability requirement;
- optional `reason=<availability_reason_id>` primary unavailable reason override;
- `target`: block reference or `END`;
- `availability`: evaluated at runtime;
- `echo`: explicit echo policy.

Unavailable choices must be included in runtime output by default so callers can render disabled
choices. Hidden choices are authored structurally by placing the choice inside a `:if` branch. A
hidden choice is omitted from the prompt entirely; it is not a disabled prompt item.

Choice header clauses are dedicated syntax, not metadata:

```text
? ask_news@b34dda3cb1fa5853566e requires=(trust_gte(innkeeper, player, 3))
  What's the news?
  -> local_news

? ask_news_deeper@3ef1d3aca256d6ad1260 topic=rumours requires=(trust_gte(innkeeper, player, 5)) reason=innkeeper_trust_hint
  What aren't you telling visitors?
  -> guarded_rumours
```

Rules:

- `requires=(...)` is evaluated through the §6 pure condition language. If it evaluates true, the
  choice is available. If it evaluates false, the choice remains in prompt output with
  `availability.is_available = false` and structured availability reason data when one can be
  resolved.
- `reason=<availability_reason_id>` is an explicit primary presentation reason used when the
  requirement is false. The ID must reference a schema-declared, parameterless availability reason
  (§10.2.3). Use this for narrative exceptions or for negated and otherwise ambiguous expressions
  where automatic condition-derived reasons would be misleading. It does not erase the detailed
  derived reason tree when one can be produced.
- Metadata clauses may appear before or after `requires=(...)` and `reason=...`; metadata order must
  be preserved relative to other metadata entries. `requires` and `reason` are not emitted as
  metadata entries.
- `:if` is for structural omission and hidden choices. A choice omitted by `:if` is not in the
  previous prompt choice set; selecting its ID is invalid or stale, not unavailable.
- The old trailing choice `if` form is malformed syntax in v1. Authors should use `requires=(...)`
  for visible-but-unavailable choices and `:if` for hidden or structurally different dialogue.

Examples:

```text
# Plain single-player dialogue: disabled until trust is high enough.
? ask_news@6a6b706d5c267f9f7da2 requires=(trust_gte(innkeeper, player, 3))
  What's the news?
  -> local_news

# Visual novel: structural omission for a route-specific option.
:if route_active(rhea_confession)
  ? confess@f6c109bab34c9529ca23
    Tell Rhea the truth.
    -> confession

# Twine-like interactive fiction: disabled affordance with a reusable hint.
? open_door@c268a2f7f56bab22d1e3 requires=(has_flag(cell_key)) reason=need_cell_key
  Unlock the cell door.
  -> cell_exit

# CRPG-flavoured content without RPG-specific core syntax.
? intimidate_guard@a2622f8e848318ad7f2b requires=(trait_gte(player, presence, 4)) reason=presence_too_low
  Make the guard stand aside.
  -> guard_intimidated
```

Choice echo policies:

```text
echo = none
echo = selected_text
echo = line(4b3a1d9e8c7f6a5b2c10)
```

The default should be `none`. If a game wants the protagonist to repeat the selected choice, it
should be an explicit authored output, not a runtime quirk.

### 5.4.1 Choice Availability And Reasons

Choice availability is a prompt affordance, not control flow. It answers "can the player select this
visible option now?" Structural branches answer "does this dialogue content exist in this
traversal?"

Runtime behavior:

- A choice with no `requires=(...)` clause is available.
- A choice with `requires=(...)` remains in prompt output by default whether available or
  unavailable.
- Unavailable choices remain in previous-prompt/session state so the runtime can reject selection
  with an unavailable-choice error instead of treating the ID as stale.
- Selecting an unavailable choice returns a structured unavailable-choice error, does not advance
  traversal, does not emit choice echo, and does not record selected-choice history.
- Choices omitted by `:if` are not prompt choices. Selecting an omitted choice ID is invalid or
  stale according to the current prompt/session state.

Unavailable reason ownership:

- Recite runtime must not invent project-facing prose.
- Reusable unavailable reasons are declared in schema as localisable templates with typed parameters
  (§10.2.3).
- Boolean condition definitions may declare a default reason mapping from condition arguments to a
  reason template.
- Choice `reason=...` is a v1 primary presentation reason used when the requirement is false. It
  takes precedence for compact UI presentation and `primary_reason` output, while the detailed
  derived reason tree remains available when one can be produced safely. It must reference a
  parameterless availability reason in v1. Parameterised per-choice overrides require an explicit
  binding syntax and are deferred.
- Negated expressions (`not has_key(cell_key)`) and ambiguous compound expressions do not produce
  automatic reasons by default. Use a parameterless explicit `reason=...` override when presentation
  matters.

Compound requirements preserve their boolean structure in runtime output:

- `and` produces an `all` group: every failed child requirement explains why the choice is
  unavailable.
- `or` produces an `any` group: failed alternatives are preserved as alternatives, not flattened
  into one prose sentence.
- Parentheses preserve grouping.
- Leaf reason nodes include origin identity (condition call or full requirement expression), stable
  reason ID when resolved, template/source text when available, localized text when resolved, and
  bound reason arguments.
- If a choice-level primary reason is used for a negated or ambiguous expression, the detailed
  derived tree may be absent. The primary reason leaf records the full requirement expression as its
  origin and does not invent leaf reasons for the expression's child calls.

CLI/TUI surfaces may render a compact primary reason for readability, but `trace`, tests, and
adapter conformance output must expose the full structured reason tree.

#### Choice Presentation And Selection Resolution

Choice availability is the only core selection affordance in v1. Other choice-facing facts, such as
costs, risk labels, chance estimates, skill labels, consequence hints, route markers, tone labels,
or risky-option presentation, use the general metadata projection contract in §5.6.1. They must not
introduce choice-only magic metadata behavior.

Selection resolution remains host-owned:

- selecting a choice is always a deterministic `ChoiceId` operation;
- pre-selection gating uses `requires=(...)` with pure conditions;
- selecting an unavailable choice returns the structured unavailable-choice error described above
  and does not advance traversal;
- costs, rolls, random outcomes, inventory changes, relationship changes, and other game mutations
  are represented as schema-checked effect requests or as game state changes outside Recite, not as
  runtime behavior;
- if dialogue must branch on the result of a game operation, the game updates state and later
  dialogue queries that state through conditions. Blocking effects only acknowledge completion or
  failure in v1.

For example, a chance-based skill check is authored as ordinary choice metadata plus host-owned
resolution:

```text
? talk_down_guard@e8abb4465a68f6ad75bd check_skill=speech check_threshold=20 check_actor=player
  Talk the guard down.
  -> attempt_talk_down

@attempt_talk_down
! blocking resolve_dialogue_check(talk_down_guard, player, speech, 20)
:match dialogue_check_result(talk_down_guard)
  success:
    > guard_relents@c123e8e85bf15374cb60
      Fine. Go through.
  failure:
    > guard_refuses@f64d7023a39ec8ec5345
      Not a chance.
```

Presentation such as `[Speech 12/20] Talk the guard down.` or `[Visual Calculus: Impossible] Read
the scuff marks around the body.` is projected output, not source syntax. A projector may read the
choice metadata, query host state for current skill values or difficulty bands, and return
structured presentation affordances without changing the underlying `DialogueChoice`.

### 5.4.2 ID Assignment Policy

Every line and choice must reach the compiler with a stable anchor. Source headers use
`label@anchor`: the label is editable author-facing context, and the anchor is the canonical machine
identity.

- Authors may write line and choice headers without an ID. Example: `>` alone, or draft `>
  hazel_rhea.small_talk@` with no anchor.
- The LSP inserts a deterministic-but-unique 20-character lowercase hex anchor into the source file,
  producing e.g. `> hazel_rhea.small_talk@7f3a9c2e4b6d8f019a2b`. Anchors are selected to be unique
  across the project's shared line/choice namespace at insertion time.
- Once written to disk, anchors are **frozen**. The LSP never rewrites an existing anchor. Label
  edits are display/context changes and do not create rename records.
- Replacing an anchor changes identity. Explicit migration records for anchor replacement are future
  work.
- The compiler errors if any line or choice has a missing, draft, malformed, or plain unsuffixed ID.
  `recite check-ids` enforces the same.
- Because anchors do not encode content, translation files survive author edits to source text and
  label edits.

This policy keeps gettext-style translation stable: an edit to source text or label text does not
invalidate `msgctxt`, which stores the anchor. Auto-rewriting anchors based on content is an
explicit non-goal.

### 5.5 Prompts

The runtime must be able to represent choices attached to a line. Many games present a prompt line
and choices as one UI state.

The source format should support prompts as a line with nested choices:

```text
> ta_prompt_001@573fd5e9fea65bf417b8 speaker=innkeeper portrait=neutral
  What do you need?

  ? ta_opt_room@2df8dcd8991aacebed0c
    I need a room.
    -> get_room

  ? ta_opt_news@2a8f40266bfbea97f8bd requires=(trust_gte(innkeeper, player, 3))
    What's the news?
    -> local_news
```

A prompt may also omit line text and present choices only:

```text
? ta_opt_room@e777d797e41647f748ea
  I need a room.
  -> get_room

? ta_opt_leave@9e99c50eca0ac27500fa
  Never mind.
  -> END
```

### 5.6 Metadata

Metadata must be ordered and must allow repeated keys.

A plain string map is insufficient because existing production use cases include repeated cues such
as multiple sound effects or ordered presentation hints.

Runtime representation:

```rust
pub struct MetadataEntry {
    pub key: String,
    pub value: Value,
    pub source_span: Option<SourceSpan>,
}
```

Source metadata values must distinguish author spelling from compiled/runtime meaning. The source
AST preserves this as:

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

`SourceMetadataScalar` is the scalar subset: symbol, string literal, integer, float, and bool.
Nested arrays are not part of v1.

Metadata source spelling:

- bare values such as `portrait=grin` are symbols/reference tokens;
- quoted values such as `caption="Door closes"` are literal strings;
- integer, float, boolean, and array values remain typed literals;
- arrays validate each scalar element against the same metadata definition and domain rules as a
  single value;
- runtime-bound `$name` metadata values are reserved for explicit future support and must not be
  accepted silently as ordinary symbols; they are malformed until that support is added.

Compiled/runtime metadata semantics are schema-driven. Runtime consumers should not infer meaning
from whether a source value was bare or quoted; they consume the compiled value after schema
validation has assigned the allowed type and domain.

Metadata values must support:

- string;
- integer;
- float;
- boolean;
- arrays of scalar values.

The core format must not hardcode keys such as `portrait`, `sfx`, `delay`, `shot`, `pose`, or
`focus`. Those keys belong in project schema. The tooling must still make project-specific metadata
validation excellent.

Migration note: existing examples, fixtures, and tests should leave reference-like metadata values
bare (`portrait=grin`, `sfx=chime`, `speaker=rhea`). Literal display text or values that rely on
spaces or punctuation must be quoted. Existing generated fixtures that quote registry-like
presentation values are legacy inputs until the parser/schema implementation issue updates them.

#### 5.6.1 Presentation Projection

Metadata projection is a general presentation architecture, not a choice-only special case. If
metadata on choices can drive host UI affordances, metadata on lines, blocks, and project inputs
must be able to participate in the same contract. Otherwise Recite would create hidden special
meanings for one metadata target and make adjacent metadata targets surprising.

Projection has three layers:

1. Authoring metadata and schema describe project intent.
2. A pure presentation projector turns runtime output and compiled metadata into structured
   presentation affordances.
3. Host UI and game code decide how to render or resolve those affordances.

Core Recite must not define dice, difficulty classes, stats, factions, inventory, currency,
relationship meters, chance math, portrait behavior, camera behavior, or skill checks as runtime
semantics or source syntax. Those concepts belong to project schema, host game code, adapter
presentation layers, and optional projector definitions.

The minimum useful projector definition model should be generic over selector, input-source,
affordance-kind, and slot types so shared helper code can reuse the same structure for schema
manifests, adapter-owned extensions, tests, and host UI projections:

```rust
pub struct DialoguePresentationProjectorDefinition<TSelector, TInputSource, TKind, TSlot> {
    pub id: PresentationProjectorId,
    pub candidates: TSelector,
    pub inputs: Vec<ProjectionInput<TInputSource>>,
    pub queries: Vec<ProjectionQueryDefinition>,
    pub outputs: Vec<PresentationAffordanceOutputDefinition<TKind, TSlot>>,
}

pub type SchemaPresentationProjectorDefinition = DialoguePresentationProjectorDefinition<
    SchemaProjectionSelector,
    SchemaProjectionInputSource,
    PresentationAffordanceKind,
    PresentationSlot,
>;

pub enum SchemaProjectionSelector {
    RuntimeEvent { kind: DialogueEventKind },
    MetadataKey { target: MetadataTarget, key: String },
    MetadataSet { target: MetadataTarget, required_keys: Vec<String> },
    AvailabilityReason { reason_id: AvailabilityReasonId },
}

pub struct ProjectionInput<TSource> {
    pub name: String,
    pub source: TSource,
    pub ty: SchemaTypeRef,
    pub required: bool,
}

pub enum SchemaProjectionInputSource {
    EventKind,
    CandidateLineId,
    CandidateChoiceId,
    CandidateEffectRequestId,
    CandidateBlockId,
    CandidateProject,
    CandidateMetadata { key: String, occurrence: MetadataOccurrence },
    AvailabilityReasonArg { name: String },
    Literal(Value),
}

pub enum MetadataOccurrence {
    Only,
    First,
    Last,
    Index(u32),
    All,
}

pub struct ProjectionQueryFunctionDefinition {
    pub name: String,
    pub params: Vec<ParameterDefinition>,
    pub returns: SchemaTypeRef,
    pub max_calls_per_event: Option<u32>,
}

pub struct ProjectionQueryDefinition {
    pub name: String,
    pub function: String,
    pub args: Vec<ProjectionInputRef>,
}

pub enum ProjectionInputRef {
    Input { name: String },
    QueryResult { name: String },
}

pub struct PresentationAffordanceOutputDefinition<TKind, TSlot> {
    pub id: PresentationAffordanceOutputId,
    pub target: ProjectionOutputTarget,
    pub kind: TKind,
    pub slot: TSlot,
    pub label: Option<PresentationLabelDefinition>,
    pub fields: Vec<PresentationAffordanceFieldDefinition>,
}

pub enum ProjectionOutputTarget {
    Candidate,
    Event,
    Prompt,
}

pub struct PresentationLabelDefinition {
    pub template_id: PresentationTemplateId,
    pub source_text: String,
    pub args: Vec<PresentationLabelArgDefinition>,
}

pub struct PresentationLabelArgDefinition {
    pub name: String,
    pub source: ProjectionInputRef,
    pub ty: SchemaTypeRef,
}

pub struct PresentationAffordanceFieldDefinition {
    pub name: String,
    pub source: PresentationAffordanceFieldSource,
    pub ty: SchemaTypeRef,
}

pub enum PresentationAffordanceFieldSource {
    Input { name: String },
    QueryResult { name: String },
    Literal(Value),
}
```

This model is declarative. It can live in a generated schema manifest or in an adapter-owned schema
extension, but compiler, LSP, CLI, and adapter tooling must be able to inspect it without executing
game code. Validation must reject projector definitions that reference unknown metadata keys,
metadata targets not allowed by the key definition, unknown metadata domains, unknown query
functions, wrong argument types, invalid repeated-metadata occurrence requests, or output fields
that cannot be represented as structured values.

`candidates` selects the runtime or compiled items a projector may inspect. A projector runs once
per ordered candidate unless the selector is `RuntimeEvent`, which has a single event candidate.
Candidate order is:

1. event;
2. prompt container, when the event is a prompt;
3. prompt line, when present;
4. choices in runtime output order;
5. effect request, when the event is an effect;
6. current block, when known;
7. project.

Inputs using `CandidateLineId`, `CandidateChoiceId`, `CandidateEffectRequestId`, `CandidateBlockId`,
`CandidateProject`, or `CandidateMetadata` are relative to the current candidate. Candidate ID
inputs lower to stable string values. Validation must reject a candidate ID input that cannot apply
to the selected candidate kind: for example, `CandidateChoiceId` is valid only for choice
candidates. `CandidateProject` yields the stable project/content-set ID when one is declared, or is
a projection error if the compiled project has no stable project identity.

`MetadataOccurrence::Only` requires exactly one metadata entry after schema validation; it is a
projection error if the key is absent or repeated. `First`, `Last`, and `Index` select from the
source-order-preserved metadata entries for that key. `All` returns an array value in source order
and therefore requires the input type to be an array-compatible schema type. This keeps repeated
metadata explicit instead of letting projectors accidentally collapse multiple cues.

Projection query functions are schema-global declarations, separate from condition functions.
Function names must be unique in the projection query function table. Projectors reference those
global functions by name; duplicate or unknown function references are validation errors. Query call
argument types must match the declared function parameters. A query result type is always the
declared function return type, so `ProjectionQueryDefinition` does not carry a second return type
that could drift. Runtime or adapter code may still implement handlers through host-native APIs, but
the generated manifest remains the shared truth for what can be queried.

Each output definition has a stable `id`. Presentation affordance IDs are derived from
`(projector_id, output_id, target identity, metadata occurrence identity where relevant)` and must
not use host-generated counters, object addresses, or display labels. Output ordering is
deterministic: runtime event order, candidate order, projector definition order, output definition
order, then metadata occurrence order where one output expands over repeated metadata.

`PresentationLabelDefinition` is a schema-owned localisable template. Its `template_id` is the
stable extraction key. Each placeholder is bound by a named `PresentationLabelArgDefinition`; the
`name` must match a placeholder in `source_text`, and the `source` references a declared input or
query result. Translation validation rejects missing, renamed, or extra placeholders relative to
those named bindings. Adapter-owned labels may exist as host UI helpers, but they are outside
cross-adapter conformance unless they lower to a schema-owned template with stable ID, source text,
and typed placeholders.

The canonical generated manifest lowers into the concrete `Schema...` aliases. Rust helper APIs may
instantiate the generic parameters with richer host-native selector, input, kind, or slot types, but
those host types must still lower into the canonical schema model before compiler, LSP, CLI, or
conformance tooling depend on them.

V1 does not require core runtime APIs to execute projectors. The contract is still useful because
adapters, editor tools, docs, conformance fixtures, and future shared helper crates can agree on
stable inputs and outputs.

A projector is a pure presentation pass over runtime output. It takes a `DialogueEvent`, compiled
schema/projection definitions, relevant compiled metadata context, the active locale/variant, and a
caller-provided projection context, then returns structured affordances:

```rust
pub struct ProjectedDialogueEvent<TEvent, TTarget, TKind, TSlot, TSource> {
    pub event: TEvent,
    pub affordances: Vec<PresentationAffordance<TTarget, TKind, TSlot, TSource>>,
}

pub type RuntimeProjectedDialogueEvent = ProjectedDialogueEvent<
    DialogueEvent,
    ProjectionTarget,
    PresentationAffordanceKind,
    PresentationSlot,
    PresentationAffordanceSource,
>;

pub struct PresentationAffordance<TTarget, TKind, TSlot, TSource> {
    pub id: PresentationAffordanceId,
    pub target: TTarget,
    pub kind: TKind,
    pub slot: TSlot,
    pub label: Option<PresentationLabel>,
    pub fields: Vec<PresentationAffordanceField>,
    pub source: TSource,
}

pub enum ProjectionTarget {
    Event,
    Prompt,
    Line { line_id: LineId },
    Choice { choice_id: ChoiceId },
    Effect { effect_request_id: EffectRequestId },
    Block { block_id: BlockId },
    Project,
}

pub struct PresentationLabel {
    pub template_id: PresentationTemplateId,
    pub source_text: String,
    pub text: String,
    pub args: Vec<PresentationAffordanceField>,
}

pub struct PresentationAffordanceField {
    pub name: String,
    pub value: Value,
}

pub enum PresentationAffordanceKind {
    Prefix,
    Badge,
    RequirementSummary,
    Cost,
    ChanceEstimate,
    Risk,
    ConsequenceHint,
    PresentationCue,
    Custom(String),
}

pub enum PresentationSlot {
    BeforeText,
    AfterText,
    SecondaryLine,
    Tooltip,
    Icon,
    DisabledReason,
    TranscriptCue,
    Container,
}

pub enum PresentationAffordanceSource {
    Metadata { target: MetadataTarget, key: String },
    AvailabilityReason { reason_id: AvailabilityReasonId },
    Projector {
        projector_id: PresentationProjectorId,
        output_id: PresentationAffordanceOutputId,
    },
    AdapterPolicy { name: String },
}
```

`label` is presentation text resolved from a schema-owned `PresentationLabelDefinition` for the
current locale. `fields` and `label.args` must preserve the structured data used to build that
label, such as skill ID, display name, current value, threshold, difficulty band, chance estimate,
cost item, cost amount, risk level, route hint, portrait ID, sound cue ID, or camera cue ID.
Adapters may render labels as prefixes, badges, icons, secondary lines, tooltips, portrait swaps,
transcript cues, or other host UI, but adapter conformance output must preserve structured
affordance records rather than flattening them to a single host string.

Projection must not:

- add, remove, reorder, enable, or disable runtime choices;
- change line text, choice text, IDs, echo policy, targets, effects, or availability;
- mutate game state, emit effects, advance time, or perform random rolls;
- make runtime save/load depend on projected UI state;
- require parsing project-facing prose.

Projection errors must be structured adapter/tooling errors. They do not become runtime traversal
errors unless the adapter explicitly chooses to fail display when projection fails.

Adapters may expose lifecycle hooks for projection, but those hooks operate around runtime traversal
rather than inside it:

- `after_event`: receives a runtime `DialogueEvent` and may return a `ProjectedDialogueEvent` for UI
  display;
- `refresh_projection`: recomputes projection for the current event after relevant host state
  changes while the event is still visible;
- `schema_projection_loaded`: validates or registers projector definitions when a generated schema
  manifest or adapter schema extension is loaded.

These hooks must not call `choose`, `next`, or `acknowledge_effect`; mutate the runtime session;
emit game-side effects; or make projected state part of session serialization. Reprojecting the same
event with the same projection context must produce the same projected output. Reprojecting after
host state changes may change labels such as skill values, chance bands, cost availability,
portraits, or UI hints, but it must not change runtime choice availability unless the game advances
dialogue and the runtime emits a new prompt.

Projection queries are pure host queries for presentation, separate from condition evaluation. They
may read game state needed to show labels such as `[Speech 12/20]` or `[Visual Calculus:
Impossible]`, but they must not decide core traversal semantics.

Query providers should support a batch-oriented shape:

```rust
pub struct PresentationProjectionQuery<TTarget> {
    pub projector_id: PresentationProjectorId,
    pub target: TTarget,
    pub function: String,
    pub args: Vec<Value>,
    pub expected: SchemaTypeRef,
}

pub type RuntimePresentationProjectionQuery = PresentationProjectionQuery<ProjectionTarget>;

pub trait PresentationProjectionContext<TTarget> {
    fn evaluate_projection_queries(
        &self,
        queries: &[PresentationProjectionQuery<TTarget>],
    ) -> Result<Vec<Value>, ProjectionError>;
}
```

The projector builds a deterministic query list in runtime output order, then projector definition
order. Providers may coalesce identical queries and cache within a projection pass, but they must
return results in request order. Adapters must document whether projection queries are evaluated
synchronously, asynchronously before display, or through an engine-specific UI refresh path.

Projection queries must be bounded by the emitted runtime event, compiled metadata reachable from
that event, and declared projector definitions. They must not scan arbitrary engine resources or
perform unbounded searches during display. Resource-backed value discovery belongs in schema
manifest export (§10.2 and adapter contract §7), not projection.

Examples:

```text
# Line metadata can project a portrait cue.
> rhea_greeting@79e8dc1d5f3af8157e85 speaker=rhea portrait=smile
  You came back.

# Choice metadata can project a Fallout/Skyrim-style skill prefix.
? talk_down_guard@925d7aa147feea3e7085 check_skill=speech check_threshold=20 check_actor=player
  Talk the guard down.
  -> attempt_talk_down

# Block metadata can project scene-level presentation policy.
:: intro camera_mode=close_dialogue
```

Projected output examples:

```text
[Speech 12/20] Talk the guard down.
[Visual Calculus: Impossible] Read the scuff marks around the body.
```

Those prefixes are projector output, not source syntax. A Fallout/Skyrim-style projector might query
the current skill value and combine it with metadata thresholds. A Disco-style projector might query
or compute a project-defined difficulty band and render the configured skill display name plus band
label. Both projectors keep the underlying `DialogueChoice` unchanged.

Recite should not ship a mandatory v1 plugin mechanism or first-party affordance package for these
patterns. First-party documentation may include copyable schema, projector, and source examples for
common VN, IF, plain-dialogue, and RPG/CRPG workflows, but those examples are not normative schema
packages. Deferring a plugin package ecosystem avoids freezing genre-specific names before real
adapters and projects prove which conventions repeat across domains.

Future syntax or extension proposals must satisfy all of these criteria:

- the need recurs across multiple dialogue genres, not only RPG/CRPG checks;
- existing conditions, metadata, effects, schema domains, availability reasons, presentation
  projectors, projection queries, and adapter policy are demonstrably insufficient;
- the proposal preserves deterministic traversal and keeps game-side effects outside the runtime;
- the proposal can be represented as structured compiled/runtime data and validated without
  executing game code;
- adapters can preserve the data without weakening the engine-independent contract.

If a future extension/plugin contract becomes necessary, its minimum useful shape is schema
fragments, metadata domain definitions, availability reason templates, adapter presentation hint
names, diagnostics/LSP documentation, and examples. It must not include executable game logic,
runtime mutation hooks, or host-specific semantics in core Recite.

### 5.7 Inline Markup

Inline markup is allowed inside localisable text and must be preserved through extraction and
runtime delivery.

Examples:

```text
> hazel_rhea.small_talk.005@42b9ac5ab7fc3ee50cac speaker=rhea portrait=concerned
  [slow]I didn't know it was that bad.[/slow]

> hazel_rhea.small_talk.002@70c4f1ab40a347430ba7 speaker=hazel portrait=flat
  [shake]Yeah, funny.[/shake]
```

The project must provide markup validation:

- balanced tags;
- known tag names from schema;
- required tags preserved in translation;
- no invalid nesting where a tag schema forbids nesting;
- source spans for invalid markup.

The runtime does not interpret inline markup. Presentation layers may interpret it.

The bracketed tag form `[name]...[/name]` is deliberately distinct from ink's `[choice text]`
convention. The visual collision is acknowledged; the bracket form is chosen for parser simplicity
and translator familiarity.

### 5.8 Diverts

Blocks may divert to:

- a block in the same file;
- a block in another file;
- `END`.

Unknown targets must be validation errors.

Runtime traversal of an unknown target must return an error and never silently end the scene.

### 5.9 Conditional Branches

Conditional branches gate a section of dialogue (lines, choices, effects, diverts, nested branches)
on a condition expression.

```text
:if familiarity_gte(hazel, rhea, 3)
  > greet_warm_001@944703ea8a80a2530044 speaker=rhea
    You again. Good.
:else
  > greet_cold_001@096efb0a031aa3c6582c speaker=rhea
    Do I know you?
```

Rules:

- `:if <condition>` opens a body of statements at the next indent level.
- An optional `:else` at the same indent attaches to the immediately preceding `:if`. Anything else
  at that indent terminates the conditional.
- No `:elif` in v1. Chained boolean conditions are a smell — they typically indicate that the
  dispatch is on an enum (use `:match`, see §5.9.1) or that the branches should be separate blocks.
  Adding `:elif` later is trivial if real authoring pain is reported; removing it once authors
  depend on it is not.
- Conditions reuse §6 grammar, semantics, and validation. The expression must be a boolean
  condition.
- Lines inside a branch must still carry stable IDs (§5.4.2) and are extracted to POT regardless of
  which branch evaluates true at runtime.
- Branches may be nested arbitrarily.

#### 5.9.1 Enum Match

Pattern matching is restricted, additive sugar over `:if` chains for the case where dispatch is on
an enum. It is not general destructuring.

```text
:match thread_stage(rhea_job_response)
  :case tired
    > rhea_tired_001@dda242f6d7cd21051a6d speaker=rhea
      I'm exhausted. Let's keep it short.
  :case angry
    > rhea_angry_001@03f0d7a77024f6731eb4 speaker=rhea
      Don't.
  :case fine
    > rhea_fine_001@4df2ea266529d3f1a0ff speaker=rhea
      All right, what's up?
  :case _
    > rhea_default_001@f85d3061266f7f9de56c speaker=rhea
      Hey.
```

Rules:

- The match scrutinee is a single condition-grammar query (§6.1) whose return type is declared in
  schema as an enum.
- Schema must declare the function as enum-returning. Boolean-returning queries are not valid
  scrutinees — use `:if` for those.
- `:case <variant>` arms must reference declared variants of that enum. Unknown variants are
  validation errors.
- `:case _` is the wildcard arm. It matches any variant not covered above and may appear at most
  once, as the last arm.
- Arms are evaluated top-to-bottom. The first matching arm runs; the rest are skipped.
- The compiler validates **exhaustiveness**: a match must either cover every declared variant of the
  enum or include `:case _`. Missing arms are an error, not a warning.
- Duplicate `:case <variant>` arms are validation errors.
- Each arm's body follows the same indentation rules as `:if` bodies and may contain lines, choices,
  effects, diverts, nested `:if`, or nested `:match`.
- Schema producers should mark a condition function as enum-returning in the canonical schema model.
  Adapter code should do this through typed bindings, and the generated manifest records the enum
  type for compiler and LSP use:

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

- Runtime evaluation extends `DialogueContext` with an enum-returning lookup or, equivalently,
  schema-generated bindings convert host return values to declared variants. Either path is
  acceptable; the runtime contract is that the scrutinee returns one declared variant of the schema
  enum or evaluation fails as a structured error.

The intent is narrow: schema-checked exhaustive dispatch on declared enum state. Writers who do not
need it never see it; writers who do get compile-time coverage warnings when a new enum variant is
added and an old `:match` was not updated.

### 5.10 Text Interpolation

Localisable text may interpolate named values supplied by the caller.

Placeholders use curly-brace syntax. Each placeholder is `{name}`, where `name` is a lowercase ASCII
identifier (letters, digits, underscores; must start with a letter). Whitespace inside the braces is
not permitted.

```text
> letters_001@c6df367933e543042076 speaker=narrator bind=(letters_remaining:int=$letters_remaining)
  You have {letters_remaining} letters.
```

Placeholders must be declared on the line header using grouped `bind=(name:type=$value_name)`
attributes. Each attribute binds a placeholder name to a caller-supplied typed value at delivery
time. The same clause is accepted on choice headers. `type` is one of `string`, `int`, `float`, or
`bool`; grouped binding syntax is the only v1 binding form.

- An undeclared placeholder is a validation error.
- A declared attribute that is not referenced in the line's text is a validation error; remove the
  unused binding before compiling.
- The `$` sigil distinguishes runtime-bound references from metadata symbols and literal strings
  (`portrait=flat`, `caption="Door closes"`). `$name` metadata values remain reserved until explicit
  runtime-bound metadata support is designed.

Interpolation rules:

- Placeholders are preserved verbatim through POT extraction; translators see `{name}` in `msgid`
  and must preserve the same names in `msgstr`.
- Translation validation must catch missing, renamed, or extra placeholders relative to the source.
- Placeholders may appear inside inline markup (`[slow]{name}[/slow]`) but must not span tag
  boundaries.
- The runtime substitutes placeholders after locale lookup, before delivering the line text on
  `DialogueLine.text`. `DialogueLine.source_text` retains the unsubstituted source for diagnostics
  and fallback.
- Literal `{` and `}` in source text must be escaped as `\{` and `\}`. Escapes are preserved through
  extraction; the runtime emits literal braces in `text` and `source_text`.

Caller-supplied values are supplied through an explicit typed interpolation value provider/map
separate from `DialogueContext` and serialised session state. Missing values for declared attributes
are a structured runtime error, not silent omission.

Determinism: same line id, same declared values, same locale → same delivered text.

### 5.11 Plural Lines

Lines whose text varies by count declare two source forms — singular and plural — using a
continuation line prefixed with `|`. Selection between forms is governed by the locale's validated
gettext `Plural-Forms` expression, not by recite source syntax.

```text
> letters_001@d6e98b87e1e0a4699603 speaker=narrator bind=(count:int=$letters_remaining)
  You have one letter.
  | You have {count} letters.
```

Plural line rules:

- The line header must include a grouped `bind=(count:int=$<name>)` attribute. The bound value must
  resolve to a non-negative integer at delivery time.
- The singular form is the first body line. The plural form is the immediately following body line
  prefixed with `|`. Exactly two source forms are permitted; additional plural arms for translated
  locales live in `.po` (see §9.7).
- Both forms must be valid localisable text and may contain interpolation placeholders and inline
  markup.
- The placeholder bound by `count` may, but need not, appear in either form.
- POT extraction emits the line as a single entry with `msgid`, `msgid_plural`, and `msgstr[N]` arms
  (§9.7).
- The runtime resolves which form to deliver via the locale provider's plural lookup, supplying the
  count value. If the locale provider returns no translation, the runtime falls back to the source
  forms using the English rule (`n == 1 → singular`, otherwise plural).
- Plurals compose with variants (§9.5): `id&formal` may be a plural line.

Multiline body prose is not permitted on plural lines in v1. If a plural line needs more than one
paragraph, split it into separate adjacent lines.
