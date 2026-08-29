# Language authoring audit

This is the checked-in evidence for issue #165 and the language/schema part of
Milestone 2. It asks a practical question: can a writer keep a scene in their
head while the file is being edited, translated, split, and checked by tools?
The answer cannot come from a pretty minimal example. A useful language has to
survive the awkward scene: a half-typed header, a sentence beginning with an
arrow, a repeated presentation cue, a new enum variant, a translated plural,
and a block moved to another file on the same afternoon.

This document is an audit and a decision record in progress. It does not select
the final syntax for source composition, ordered bodies, marker escaping, or
identifiers. The alternatives below are deliberately capable designs, not
conservative defaults. They are here so that a later choice can be rejected by
evidence rather than by taste.

## Boundary and evidence

The accepted evidence baseline is commit `435c82e` (`integration/language-schema-readiness`) and
the current production contract in §§5–10, 12, 14–18, and 20 of
[`docs/recite-production-spec.md`](recite-production-spec.md). The observations
come from the parser, core model, compiler, runtime, LSP, and checked-in
fixtures, not from a hypothetical editor.

| Surface | What exists at the baseline | What this audit must protect |
| --- | --- | --- |
| Parse representation | Rowan-style lossless syntax, one logical source line at a time, with recovery diagnostics and source text round-tripping. | Trivia, malformed regions, spans, and enough partial structure for an editor to keep working. |
| Source model | `SourceFile` contains ordered blocks; `Statement` contains lines, choices, branches, matches, effects, diverts, and comments. | No second semantic authority in the CST or in an editor client. |
| Prose bodies | Indented prose is collected until a sibling statement; blank lines become paragraph breaks. Prose after a nested statement is diagnosed. | The author's paragraph and event order must be visible and deterministic. |
| Identity | Headers use `label@anchor`; frozen anchors are twenty lowercase hexadecimal characters. `SourceId` labels accept Unicode XID characters plus `_`, `-`, and `.`. | Labels may be edited without changing translation identity; anchor replacement must remain explicit. |
| Header values | Fields are whitespace-separated with quoted, parenthesised, and array-aware scanning. Metadata preserves order and repeated keys. | Presentation data must not turn into an unordered map or silently change type. |
| Conditions and effects | Conditions have calls, scalar arguments, `and`/`or`/`not`, grouping, and precedence. Effects are typed requests with explicit mode. | Conditions stay pure; runtime emits effects and never performs game work. |
| Match and references | `:match`/`:case` lower to an enum-shaped branch; `path::block` diverts are parsed and resolved by the compiler project index. | Exhaustiveness, ambiguity, and cross-file provenance must be diagnosed before runtime. |
| Text features | Markup is retained in `SourceText`; interpolation bindings and two-form plural lines lower through compiler/runtime delivery, and schema markup validation has stable diagnostics. | Markup, placeholders, and translator context must survive the same source-to-runtime path. |
| Localisation | Runtime locale lookup accepts stable ID, source text, domain, locale, and explicit variant. CLI catalogue loading has BCP-47 fallback and plural-arm traces. | Locale choice is explicit; fallback is observable; source-only sessions remain possible. |
| Editor state | LSP keeps open documents, rejects stale versions, and offers schema-aware hover/completion and stable-ID actions. | Recovery and edits must preserve IDs, positions, and cross-file meaning. |

Two concrete authoring risks remain. The parser currently classifies an
indented line beginning with `>`, `?`, `!`, `->`, `:if`, `:else`, `:match`,
`:case`, or `#` as a statement even when a writer means it as prose. The
condition lexer and metadata symbol parser currently accept ASCII identifier
characters, although source labels already use Unicode XID rules. The
accepted base now has executable interpolation and plural source-form
evidence; those core-v1 requirements are no longer an implementation gap in
this audit. Neither open risk selects marker escaping, body composition, or
identifier grammar.

## The pressure corpus

The corpus is split on purpose. The first pair is an executable, currently
supported core-language sample (assuming the referenced schema functions and
blocks exist). The next samples are target-only text probes or malformed
editing buffers. They must not be fed to today's runtime or POT extractor as
if they were valid assets. `market.recite` has the sole default block;
`archive.recite` is a non-default library file.

### Baseline-executable source

```text
# market.recite
:: marché.default default speaker=élise location=quai location=night

# The label is editable context; the anchor is machine identity.
> arrivée.entrée@5a74c6f3c0d8e1a2b4f6 speaker=élise mood=calm mood=urgent
  The tide is turning.

  The chalk on the door says something about the road.

  ? ask_news@10d1e2f3a4b5c6d7e8f9 tone=plain requires=(reputation.gte(elise, player, 3))
    What changed while I was gone?
    -> archive.recite::opening

:match dossier.stage(player)
  :case sealed
    > stage.sealed@abcdef0123456789abcd
      The dossier stays shut.
  :case open
    ! immediate play_sfx(lock_turn)
    -> archive.recite::opened
  :case _
    ! deferred record_thread(marche.default, uncertain)
    -> END
```

```text
# archive.recite (library; intentionally no default block)
:: opening speaker=archivist
> archive.opening@11223344556677889900 speaker=archivist
  The ledger remembers a different name.
-> END

:: opened speaker=archivist
> archive.opened@22334455667788990011
  Take it. The seal is already broken.
-> END
```

