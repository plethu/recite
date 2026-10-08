# Runtime and localisation

Part of the [production specification](../recite-production-spec.md). Section numbers remain stable.

## 8. Runtime

### 8.1 Core Requirements

The Rust runtime must be deterministic, side-effect free and testable without engine dependencies.
Session state is serialisable, including while a blocking effect is pending. Malformed content
returns structured errors rather than panicking.

### 8.2 Runtime API

The [runtime API](../../crates/recite-runtime/src/lib.rs) owns concrete types and signatures.
Callers start a session, advance it, select by stable choice ID and acknowledge blocking effects by
request ID. Section 9 defines locale selection.

### 8.3 Event Model

Events distinguish a line, a prompt with optional line and choices, an immediate or blocking effect,
and an ending with collected deferred effects. Hosts preserve event order and structured values. See
[`DialogueEvent`](../../crates/recite-runtime/src/event.rs).

### 8.4 Line Model

`text` is the resolved localized text.

`source_text` is the selected decoded source form, retained for diagnostics, fallback, tests, and
gettext semantics. For a plural line, the optional `DialoguePlural.singular_source_text` and
`DialoguePlural.plural_source_text` fields are the authored/raw source forms; they are not localized
templates and are not replaced by the selected decoded compatibility form in
`DialogueLine.source_text`.

### 8.5 Choice Model

Core selection uses `ChoiceId`. Adapters may expose index-based APIs for engine ergonomics, but the
core runtime should preserve stable choice identity.

`availability.primary_reason` is present only when an explicit choice-level `reason=...` override
applies. Tooling and adapters may derive compact display reasons from `reason_tree`, but that
presentation choice is outside runtime conformance output. `availability.reason_tree` is present
only for unavailable choices when the compiler and schema can resolve detailed structured reason
data. Available choices have neither a primary reason nor a reason tree; restoration rejects
contradictory saved availability rather than requerying the host. A flat display string does not
replace this data.

### 8.6 Session State

Session snapshots retain asset identity, traversal position, choices, effects, locale and history
needed for exact resumption. They never contain game state. The runtime's versioned snapshot types
own the field layout; hosts round-trip them as an opaque unit.

Restore validates the snapshot before constructing a live session. The runtime and preview snapshot
formats are versioned; both currently use v1. Unknown versions and missing or mismatched compiled
payload fingerprints are rejected, even when asset headers and source metadata match. Prepared
assets are immutable, so advancing reuses their validated identity.
[Serialization compatibility](../serialization-compatibility.md) governs unpublished snapshots and
future format changes.

#### Save/load while waiting on a blocking effect

If the session is saved while a blocking effect is pending, on resume the runtime re-emits the same
effect with the same `EffectRequestId`. The runtime makes no claim about whether the game-side
operation was partially executed before the save. The game decides whether to fast-forward, replay,
or otherwise reconcile and then calls `acknowledge_effect`. The same ID is re-emitted and requires
the same acknowledgement.

### 8.7 Error Handling

[`DialogueError`](../../crates/recite-runtime/src/error.rs) owns the structured error variants.
Malformed project content must not cause a panic.

## 9. Localisation

Dialogue content uses the gettext/POT and PO workflow below. Recite-owned UI text across the
CLI/TUI, Writer, LSP and editor extensions uses the shared
[Fluent resources](../../crates/recite-ui/resources). Stable resource IDs, English source resources,
extraction and completeness checks apply to every client. Generated host projections are allowed
where a manifest or metadata surface cannot consume Fluent; hard-coded UI strings are not another
path. Host-required metadata remains host-owned. Published non-English UI locales require human
authorship and review.

Dialogue localisation is opt-in. Without an explicit dialogue locale, the session and saved locale
remain unset, providers are bypassed and source text is delivered. Localised projects must declare a
default locale and fallback/catalogue policy at the project or fixture boundary. Neither mode may
infer dialogue locale from the host environment; Fluent UI locale never selects dialogue locale.

### 9.1 Requirements

The project must support gettext/POT workflows as a first-class path.

The standalone GUI must provide gettext PO catalogue editing as the required v1 editable
dialogue-catalogue path. It must preserve comments, context, unknown fields, stable IDs,
placeholders, and markup, and write changes through safe atomic replacement. Other catalogue formats
are explicitly read-only or import/export-only in v1; they must not be presented as editable
authoring surfaces. PO editing must remain separate from the Fluent resource contract for
Recite-owned UI text.

The current lossless PO editor retains its source representation. The bounded `polib` 0.3.0 and
`rspolib` 0.1.2 writer probes normalized untouched catalogues, so they do not replace that owner.
Reevaluate a dependency when it provides source-preserving edits and ranges, rather than maintaining
both its parser and our lossless representation. CLDR plural categories also do not replace
gettext's positional `Plural-Forms` expressions or their bounded validation.

Lines, choices, availability reasons, presentation labels, speaker display names and optional
schema-defined localisable metadata require stable IDs, source text and locations, translator
comments, and block/scene context where available.

### 9.2 POT Extraction

