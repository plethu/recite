# Source format

Part of the [production specification](../recite-production-spec.md). Section numbers remain stable.

## 5. Source Format

### 5.1 Requirements

Recite is an indentation-first language with structured statement headers and unquoted prose bodies.
Its formal grammar must remain usable without general programming knowledge beyond variables,
function-style conditions, simple boolean logic and annotations. Do not mix object literals,
curly-brace blocks or competing nesting styles into this vocabulary:

```text
:: block_name default      # block
> fields @line_id          # line
? fields @choice_id        # choice
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

The format must support exactly one default block per file or project, references within and across
files, includes/imports, and stable source spans for diagnostics. The following sections specify the
statement forms.

### 5.1.1 Parser Architecture

The rowan parser preserves source, trivia, malformed regions and diagnostic spans, then lowers to
the compiler-facing AST. Syntax diagnostics are eager; the immutable lossless tree is materialised
on first syntax access. Callers that only need the AST can lower borrowed source directly without
constructing the lossless tree. Both paths share syntax diagnostics and recovery scanning; region
lowering preserves original file positions. Root-statement visitors share that lowering and recovery
path, retain nested statements within their root, and can stop between roots without retaining a
whole block AST. Compiler validation owns IDs, references, schema rules and deterministic output.
Runtime traversal does not depend on parser trivia. Editor Tree-sitter grammars provide syntax
highlighting only; they are not another semantic parser.

### 5.2 Blocks

Named blocks contain statements and may declare metadata and a default speaker context:

```text
:: tavern_arrival default

> speaker=innkeeper portrait=neutral @ta_001
  Welcome to the Rusty Flagon. Haven't seen you in a while.
```

### 5.3 Lines

A line is a localisable output unit with a stable ID, optional speaker, ordered metadata and source
location. Its source text preserves inline markup, which is validated separately.

Speaker names must not be parsed from line text. The following is invalid as a speaker declaration:

```text
Rhea: Hello.
```

Instead, speaker must be structured in the line header:

```text
> speaker=rhea @rhea_001
  Hello.
```

Multiline prose is represented by the indented body:

```text
> speaker=rhea portrait=concerned @rhea_014
  I didn't know it was that bad.

  I mean, I knew it was bad.
  Just not... that bad.
```

Standalone effects (`!`) are top-level statements between dialogue events, not children of a line
body. Per-line presentation cues belong in metadata. See §7.5.

### 5.4 Choices

Choices have stable localisable IDs, source text, ordered metadata and a target block or `END`.
`requires=(...)` declares availability; `reason=...` supplies a primary unavailable reason (§5.4.1).
These clauses are dedicated syntax, not metadata:

```text
? requires=(trust_gte(innkeeper, player, 3)) @ask_news
  What's the news?
  -> local_news

? topic=rumours requires=(trust_gte(innkeeper, player, 5)) reason=innkeeper_trust_hint @ask_news_deeper
  What aren't you telling visitors?
  -> guarded_rumours
```

Rules:

- `requires=(...)` uses the §6 pure condition language.
- Metadata clauses may appear before or after `requires=(...)` and `reason=...`; metadata order must
  be preserved relative to other metadata entries. `requires` and `reason` are not emitted as
  metadata entries.
- The old trailing choice `if` form is malformed syntax in v1. Authors should use `requires=(...)`
  for visible-but-unavailable choices and `:if` for hidden or structurally different dialogue.

Use `:if` to omit a choice entirely:

```text
:if route_active(rhea_confession)
  ? @confess
    Tell Rhea the truth.
    -> confession
```

Choice echo policies:

```text
echo = none
echo = selected_text
echo = line(confession_echo)
```

The default is `none`; repeating the selected choice requires an explicit echo policy.

### 5.4.1 Choice Availability And Reasons

A choice is available unless its `requires=(...)` expression evaluates false. Unavailable choices
remain in prompt output and session state so callers can render them disabled. Selecting one returns
a structured unavailable-choice error without advancing, emitting echo or recording a selection. A
choice omitted by `:if` is absent from the prompt; selecting its ID is invalid or stale.

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

Selection is a deterministic `ChoiceId` operation. Costs, rolls and other game mutations remain
host-owned, requested through typed effects or performed outside Recite. To branch on a game
operation's result, update game state and query it through a later condition. Blocking effects only
acknowledge completion or failure in v1.

For example, a chance-based skill check is authored as ordinary choice metadata plus host-owned
resolution:

```text
? check_skill=speech check_threshold=20 check_actor=player @talk_down_guard
  Talk the guard down.
  -> attempt_talk_down