### Executable text probes

These examples exercise core-v1 interpolation and plurals. They are executable
parser/compiler/runtime fixtures; marker-leading prose remains a separate
authoring-design probe.

```text
> arrivée.interpolée@55667788990011223344 speaker=élise bind=(traveller_name:string=$traveller_name)
  [slow]The tide is turning, {traveller_name}.[/slow]

> letters.ledger@0a1b2c3d4e5f60718293 speaker=élise bind=(count:int=$letters_remaining)
  You have one letter.
  | You have {count} letters.

> sign@33445566778899001122
  The chalk on the door says:
  -> East, if you can read it.
  :if this is a sentence, not a branch.
  # ash marks the lintel.
```

### Malformed editing probes

These are expected to produce diagnostics and recovered CST nodes. They are
not valid source and must not be sent to runtime traversal or POT extraction.

```text
# half-typed buffer
:: marché.default default speaker=élise
> letters.ledger@ speaker=élise mood=$hero count=$letters_remaining
  [slow]The tide is turning, {traveller_name}.
  ? ask_news@10d1e2f3a4b5c6d7e8f9 requires=(reputation.gte(
    What changed?
  :case
    Still typing.
    -> archive.recite::opening
    Wrong indentation.

> indent_probe@abcdef0123456789abcd
    First indentation.
     Mixed indentation (expected RECITE_PARSE007).
```

The `indent_probe` body intentionally uses four spaces followed by five. `P`
must retain both lines and report the existing mixed-indent diagnostic
(`RECITE_PARSE007`) at the second body line; it is not part of an executable
corpus.

The marker-leading body lines are not exotic. A writer can quote a sign,
transcribe a note, or write a line of dialogue about a rule. The compiler must
not quietly turn those words into a divert, branch, or comment. Conversely, a
real nested choice must remain a choice. That is the central authoring tension
in this audit.

### Localisation material

This PO and fallback setup pair with the executable source text above. The
runtime fixture accepts the locale/catalogue configuration and the CLI trace
records plural source forms, count, and selected arm. The `[[scenario]]` table
below remains a review worksheet rather than runtime configuration.

The future catalogue exercises markup and plural forms. Its header is part of
that target scenario, not decorative PO syntax:

```po
# locales/fr-CA.po
msgid ""
msgstr ""
"Language: fr-CA\n"
"Plural-Forms: nplurals=2; plural=(n > 1);\n"

msgctxt "55667788990011223344"
msgid "[slow]The tide is turning, {traveller_name}.[/slow]"
msgstr "[slow]La marée tourne, {traveller_name}.[/slow]"

msgctxt "0a1b2c3d4e5f60718293"
msgid "You have one letter."
msgid_plural "You have {count} letters."
msgstr[0] "Vous avez une lettre."
msgstr[1] "Vous avez {count} lettres."

msgctxt "55667788990011223344&formal"
msgid "[slow]The tide is turning, {traveller_name}.[/slow]"
msgstr "[slow]La marée change, {traveller_name}.[/slow]"
```

```po
# locales/fr.po (configured fallback catalogue)
msgid ""
msgstr ""
"Language: fr\n"
"Plural-Forms: nplurals=2; plural=(n > 1);\n"

msgctxt "11223344556677889900&formal"
msgid "The ledger remembers a different name."
msgstr "Le registre se souvient d'un autre nom."
```

The lookup setup for this catalogue is explicit:

```toml
[dialogue]
locale = "fr-CA"
fallback_catalogs = ["fr"]

[[scenario]]
line_id = "55667788990011223344"
variant = "formal"
count = 2

[[scenario]]
line_id = "11223344556677889900"
variant = "formal"

[[scenario]]
line_id = "missing-id"
```

For the first scenario the future provider tries `fr-CA&id&formal` and selects
the plural arm for `count=2`. For the second, the `fr-CA` catalogue has no
entry, so lookup falls through to `fr&id&formal`. For the third, both
catalogues miss and source text is delivered. The future `L` oracle records
each attempted key and the final result, including `fr-CA → fr → source`;
variant and count remain separate axes. No current configuration loader or
runtime method is being claimed by this description.

### Scenario matrix

Each row names an observable test. “Current” describes the baseline; “target”
describes the contract the eventual implementation must earn. A row is not
passed by accepting text and losing its meaning downstream.