The CLI must emit POT files. The compiler extracts and validates entries; `recite-core::po` owns POT
values and serialization alongside lossless PO editing, so both use the same gettext escaping rules.

For dialogue lines and choices:

```po
#. file: Dialogue/hazel_rhea/small_talk.recite
#. block: small_talk_start
#. source id: small_talk_001@8f1c2d3e4a5b6c708192
#. speaker: rhea
msgctxt "8f1c2d3e4a5b6c708192"
msgid "Oh, hey! Didn't expect to see you here."
msgstr ""
```

Schema text uses separate gettext contexts:

| Text                         | `msgctxt`                          |
| ---------------------------- | ---------------------------------- |
| Speaker display name         | `dialogue_speaker:<speaker_id>`    |
| Availability reason template | `availability_reason:<reason_id>`  |
| Presentation label template  | `presentation_label:<template_id>` |

Availability reason placeholders follow the same placeholder syntax as line interpolation (§5.10).
Translation validation must reject missing, renamed, or extra placeholders relative to the source
template. Runtime reason localisation first resolves the template by `availability_reason:<id>`,
then renders the template with the structured `AvailabilityReasonArg` values recorded on the reason
leaf. `localized_text` on a reason leaf is the rendered display string; the localized template and
source template remain available through the reason ID and `template_source_text` for trace/debug
output.

Projection label placeholders follow the same placeholder syntax and validation rules as
availability reason placeholders. Runtime or adapter projection first resolves the template by
`presentation_label:<id>`, then renders it with the structured fields declared by the projector
output. Cross-adapter conformance output must preserve the template ID, source template, localized
text when resolved, and bound structured fields.

Reason parameters with registry-backed IDs render as stable symbols in v1. Localized display names
for registry values require a future self-contained compiled/localisation contract and must not be
fetched from game code or adapter registries during traversal.

### 9.3 Locale Provider

The [locale provider](../../crates/recite-runtime/src/locale.rs) receives stable ID and source text:
gettext lookup must match both `msgctxt` and `msgid`. The caller supplies the optional variant
(§9.5).

### 9.4 Fallback

If no translation is found for the requested locale, the locale provider must attempt broader
locales via BCP-47 region truncation before falling back to source text. Example: a lookup for
`pt-BR` falls back to `pt`, then to `msgid`.

The chain is the responsibility of the locale provider implementation. The spec requires:

- The terminal fallback is always the source text (`msgid`, or for plural lines `msgid` /
  `msgid_plural` selected by the English rule `n == 1`).
- Each step in the chain — including the terminal source fallback — must be observable in
  diagnostics or trace mode so missing translations and unintended fallbacks can be caught in tests.
- Fallback resolution must be deterministic for a given `(id, source, locale, variant, count)`
  tuple.

The runtime never invents broader locales beyond BCP-47 truncation. Cross-locale fallback (e.g.,
`nb` → `nn`) is the caller's job, configured outside the provider.

### 9.5 Grammatical Variants

IDs may support variant suffixes:

```text
8f1c2d3e4a5b6c708192&formal
8f1c2d3e4a5b6c708192
```

The caller explicitly selects the grammatical or register variant; the runtime never infers one.
Lookup tries `id&variant` through the locale fallback chain, then the base `id` through that chain,
then source text. Gettext `msgctxt` contains the full suffixed ID. Plural counts (§9.7) are a
separate axis, selected by the locale's validated gettext rule.

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
#. source id: letters_001@5e4d3c2b1a0987654321
#. speaker: narrator
msgctxt "5e4d3c2b1a0987654321"
msgid "You have one letter."
msgid_plural "You have {count} letters."
msgstr[0] ""
msgstr[1] ""
```

POT is a locale-neutral template: plural entries always contain exactly two empty `msgstr` arms for
extraction and deliberately do not contain a `Plural-Forms` header. A translated PO catalogue must
carry its own validated locale header and the number of arms declared there. Editing a POT and
loading the resulting translated PO are separate operations; the latter cannot borrow plural
metadata from another catalogue or from the source language.

The locale provider exposes structured plural resolution alongside singular lookup.

`PluralResolution` carries the optional translated template, matched locale, context, and source
key, matched arm, and deterministic candidate attempts. An attempt records its candidate locale,
context, source key, arm selected by that candidate's validated header, and outcome. A
provider-selected arm is authoritative only when that exact catalogue entry supplied the translated
template.

Lookup priority for plurals mirrors §9.5:

1. `id&variant` plural entry matching the locale's validated gettext rule for `count`;
2. `id` plural entry matching the locale's validated gettext rule for `count`;
3. source singular (if `count == 1`) or source plural (otherwise).

The fallback chain in §9.4 applies between steps 1 and 2 and between step 2 and step 3.

Plural translation validation must additionally detect:

- missing required `msgstr[N]` arms for the locale's declared `nplurals`;
- placeholder mismatch between any `msgstr[N]` and the corresponding source form;
- locales missing the `Plural-Forms` header in their `.po`.

The provider and catalogue loader share one bounded gettext expression parser and evaluator. A
catalogue's validated `Plural-Forms` expression is the only source of locale-specific arm selection;
clients do not embed separate locale-specific evaluators.
