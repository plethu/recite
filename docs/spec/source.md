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

The rowan parser preserves source, trivia, malformed regions and diagnostic spans, then lowers to
the compiler-facing AST. Compiler validation owns IDs, references, schema rules and deterministic
output. Runtime traversal does not depend on parser trivia. Editor Tree-sitter grammars provide
syntax highlighting only; they are not another semantic parser.

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

:: attempt_talk_down
! blocking resolve_dialogue_check(talk_down_guard, player, speech, 20)
:match dialogue_check_result(talk_down_guard)
  :case success
    > guard_relents@c123e8e85bf15374cb60
      Fine. Go through.
  :case failure
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
- Anchors and gettext contexts survive prose and label edits. Changed prose requires catalogue
  refresh and translation review because lookup matches both context and source text.

The anchor is `msgctxt`; the source text is `msgid`. A retained translation for changed source is
review material, not an exact match. Tooling never regenerates anchors from edited prose.

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

Source values preserve the distinction between symbols and quoted strings. Scalars are symbols,
strings, integers, floats or booleans; arrays contain scalars only. Nested arrays are not v1 syntax.

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

The core format must not hardcode keys such as `portrait`, `sfx`, `delay`, `shot`, `pose`, or
`focus`. Those keys belong in project schema. The tooling must still make project-specific metadata
validation excellent.

#### 5.6.1 Presentation Projection

Metadata can drive presentation on lines, choices, blocks and project inputs. A host may project
`portrait=smile` into a portrait cue or skill metadata into a label such as `[Speech 12/20]`. These
remain project conventions, not built-in runtime semantics. Costs, rolls, inventory changes and
other game operations use host state and typed effects.

[Schema §10.2.4](schema.md#1024-presentation-projection) owns projector declarations, validation,
ordering, stable affordance identity and presentation-query boundaries. Projection adds structured
presentation without changing the underlying event or its choice availability. A mandatory plugin
system or genre-specific schema package is not required for v1.

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
- Branches may be nested.

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
- The schema declares the query's enum return type. Runtime evaluation must return one declared
  variant or a structured error.

The intent is narrow: schema-checked exhaustive dispatch on declared enum state. Writers who do not
need it never see it; writers who do get compile-time coverage errors when a new enum variant is
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