| ID | Pressure case and concrete input | Current observation | Target evidence and owner |
| --- | --- | --- | --- |
| A-01 | `arrivée.entrée` has two paragraphs, then a nested `? ask_news` choice. | Paragraphs are preserved; a sibling statement ends prose; prose after a child statement gets `RECITE_PARSE017`. | CST keeps paragraph boundaries; AST records body order; compiler/runtime produce the same prompt and line events. Parser, compiler, runtime tests. |
| A-02 | `arrivée.entrée@5a74c6f3c0d8e1a2b4f6` is renamed to `arrivée.quai` without touching the anchor. | `SourceId` distinguishes the editable label from the frozen anchor. | POT `msgctxt`, compiled line identity, traces, and LSP rename all retain `5a74...`; an explicit anchor replacement is a migration, not a rename. Compiler, LSP, localisation tests. |
| A-03 | `mood=calm mood=urgent location=quai` and an array such as `tags=[dock, "night watch"]`. | Lowering keeps repeated metadata in source order and preserves scalar types. | Schema validation sees every entry and its spans; compiled output has deterministic order; a projector consumes the declared domain rather than guessing from spelling. Core/compiler tests. |
| A-04 | `requires=(reputation.gte(elise, player, 3))`, `! immediate play_sfx(lock_turn)`, and `! deferred record_thread(...)`. | Dotted ASCII condition names and typed scalar arguments parse; effect mode is explicit. | Schema catches unknown functions, arity, and types; runtime returns an unavailable choice or typed effect request without game mutation. Compiler/runtime tests. |
| A-05 | `:match dossier.stage(player)` has `sealed`, `open`, and `_` arms. | Match and case nodes lower with source spans; schema owns enum return and exhaustiveness. | Unknown and duplicate variants, a missing arm, and a non-enum scrutinee produce stable diagnostics; arm traversal is first-match and deterministic. Compiler/runtime/LSP tests. |
| A-06 | `[slow]...[/slow]` wraps an interpolated name; an unmatched `[quiet]` is added while editing. | Source text is retained; compiler markup validation can report unknown, unbalanced, and invalid nesting tags. | The CST and source map locate the tag; translated text must retain allowed tags; runtime delivers markup without interpreting game effects. Compiler/localisation tests. |
| A-07 | `-> archive.recite::opening` and `-> archive.recite::opened`. | External block references parse as file plus block; compiler builds a project-wide index. There is no settled import/include surface. | File ordering cannot change resolution; missing file/block, path escape, duplicate exported block, and cycles have structured diagnostics; runtime asset contains resolved references only. Compiler/LSP tests. |
| A-08 | `bind=(count:int=$letters_remaining)` and `{traveller_name}` in source and PO. | Grouped bindings lower to typed compiled rows; runtime resolves caller-owned values after locale lookup and validates translated placeholders. | Compiler rejects undeclared/invalid placeholders; locale validation rejects missing/extra names; runtime errors on a missing or wrong-typed caller value. Core/compiler/runtime/localisation tests. |
| A-09 | Singular body followed immediately by `| You have {count} letters.`. | The parser lowers exactly two source forms; POT emits one gettext plural entry and runtime selects a source or translated arm. | Non-negative integer `count`, malformed/extra source forms, gettext arm/header validation, bounded evaluator selection, and variant/plural composition remain covered by parser/compiler/runtime/localisation tests. |
| A-10 | `speaker=élise`, label `arrivée.entrée`, `dossier.stage`, and a registry value `night-watch`. | Source labels accept Unicode XID plus `_-.`; condition and metadata symbol scanners are ASCII-started; LSP word extraction is narrower still. | The chosen grammar states whether Unicode is an identifier, a quoted value, or a display label; normalization, UTF-16 positions, lookup, and diagnostics agree. Parser/core/LSP tests. |
| A-11 | **Checked-in malformed probe:** body prose begins with `->`, `:if`, or `#`; a real nested `?` follows. | Marker classification wins before prose ownership: the probe records `RECITE_PARSE011` and `RECITE_PARSE013`, while source text and the recovered line owner remain inspectable. It is not a baseline runtime/POT input. | A writer can express marker-leading prose without changing the event stream; a real marker at the body boundary remains structural. The grammar decision remains open. CST/parser/LSP tests. |
| A-12 | **Malformed probe:** while typing `> letters.ledger@`, `mood=$hero`, `:match dossier.stage(`, `:case`, mixed indentation, and an unclosed tag. | Rowan round-trips the malformed text; parser and lowering diagnostics are stable and span-bearing. The probe is not valid runtime/POT input. | Partial trees retain the nearest useful owner; diagnostics do not cascade into dozens of invented statements; code actions never rewrite existing anchors. Parser/LSP/compiler tests. |
| A-13 | `fr-CA` lookup for line `5566...`, `fr` fallback, source fallback, `count=2`, and explicit `5566...&formal` variant, using the PO/header and TOML setup above. | Locale provider and CLI catalogue support explicit lookup, BCP-47 truncation, gettext plural arms, and source fallback. Runtime trace exposes both forms, count, and selected arm. | Variant and count remain independent; fallback remains deterministic; a missing translation cannot change IDs or effect order. Runtime/CLI/localisation tests. |
| A-14 | Move `archive.recite::opening` to another source root, then migrate an imported knot with a changed label. | Cross-file refs are compiler inputs. No importer implementation is implied here: the current repository only specifies bounded importer boundaries and ordinary-source output. | Migration reports changed identity, skipped constructs, and provenance. No importer forces Recite to adopt another tool's syntax or weakens native validation. Compiler/CLI migration tests. |

The baseline-executable pair is intentionally broader than a “hello scene”
but still small enough to run in every consumer. That same valid source should
feed parser snapshots, compiler diagnostics, headless runtime traces, and
cross-file resolution checks. Target-only probes feed the future parser,
compiler, runtime, and localisation checks named in their rows; malformed
buffers feed only recovery, diagnostic, and LSP checks. A client-specific
copy of the valid corpus would hide semantic drift.

### Accepted-base evidence closure

The bounded shared fixture is
[`fixtures/recite/valid/language_pressure.recite`](../fixtures/recite/valid/language_pressure.recite).
It is intentionally one ordinary source file with a local divert: the
fixture proves the current contract without choosing an import, body, marker,
or identifier design. The malformed companion is
[`parser_marker_leading_prose.recite`](../fixtures/recite/invalid/parser_marker_leading_prose.recite);
it is a recovery probe, not a runtime or POT input.