:: attempt_talk_down
! blocking resolve_dialogue_check(talk_down_guard, player, speech, 20)
:match dialogue_check_result(talk_down_guard)
  :case success
    > @guard_relents
      Fine. Go through.
  :case failure
    > @guard_refuses
      Not a chance.
```

Presentation such as `[Speech 12/20] Talk the guard down.` or `[Visual Calculus: Impossible] Read
the scuff marks around the body.` is projected output, not source syntax. A projector may read the
choice metadata, query host state for current skill values or difficulty bands, and return
structured presentation affordances without changing the underlying `DialogueChoice`.

### 5.4.2 ID Assignment Policy

Every line and choice must reach the compiler with a stable ID. A header spells its ID as one
trailing `@id` field after any metadata and clauses:

```text
> speaker=mara @desk_greeting
? requires=(trusts(player)) @k7q3m9x
```

- An ID uses lowercase ASCII letters, digits, `_`, `.` and `-`, and starts with a letter or `_`. `&`
  (localisation variants), `:` (catalogue context prefixes) and `/` (reserved for a future pack
  qualifier) never appear in an ID. Lines and choices share one project namespace.
- Generated IDs are the documented default: seven lowercase Crockford base32 characters without `i`,
  `l`, `o` or `u`, starting with a letter. Tooling draws them at random and checks each against the
  project namespace, so branches that insert at the same place do not generate the same ID. Authors
  may write a readable ID instead; it carries the same freeze obligation.
- A header without an ID is incomplete. `recite id assign` and the LSP missing-ID action append a
  generated `@id` field after the header's last field. Neither rewrites an existing or malformed ID.
- Once written, an ID is **frozen**. Changing it changes identity: translations and saves keyed on
  the old ID no longer match. Before a release, an ID change only requires regenerating catalogues
  and development saves. After a release, treat it as a breaking content change: `recite id check
  --since <release tag>` fails for every ID that has disappeared since that git revision unless it
  is acknowledged with `--allow-removed <id>`, and fails if the revision is unavailable rather than
  passing on an empty baseline. Projects outside git keep only the duplicate check.
- Concurrent branches can still add the same ID. The compiler and `recite id check` report the
  duplicate before merge; reassign the branch's new ID.
- Renaming or reassigning an ID is one explicit, tool-assisted edit: `recite id rename <ID|PATH:LINE>
  <NEW>`, `recite id reassign <ID|PATH:LINE>` (a freshly generated ID) and LSP rename on an `@id`
  field share one authoring-kernel plan. It rewrites the header and, in every project catalogue
  (§11.1 `[discovery].catalogs`), each active, obsolete (`#~`) and previous (`#|`) `msgctxt` equal
  to the old ID or `old&variant`, keeping the variant. It refuses a malformed ID, one containing
  `&`, `:` or `/`, one already carried by a line or choice, and one that still keys catalogue
  entries, which would otherwise revive orphaned translations. When the old ID is duplicated, only
  the selected header changes; catalogue entries cannot be attributed to one side and stay with the
  rest. There is no alias table, sidecar or automatic rename detection: after a release the old ID
  is reported as removed unless acknowledged as above.
- The compiler errors if any line or choice has a missing, malformed or duplicate ID. `recite id
  check` enforces the same.
- A bare ID belongs to the project's own content. A future pack or DLC qualifier is applied when
  assets are loaded and does not change source.

The ID is gettext `msgctxt`; source text is `msgid`. IDs survive prose and metadata edits, but
changed prose requires catalogue refresh and translation review: lookup matches both fields. A
retained translation for changed source is review material, not an exact match.