| Contract evidence | Exact checked-in pathway |
| --- | --- |
| Lossless parse, Unicode labels, stable anchors, line/choice bindings, plural source form, and ordered metadata | [`shared_language_pressure_fixture_round_trips_and_lowers_without_diagnostics`](../crates/recite-parser/tests/parser/syntax_and_fixtures.rs) |
| Current marker-leading-prose classification and source spans (`RECITE_PARSE011` and `RECITE_PARSE013`) | [`fixture_snapshots_record_current_marker_leading_prose_recovery`](../crates/recite-parser/tests/parser/syntax_and_fixtures.rs) and its [diagnostic snapshot](../crates/recite-parser/tests/snapshots/parser__fixture_support__fixtures_recite_invalid_parser_marker_leading_prose_diagnostics_txt.snap) |
| Compiler validation, source identity, interpolation bindings, plural forms, and preserved markup | [`shared_language_pressure_fixture_can_be_reused_by_compiler_validation`](../crates/recite-compiler/tests/validation/fixtures.rs), [`shared_language_pressure_fixture_preserves_ids_forms_and_bindings`](../crates/recite-compiler/tests/asset/shared_pressure.rs), and [`extracts_shared_language_pressure_fixture_entries_without_losing_context`](../crates/recite-compiler/tests/pot_extraction/shared_pressure.rs) |
| Runtime markup delivery, choice interpolation, plural count selection, and source fallback under an explicit locale | [`shared_language_pressure_fixture_preserves_localised_markup_and_source_fallback`](../crates/recite-runtime/tests/traversal/localisation/shared_pressure.rs) |
| CLI `fr-CA → fr → source` fallback, translated interpolation, plural-arm trace, and stable IDs | [`shared_language_pressure_fixture_exercises_locale_fallback_and_interpolation`](../crates/recite-cli/tests/dialogue_locale/shared_pressure.rs) |
| LSP reuse of the same valid source plus structured marker-diagnostic projection | [`shared_language_pressure_fixture_publishes_no_diagnostics`](../crates/recite-lsp/src/tests/diagnostics.rs) and [`shared_language_pressure_fixture_projects_marker_diagnostics`](../crates/recite-lsp/src/tests/diagnostics.rs) |

The valid parser, compiler, runtime, CLI, and LSP rows above consume the same
source bytes. The malformed parser and LSP rows consume the malformed companion
source to verify recovery and projected diagnostics. The CLI test writes small
in-test catalogues because this slice does not
change PO parsing or translated-markup validation ownership. The runtime test
also exercises source fallback for a missing translation; the CLI test proves
the configured broader-locale chain and translated plural arm.

### Core-v1 findings by severity

Only behaviours reproduced by the checked-in corpus or existing implementation
are classified here. The competing designs below remain review material, not
new decisions.

| Finding | Core-v1 severity | Evidence-backed status |
| --- | --- | --- |
| Marker-leading prose is classified as control syntax when it matches a statement marker; the focused probe records divert and condition diagnostics while preserving the source and recovered line owner. | P1 authoring-correctness risk | Open. A later decision must earn a body/marker grammar with the P/C/R/L/E oracles; this slice changes no grammar. |
| Condition and metadata symbol scanning remains ASCII-oriented while source labels accept Unicode XID names. | P2 authoring-consistency risk | Open. No identifier grammar, normalisation, or confusable policy is selected here. |
| Line and choice interpolation, inline markup preservation, plural source forms, and explicit locale/source fallback cross parser, compiler, POT, runtime, and CLI paths. | Resolved core-v1 evidence | Implemented at the accepted base and covered by the shared fixture plus the linked tests. Translated-markup validation remains a separate ownership boundary. |
| Stable source diagnostics are structured, span-bearing, snapshotable, and reused by LSP. | Resolved core-v1 evidence | Implemented at the accepted base and covered by the malformed probe, parser snapshot, and LSP projection of both diagnostic codes and ranges. |

### Settled maintainer boundaries

These boundaries are already present in the project instructions and production
specification and are recorded here so the pressure fixture cannot silently
move ownership:

- The parser/CST and lowering own syntax shape, source text, recovery, and
  spans. They do not decide runtime policy or a future marker/body grammar.
- The source AST and compiler own stable-ID validation, references, schema
  checks, source maps, POT extraction, and deterministic compiled output.
- Runtime owns deterministic traversal and structured delivery. It resolves
  caller-owned interpolation values after locale lookup and never executes
  game-side effects.
- CLI fixture/configuration owns explicit dialogue locale and catalogue paths;
  omitting the dialogue table remains the source-text-only mode.
- LSP consumes the shared parser/compiler diagnostic model and must not become
  an alternate semantic implementation.
- PO parsing/editor and translated-markup validation are not changed by this
  slice; the shared source is sufficient for source-markup/POT evidence, while
  translated-content validation remains with its separately bounded work.

### Operational oracle vocabulary

The rejection tests below are executable obligations, not requests for a
future “feel” review. Each candidate gets the same temporary fixture names and
the same host-neutral driver:

- **P (parse):** parse the exact UTF-8 buffer, assert
  `syntax().text() == input`, and compare the ordered diagnostic tuple
  `(category, primary span, related spans)`. A recovered buffer is successful
  only when its expected owner and all later statements remain addressable.
- **C (compile):** compile the valid corpus twice with the same schema and
  sorted project inputs, then compare compiled bytes, source-map entries, and
  diagnostic tuples. A permutation of independent files is an additional
  determinism check.
- **R (trace):** run the headless runtime with a scripted context and capture
  structured events `(line/choice/effect ID, source span, locale step,
  rendered text)`. Compare the complete ordered trace, including a bounded
  `TraversalLimitExceeded` result for an intentional divert loop.
- **L (catalog):** extract POT, load the supplied PO, and assert exact
  `(msgctxt, msgid, msgid_plural, msgstr arms, placeholders, markup)` tuples.
  For the locale setup above, assert the lookup sequence
  `fr-CA&id&formal`, `fr-CA&id`, `fr&id&formal`, `fr&id`, `source` when each
  preceding entry is absent. Assert `count=1` and `count=2` select the
  expected plural arms without changing the variant sequence.
- **E (editing):** apply a scripted edit, reparse the resulting buffer, and
  compare the exact anchor set, source spans for untouched nodes, and
  diagnostics before/after. No edit may rewrite a frozen anchor or turn a
  recovered node into an unrelated statement without a diagnostic.

For the human authoring claim, use a within-subject protocol with at least
five experienced authors, the same editor build and keyboard layout, and
randomised candidate order. Each author performs the seven corpus edits (add
a choice between paragraphs, paste a marker-leading sign, rename a label,
split a file, type an unclosed condition, add an enum case, and alter a plural
placeholder). Record completion time, undo count, and semantic recovery
errors. A candidate fails the edit-study gate if its median time exceeds the
current shape by 25% on at least three tasks, or if any task has a median of
more than one semantic recovery error. The study is supporting evidence; P,
C, R, and L remain hard correctness gates.

## Competing source-composition designs

Composition is where a local file becomes a project. The important distinction
is whether a reference names a dependency, whether a block is assembled from
fragments, and whether a writer can tell what will be compiled by looking at
the file. None of these three designs is selected here.

An import or fragment dependency cycle is not the same thing as a dialogue
loop. Composition cycles make the source graph ambiguous and must be rejected
or explicitly linearised. A dialogue divert loop can be intentional: a block
may return to an earlier block while the runtime advances through a finite
event budget. The `R` oracle accepts such a loop only when it produces a stable
`TraversalLimitExceeded` result at the declared bound, never an infinite host
call or a stack overflow.

### S-1: Explicit imports with qualified exports

```text
import "archive.recite" as archive

:: marché.default
-> archive::opening
```

Authoring is explicit and navigable: a file advertises its dependencies and a
qualified target tells the reader which file owns the block. The cost is a
header line and a rename operation when a file moves. The ambitious version
would allow selective imports, aliases, and re-exports while keeping every
edge visible in the lossless source.

The CST needs an `Import` node retaining path, alias, comments, and spans. The
AST needs ordered imports and a block-reference form that retains the written
qualification before resolution. The compiler can build a dependency graph and
reject cycles. If a future design permits composition cycles, it must specify a
deterministic linearisation order, preserve provenance for every inserted
statement, and pass the same `P`/`C` oracles before adoption. Runtime sees resolved block
identities, not filesystem reads. LSP can complete exports from an imported
file, offer move/rename edits, and report an unresolved import before compile.
Localisation IDs remain anchor-based, but POT comments should retain both
source file and imported provenance. Migration from a flat project can add
imports mechanically; migration must not guess an alias when two files export
the same name.

Failure cases include a path that differs only by case, an alias shadowed by a
local block, a dependency cycle with a default block on both sides, and an
import that is valid in the compiler but absent from an unsaved LSP overlay.
Run `P` on each malformed import and require one resolution diagnostic at the
path span; run `C` with imports permuted and require identical bytes; run `R`
on a permitted dialogue divert loop and require the bounded structured result.
Reject this design if any source span is lost, if import order changes the
compiled bytes, or if a file move requires rewriting frozen line/choice
anchors rather than only references.

### S-2: Open project namespace with qualified file paths

There are no import statements. The project manifest declares source roots and
all blocks enter a deterministic namespace; references name the file directly:

```text
-> dialogue/archive.recite::opening
```

This is fast for a small team: splitting a file does not require editing every
dependent header, and a project index can show every block. It is also an
ambitious open-world model: adding a source file can add symbols immediately.
That convenience makes dependency boundaries less legible and gives the
manifest more semantic weight than a source reader may expect.

The CST/AST need only preserve the reference and project-relative path; the
compiler owns canonical path, duplicate block, and root-boundary rules. The
runtime asset is as straightforward as S-1 once references resolve. LSP must
maintain an index over saved files plus unsaved overlays and clearly mark when
a reference is unresolved because a file is outside the open workspace. POT
provenance is direct, although moving a file changes translator comments while
the anchor remains stable. Migration from a directory of files is cheap; a
tool must report every reference whose path moved rather than silently finding
a same-named block elsewhere.