Compiled assets carry a format version. A runtime refuses a version it does not know rather than
migrating it; development assets and snapshots from before this ID format are regenerated.

### 5.5 Prompts

The runtime must be able to represent choices attached to a line. Many games present a prompt line
and choices as one UI state.

The source format should support prompts as a line with nested choices:

```text
> speaker=innkeeper portrait=neutral @ta_prompt_001
  What do you need?

  ? @zbcjedn
    I need a room.
    -> get_room

  ? requires=(trust_gte(innkeeper, player, 3)) @ta_opt_news
    What's the news?
    -> local_news
```

A prompt may also omit line text and present choices only:

```text
? @mb944yy
  I need a room.
  -> get_room

? @ta_opt_leave
  Never mind.
  -> END
```

### 5.6 Metadata

Metadata preserves entry order and repeated keys, including repeated sound or presentation cues.

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

Keys such as `portrait`, `sfx` and `delay` belong in project schema, not the core format.

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
> speaker=rhea portrait=concerned @hazel_rhea.small_talk.005
  [slow]I didn't know it was that bad.[/slow]

> speaker=hazel portrait=flat @hazel_rhea.small_talk.002
  [shake]Yeah, funny.[/shake]
```

The project must provide markup validation:

- balanced tags;
- known tag names from schema;
- required tags preserved in translation;
- no invalid nesting where a tag schema forbids nesting;
- source spans for invalid markup.

The runtime does not interpret inline markup. Presentation layers may interpret it.

`[name]...[/name]` denotes markup, not choice syntax. Parser simplicity and translator familiarity
justify the visual overlap with Ink's bracketed choice text.

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
  > speaker=rhea @greet_warm_001
    You again. Good.
:else
  > speaker=rhea @greet_cold_001
    Do I know you?
```

Rules:

- `:if <condition>` opens a body of statements at the next indent level.
- An optional `:else` at the same indent attaches to the immediately preceding `:if`. Anything else
  at that indent terminates the conditional.
- There is no `:elif` in v1. Use nested branches, separate blocks or `:match` for enum dispatch.
  Adding it after demonstrated need is easier than removing syntax authors already depend on.
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
    > speaker=rhea @rhea_tired_001
      I'm exhausted. Let's keep it short.
  :case angry
    > speaker=rhea @rhea_angry_001
      Don't.
  :case fine
    > speaker=rhea @rhea_fine_001
      All right, what's up?
  :case _
    > speaker=rhea @rhea_default_001
      Hey.
```

Rules:

- The scrutinee is a single condition-grammar query (§6.1) declared in schema as enum-returning. Use
  `:if` for boolean queries.
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
- Runtime evaluation must return one declared variant or a structured error.

### 5.10 Text Interpolation

Localisable text may interpolate named values supplied by the caller.

Placeholders use curly-brace syntax. Each placeholder is `{name}`, where `name` is a lowercase ASCII
identifier (letters, digits, underscores; must start with a letter). Whitespace inside the braces is
not permitted.

```text
> speaker=narrator bind=(letters_remaining:int=$letters_remaining) @fdm9t1h
  You have {letters_remaining} letters.
```

Placeholders must be declared on the line header using grouped `bind=(name:type=$value_name)`
attributes. Each attribute binds a placeholder name to a caller-supplied typed value at delivery
time. The same clause is accepted on choice headers. `type` is one of `string`, `int`, `float`, or
`bool`; grouped binding syntax is the only v1 binding form.

- An undeclared placeholder is a validation error.
- A declared attribute that is not referenced in the line's text is a validation error; remove the
  unused binding before compiling.
- `$` distinguishes caller-bound values from metadata symbols and literals (§5.6).

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
> speaker=narrator bind=(count:int=$letters_remaining) @a2az6wq
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
- [Localisation §9.7](runtime-localisation.md#97-plural-forms) defines extraction, variant lookup,
  locale-specific arm selection and the English source fallback.

Multiline body prose is not permitted on plural lines in v1. If a plural line needs more than one
paragraph, split it into separate adjacent lines.