Failure cases are accidental capture of an unrelated file, two files defining
the same path/block under different spellings, and a build whose result depends
on directory traversal order. Run `C` once with an unrelated file and once
without it and require the same resolved reference and bytes; run `P`/`C` on
case-folded duplicates and require one ambiguity diagnostic; run `R` on a
permitted divert loop and require the same traversal-limit event. Reject this
design if adding an unrelated file changes an existing reference, if
case-folding differs between platforms, or if LSP and CLI disagree about the
live project namespace.

### S-3: Composable block fragments with explicit assembly

Files can contribute named fragments to a block. Assembly is itself source and
has ordering:

```text
:: marché.default
  + fragment "archive.recite::opening" after opening_line
  + fragment "market-notes.recite::warning" before choices
```

This design treats a chapter as an authored composition rather than a pile of
jumps. It can support reusable greetings, conditional inserts, and producer
generated fragments. The price is that a writer reads a block through a
linearisation operation, not only through its visible body. A fragment must
never be an invisible include that changes event order.

The CST needs fragment directives and their comments/spans. The AST needs an
explicit `BodyFragment` or block-assembly node with origin and insertion
anchor; lowering must preserve the pre-assembly order as provenance. The
compiler performs deterministic linearisation, checks fragment cycles,
identity collisions, and target validity, then emits an ordinary runtime
asset. Runtime need not know fragments exist. LSP should offer a virtual
assembled view alongside source navigation and show both the insertion site
and the fragment origin. Localisation needs stable IDs from the fragment's
source, collision diagnostics, and POT comments naming both owner and origin.
Migration can map selected ink knots or JSON arrays to fragments, but skipped
control flow must be reported rather than flattened.

Failure cases include two fragments inserting at the same relative position,
a fragment exporting a choice with an anchor already present in its owner, a
fragment that imports its owner, and a stale fragment whose source changed
after the owner was compiled. Run `P`/`C` on each and require an insertion,
identity, cycle, or freshness diagnostic at the fragment span; run `R` on a
permitted divert loop and require the same bounded result as the non-fragment
version. Reject this design if linearisation cannot be explained by a stable
source map, if a fragment move rewrites IDs, or if an editor cannot show the
final statement order without running game code.

## Competing ordered prose and statement bodies

The current body rule is intentionally simple: prose comes first and sibling
statements end it. That rule makes a line's localisable text a single clear
unit, but it cannot describe a sign that continues after a nested choice. The
following alternatives explore whether that is a grammar limit or a product
choice.

### B-1: Contiguous prose, then ordered statements (current shape)

```text
> prompt@1234567890abcdef1234
  Before the question.

  ? ask@abcdef0123456789abcd
    Ask it.
    -> next
```

This gives the writer one prose unit and a clear visual boundary. A body can
have many statements in source order, but prose cannot resume after the first
child. The CST needs no new body item; the AST keeps `SourceText` plus ordered
children. Compiler/runtime traversal is easy to reason about and localisation
gets one stable entry. LSP can complete a child at the boundary and diagnose
trailing prose with a precise span. Migration is mechanical for existing files;
authors must split a line when they need interleaving.

Failure cases are the marker-leading sign in A-11 and an author who indents a
sentence after a choice expecting it to remain part of the prompt. Reject this
shape if `P`/`R` require repeated line IDs solely to express one continuous
utterance, or if the edit-study protocol records a normal prose edit as a
semantic recovery error without a usable action.

### B-2: Interleaved body items with resumable prose

```text
> prompt@1234567890abcdef1234
  Before the question.
  ? ask@abcdef0123456789abcd
    Ask it.
    -> next
  After the question.
```

This follows the writer's eye: prose and nested events can alternate. The
source feels like a small stage direction stream, and an ambitious version can
support effects, choices, and conditional branches between paragraphs.

The CST must preserve each prose chunk and statement boundary as a `BodyItem`.
The AST needs an ordered body rather than `SourceText` plus `statements`; each
prose chunk needs a localisation identity or an explicit rule that all chunks
share the parent line ID. The compiler must lower the body into deterministic
events, and runtime must define whether a choice pauses within the same line
or ends it. LSP navigation and completion become richer but must handle a
partial item without reclassifying later prose. POT extraction, placeholder
validation, and markup checks operate per chunk; translators need paragraph
context and stable chunk IDs. Migration from B-1 can wrap the old text as one
item, but reverse migration is lossy.

Failure cases include a translated chunk changing paragraph count, a choice
selected after the parent line has already emitted a later chunk, and a save
reload that resumes in the middle of a body with no stable item identity.
Reject B-2 if `R` cannot identify the same body item after save/load, if `L`
finds two chunks under one POT context, or if the host driver must inspect CST
trivia to know which text is currently displayed.

### B-3: Explicit prose/event blocks

The body remains an ordered stream, but prose owns a visible delimiter:

```text
> prompt@1234567890abcdef1234
  text:
    Before the question.
  ? ask@abcdef0123456789abcd
    Ask it.
    -> next
  text:
    After the question.
```

This makes ordering unambiguous and gives future tools a place to attach
paragraph metadata, source comments, or a chunk-local ID. It is more ceremony
for an ordinary line, but it can support an authoring view that collapses
single text blocks back to the familiar shape.

The CST gets explicit `TextBlock` nodes; the AST gets ordered text and
statement items with spans and an authored identity policy. Compiler/runtime
semantics are clear because every emitted text event has an owner. LSP can
recover an unfinished `text:` body without stealing the next statement.
Localisation can extract each text block with explicit context and validate
markup/placeholders independently. Migration from existing prose can generate
text blocks without changing frozen anchors, while imports from systems with
interleaved events have a direct target. Runtime state becomes more granular,
so snapshot and replay tests must record the current text-block position.

Failure cases include an empty `text:` block, a nested choice accidentally
indented into text, and an author expecting two adjacent text blocks to be one
translation unit. Reject B-3 if ordinary one-paragraph scenes become
measurably harder to read under the edit-study protocol's 25%/three-task
threshold, or if `P`/`L` show that the explicit block buys no source-map or
localisation property that B-2 cannot provide.

## Competing marker and escape grammars

Markers are useful because they make the event vocabulary visible. They are
dangerous because prose is allowed to contain punctuation. The test is not
whether a clever parser can guess; it is whether a writer can predict the
result while typing a malformed line.

### M-1: Escape the first marker character

```text
> sign@1234567890abcdef1234
  \-> East, if you can read it.
  \:if this is painted on the wall.
  \# not a comment in the inscription.
```

The author keeps the compact marker vocabulary and gets a local, teachable
escape. The parser must distinguish an escaped body marker from a real child;
the rendered source text must contain the punctuation, not the backslash.

The CST retains the backslash token and raw text; the AST either stores an
unescaped `SourceText` plus source spelling or stores an escape-aware text
node. Compiler/runtime must never treat the escaped sequence as control flow.
LSP highlighting and completion must understand an escape while the line is
half typed. Localisation extraction should use the unescaped source text while
preserving enough spelling for a source-preserving editor. Migration can add
escapes only where the old parser classified prose as structure; it must not
rewrite existing prose blindly.

Failure cases are a literal backslash before a marker, `\\->` parity, an
escaped marker inside markup, and a missing escape while the author is typing.
Reject M-1 if `P` changes the number of backslashes, if `L` finds a PO `msgid`
different from the unescaped runtime source, or if the malformed-escape case
loses the following paragraph from the recovered CST.

### M-2: Explicit prose sigil for marker-leading lines

```text
> sign@1234567890abcdef1234
  | -> East, if you can read it.
  | :if this is painted on the wall.
  | # not a comment in the inscription.
```

Here `|` says “this is prose” and leaves the statement markers themselves
unescaped. It is visually strong in an editor and can make a block of quoted
signage obvious. It collides with the required plural continuation line, so a
plural-aware context rule or a different prose sigil would have to be earned;
that collision is evidence, not a reason to quietly remove plurals from v1.

The CST records the prose sigil and its span; the AST stores prose without the
sigil and keeps plural form nodes distinct. Compiler/runtime behavior is
simple once lowering has decided whether `|` belongs to a plural line or a
prose block. LSP can offer the sigil as a code action and show the collision
when a writer places it on a non-plural line. Localisation must never expose
the sigil in `msgid`. Migration can prefix only lines proven to have been
misclassified, with a reviewable edit report.

Failure cases include a real plural line beginning with a quoted pipe, a
choice whose text starts with `|`, and a partial `|` typed before the rest of a
line. Reject M-2 if plural parsing depends on hidden indentation heuristics,
if `L` finds the sigil in catalogue text, or if `P` emits more than one marker
diagnostic when the pasted inscription is a single malformed region.

### M-3: Token-aware markers plus raw prose fences

Ordinary prose stays unescaped unless it is in an explicit raw region:

```text
> sign@1234567890abcdef1234
  prose <<
  -> East, if you can read it.
  :if this is painted on the wall.
  # not a comment in the inscription.
  >>
  ? ask@abcdef0123456789abcd
    Ask what the sign means.
    -> next
```

Outside the fence, a marker is structural only when its header passes the
statement grammar at the body's statement indentation. Inside it, every line
is prose. This is the most capable option for transcribed documents and
generated text, at the cost of a second body mode and a fence that can itself
be mistyped.

The CST needs explicit raw-region nodes and recovered closing fences. The AST
must preserve raw prose as text while retaining source spans; compiler/runtime
see no statements inside it. LSP can provide a raw-region outline and recover
an unclosed fence to end-of-body. Localisation gets one source entry with
literal markers and still validates markup/placeholders. Migration can fence
known marker-leading prose, but a tool must ask before fencing ambiguous
lines. Runtime snapshots do not change because raw prose lowers to text.

Failure cases are a fence token in a genuine inscription, an unclosed fence
that swallows a following choice, and a nested raw region. Reject M-3 if an
unclosed fence hides a valid statement without a related `P` diagnostic, if
`P` cannot round-trip the raw mode, or if the edit-study protocol rejects the
additional mode under the same threshold as M-1.

## Competing identifier grammars

An identifier is not only a token rule. It is a promise about Unicode, path
qualification, normalisation, editor positions, schema names, registry values,
and what translators see. Stable anchors remain lowercase ASCII hex in all
options unless a separate decision changes that invariant.

### I-1: Unicode XID names with dotted qualification

Labels, block names, speakers, condition functions, and registry symbols use a
Unicode XID start/continue rule with `_`, `-`, and `.` as explicitly permitted
separators:

```text
:: marché.default speaker=élise
:if réputation.gte(élise, player, 3)
```

This respects names authors already write and makes dotted names useful for
project namespaces. It creates hard questions about NFC/NFKC, confusable
characters, whether `a.b` is one symbol or a path, and whether hyphens are
legal in callable names. Those questions are part of the design, not an edge
case to defer.

The CST retains exact spelling and Unicode spans. The AST should carry a
canonical lookup key plus authored spelling only if the normalisation policy
is explicit; the compiler must reject collisions after normalisation and
confusable policy rather than choose one by map order. Runtime receives
resolved IDs, not a locale-dependent string comparison. LSP must use Unicode
aware word boundaries and UTF-16 conversion. Localisation remains anchor-based
for line/choice text, while speaker and registry display names can be
localisable separately. Migration from ASCII is mostly additive, but a
normalisation collision must be a blocking report.

Failure cases include `é` versus a decomposed `e` plus accent, Latin `a`
versus a Cyrillic look-alike, `dossier.stage` versus a literal dotted registry
value, and a condition function whose name differs only by case. Reject I-1 if
`C` resolves two normalisation spellings differently on two named platform
runs, if `E` produces a range that splits the same grapheme, or if `L` changes
a catalogue context when a label is merely normalised.

### I-2: ASCII machine names with explicit human labels

Machine identifiers stay ASCII and path-safe; author-facing labels are
separate fields:

```text
> @5a74c6f3c0d8e1a2b4f6 label="arrivée.entrée" speaker=elise
  The tide is turning.
```

The language becomes easy to index, compare, and bind from every host while
still allowing human names in prose and metadata. The cost is more syntax and
the possibility that authors think the label is an address. An ambitious
version gives labels first-class rename/display semantics and lets schema
registries carry translated display names without using them as keys.

The CST/AST need separate anchor, machine name, and display-label fields with
spans. The compiler has simpler symbol resolution and can keep anchor policy
strict; runtime and serialised state remain compact and deterministic. LSP
must offer label edits without touching references and explain why a machine
name is required. Localisation can extract labels where declared while line
and choice contexts remain anchors. Migration must generate machine names
deterministically and record the mapping; it must not derive a new anchor from
translated text.

Failure cases include two labels with one machine name, a missing label on an
author-created line, a label that is mistaken for a target, and an imported
source whose human name was its only identity. Reject I-2 if the edit-study
protocol exceeds its 25%/three-task threshold because every line needs two
names, if `E` changes the label/machine-name mapping during a file move, or if
the LSP oracle displays the machine name where the source label is expected.

### I-3: Qualified identifiers with quoted segments

Every symbol is a sequence of explicitly qualified segments. Bare ASCII names
remain convenient, while Unicode, spaces, and punctuation require a quoted
segment:

```text
:if reputation."élise".gte(player, 3)
-> "archive.recite"::"opening"
```

This makes the difference between a path and one registry value visible. It
also opens an ambitious route to package/project namespaces, generated schema
names, and aliases without making `.` carry two meanings. The source is more
ceremonial, and quoted segments in conditions may feel like programming when
the author is simply naming a person.

The CST needs segment and separator tokens, retaining quote/escape spelling.
The AST should represent a qualified name rather than a string, and compiler
resolution can report which segment failed. Runtime gets resolved symbols;
LSP can complete each segment and rename one namespace component. Localisation
contexts remain stable anchors, while schema error messages can show both
written and canonical names. Migration has a clear rewrite from path-like
names, but old dotted registry values need an explicit classification report.

Failure cases include a quoted segment containing `::`, a path whose segment
is empty, aliases that make two qualified names equal, and a translation that
changes a quoted condition argument. Reject I-3 if `P` needs divergent token
rules for header fields and conditions, if `E` drops quote style, or if the
edit-study protocol exceeds its 25%/three-task threshold because ordinary
identifiers must be quoted merely to avoid ambiguous dots.

## Decision gates and remaining work

The next implementation slice should turn the pressure corpus into shared
fixtures before choosing syntax. At minimum, every candidate that remains in
contention must demonstrate:

- a lossless CST round trip for complete and half-typed source;
- one lowered AST shape with spans and source provenance, with no parser-only
  trivia consulted by runtime traversal;
- deterministic compiler diagnostics for duplicate names, missing references,
  malformed markup, invalid placeholders, non-exhaustive matches, and schema
  type errors;
- deterministic headless runtime traces, including explicit locale fallback,
  interpolation after lookup, plural selection, and stable effect order;
- LSP completion, definition, rename, UTF-16 positions, stale-document
  rejection, and recovery on A-10 through A-12;
- POT/PO extraction and validation that preserve anchors, markup, placeholders,
  plural arms, translator comments, and fallback observability; and
- a migration report that names every lossy or ambiguous construct instead of
  silently changing it.

The review should measure real editing actions rather than only count grammar
tokens: insert a choice in a paragraph, paste a marker-leading sign, rename a
label, split a file, type an unclosed condition, add an enum case, and change a
translation's plural placeholder. A candidate fails when it makes one of
those actions ambiguous, loses identity, or requires a client to reimplement
compiler semantics.

Two product decisions are already settled for this audit. Interpolation and
plural lines are core v1, not optional extensions. Also, the project is
pre-release: preserving a weak syntax solely to avoid migration is not an
acceptance criterion. The remaining choices belong in a follow-up decision
after the shared corpus, fixtures, and rejection evidence exist.
