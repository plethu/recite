# Runtime and localisation

Part of the [production specification](../recite-production-spec.md). These are
requirements; implementation and release readiness require evidence from code,
tests and the current GitHub milestone. Section numbers remain stable.

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
    locale: Option<LocaleId>,
) -> Result<DialogueSession, DialogueError>;

pub fn next(
    session: &mut DialogueSession,
    context: &dyn DialogueContext,
    locale_provider: &dyn LocaleProvider,
) -> Result<DialogueEvent, DialogueError>;

pub fn choose(
    session: &mut DialogueSession,
    choice_id: ChoiceId,
    context: &dyn DialogueContext,
    locale_provider: &dyn LocaleProvider,
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

The concrete API may differ, but the semantics must hold. `None` starts a
source-text-only session. The runtime may still receive a locale provider for a
caller that can localise dialogue, but it must bypass that provider entirely
when the session locale is `None` and emit the compiled source text. A provider
must not be required to represent an absent locale or infer one from the host
environment.

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
    pub plural: Option<DialoguePlural>,
    pub pending_deferred_effects: Vec<DialogueEffectRequest>,
}
```

`text` is the resolved localized text.

`source_text` is the selected decoded source form, retained for diagnostics,
fallback, tests, and gettext semantics. For a plural line, the optional
`DialoguePlural.singular_source_text` and
`DialoguePlural.plural_source_text` fields are the authored/raw source forms;
they are not localized templates and are not replaced by the selected decoded
compatibility form in `DialogueLine.source_text`.

### 8.5 Choice Model

```rust
pub struct DialogueChoice {
    pub id: ChoiceId,
    pub source_text: String,
    pub text: String,
    pub metadata: Vec<MetadataEntry>,
    pub availability: ChoiceAvailability,
    pub echo: ChoiceEchoMode,
}

pub struct ChoiceAvailability {
    pub is_available: bool,
    pub primary_reason: Option<AvailabilityReasonLeaf>,
    pub reason_tree: Option<AvailabilityReasonTree>,
}

pub enum AvailabilityReasonTree {
    All(Vec<AvailabilityReasonTree>),
    Any(Vec<AvailabilityReasonTree>),
    Leaf(AvailabilityReasonLeaf),
}

pub struct AvailabilityReasonLeaf {
    pub reason_id: Option<AvailabilityReasonId>,
    pub template_source_text: Option<String>,
    pub localized_text: Option<String>,
    pub args: Vec<AvailabilityReasonArg>,
    pub origin: AvailabilityReasonOrigin,
}

pub enum AvailabilityReasonOrigin {
    ConditionCall {
        function: String,
        args: Vec<Value>,
    },
    RequirementExpression {
        source: String,
    },
}

pub enum ChoiceEchoMode {
    None,
    SelectedText,
    ExplicitLine(LineId),
}
```

Selection should prefer `ChoiceId` over index. Adapters may expose index-based APIs for engine ergonomics, but the core runtime should preserve stable choice identity.

`availability.primary_reason` is present only when an explicit choice-level
`reason=...` override applies. Tooling and adapters may derive compact display
reasons from `reason_tree`, but that presentation choice is outside runtime
conformance output. `availability.reason_tree` is present only for unavailable
choices when the compiler and schema can resolve detailed structured reason
data. A v1 API must not expose only a flat `Option<String>` reason.

### 8.6 Session State

`DialogueSession` must serialise enough information to resume exactly:

- compiled asset identity/version;
- canonical fingerprint of the complete compiled asset payload;
- current block;
- statement pointer;
- call/divert stack if applicable;
- collected deferred effects;
- pending blocking effect;
- previous prompt choices;
- optional dialogue locale, serialized as `None` in source-text-only mode;
- deterministic trace counters;
- selected choice history.

The session must not serialise game state.

Live sessions represent running, awaiting a choice, awaiting a blocking effect,
and ended as exclusive states. The versioned snapshot keeps its existing fields;
restore validates them and converts them into one live state.

Runtime session snapshots use an explicit format version. The initial v1 stores the
canonical compiled payload fingerprint so restoring against an asset with the
same header and source metadata but different semantic tables is rejected.
Preview snapshot envelopes also use their initial v1 format. Before publication,
development snapshots may be regenerated as these contracts are completed;
they do not require compatibility aliases or migration readers. Unknown versions
and snapshots missing the required payload identity are rejected.

Compilation and asset decoding prepare the canonical payload fingerprint once.
Starting another session from that asset reuses the prepared identity; it must
not serialize and hash the whole asset again. Prepared assets expose no mutable
payload access. Deliberate edits consume the asset into a raw payload and
require construction of a new asset.
Advance and choice operations compare that cached identity against the session's
identity before they inspect executable tables. A changed payload with unchanged
header and source metadata is rejected with a structured content mismatch.

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
- Recite-owned UI text across the CLI/TUI, standalone GUI, LSP, and editor
  extensions, owned by one canonical shared Fluent resource set.

Dialogue content uses the gettext/POT and PO workflow in this section. Every
Recite-owned UI string—CLI/TUI helper text, GUI labels and status, LSP messages,
and editor-extension text—must use the shared Fluent resource contract so
variables, future plural/select rules, and deterministic fallback behavior are
available in every client. The shared set need not become a new crate before
the ownership boundary is proven. It includes stable resource IDs, English
source resources, extraction, and completeness checks across every client;
generated host-specific projections are allowed where a host manifest or
metadata surface cannot consume Fluent directly. Host-required metadata remains
owned by that host and is distinct from Recite-owned strings. Hard-coded UI
strings are not a second path. Published non-English UI locales require human
authorship and review; machine-generated translations are not supported locale
claims. Fluent UI resources must not substitute for translated dialogue text,
which remains on the explicit runtime/provider path.

Dialogue localisation is an opt-in project capability, distinct from the
mandatory localisation of Recite-owned authoring text. A project may remain
source-text-only: when no dialogue locale is supplied, the CLI's
`--dialogue-locale` remains unset (`Option<String>`), the runtime session and
its serialized locale field are unset (`None`), and source text is delivered
without preview translation. A project that enables dialogue localisation must declare its
default locale and fallback locale/catalog policy at its project or fixture
configuration boundary; neither mode may infer a dialogue locale from the host
environment. This does not make `--dialogue-locale` mandatory for source-only
play or preview.

### 9.1 Requirements

The project must support gettext/POT workflows as a first-class path.

The standalone GUI must provide gettext PO catalogue editing as the required
v1 editable dialogue-catalogue path. It must preserve comments, context,
unknown fields, stable IDs, placeholders, and markup, and write changes through
safe atomic replacement. Other catalogue formats are explicitly read-only or
import/export-only in v1; they must not be presented as editable authoring
surfaces. PO editing must remain separate from the Fluent resource contract for
Recite-owned UI text.

Localisable strings:

- line text;
- choice text;
- availability reason templates;
- presentation projection label templates;
- speaker display names;
- optional project-defined localisable metadata values.

Each localisable string must have:

- stable ID;
- source text;
- source location;
- translator comments;
- block/scene context where available.

### 9.2 POT Extraction

The CLI must emit POT files. The compiler extracts and validates entries;
`recite-core::po` owns POT values and serialization alongside lossless PO
editing, so both use the same gettext escaping rules.

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

Speaker names must be extracted separately:

```po
msgctxt "dialogue_speaker:rhea"
msgid "Rhea"
msgstr ""
```

Availability reason templates are extracted by stable schema reason ID:

```po
msgctxt "availability_reason:trust_too_low"
msgid "{subject} does not trust {target} enough."
msgstr ""
```

Availability reason placeholders follow the same placeholder syntax as line
interpolation (§5.10). Translation validation must reject missing, renamed, or
extra placeholders relative to the source template. Runtime reason localisation
first resolves the template by `availability_reason:<id>`, then renders the
template with the structured `AvailabilityReasonArg` values recorded on the
reason leaf. `localized_text` on a reason leaf is the rendered display string;
the localized template and source template remain available through the reason
ID and `template_source_text` for trace/debug output.

Presentation projection label templates are extracted by stable schema template
ID:

```po
msgctxt "presentation_label:skill_check_prefix"
msgid "[{skill} {current}/{threshold}]"
msgstr ""
```

Projection label placeholders follow the same placeholder syntax and validation
rules as availability reason placeholders. Runtime or adapter projection first
resolves the template by `presentation_label:<id>`, then renders it with the
structured fields declared by the projector output. Cross-adapter conformance
output must preserve the template ID, source template, localized text when
resolved, and bound structured fields.

Reason parameters with registry-backed IDs render as stable symbols in v1.
Localized display names for registry values require a future self-contained
compiled/localisation contract and must not be fetched from game code or
adapter registries during traversal.

### 9.3 Locale Provider

The runtime locale provider must receive both stable ID and source text.

The runtime calls the provider only when the session has an explicit dialogue
locale. Source-text-only sessions have no locale to pass to `lookup`; they
bypass the provider and use the source text directly.

```rust
pub trait LocaleProvider {
    fn lookup(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<Option<String>, LocaleError>;
}
```

This supports gettext-style lookup where `msgctxt` is the stable ID and `msgid` is the source text. The `variant` parameter carries the explicit selection from the caller (see §9.5).

### 9.4 Fallback

If no translation is found for the requested locale, the locale provider must attempt broader locales via BCP-47 region truncation before falling back to source text. Example: a lookup for `pt-BR` falls back to `pt`, then to `msgid`.

The chain is the responsibility of the locale provider implementation. The spec requires:

- The terminal fallback is always the source text (`msgid`, or for plural lines `msgid` / `msgid_plural` selected by the English rule `n == 1`).
- Each step in the chain — including the terminal source fallback — must be observable in diagnostics or trace mode so missing translations and unintended fallbacks can be caught in tests.
- Fallback resolution must be deterministic for a given `(id, source, locale, variant, count)` tuple.

The runtime never invents broader locales beyond BCP-47 truncation. Cross-locale fallback (e.g., `nb` → `nn`) is the caller's job, configured outside the provider.

### 9.5 Grammatical Variants

IDs may support variant suffixes:

```text
8f1c2d3e4a5b6c708192&formal
8f1c2d3e4a5b6c708192
```

Lookup priority:

1. `id&suffix`;
2. `id`;
3. source text.

Variant selection must be explicit and deterministic. The caller selects a variant either via a session-level setter (`session.set_variant("formal")`) or via a per-call override threaded through `next` / `choose`. The runtime never infers a variant. Lookup priority remains `id&variant` → `id` → source text.

Variants are recite's mechanism for grammatical or register selection (formal/informal, masculine/feminine, polite/casual). They deliberately do not overload `msgctxt` semantically; `msgctxt` carries the full `id&variant` string and remains the stable lookup key. Counts (plural forms, §9.7) are a separate axis resolved by the locale's validated gettext rule, not by variant lookup.

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

The number of `msgstr[N]` arms per locale is determined by the locale's `nplurals` header in the `.po` file. Translators use standard po editors (poedit, weblate, crowdin) without recite-specific tooling.

POT is a locale-neutral template: plural entries always contain exactly two
empty `msgstr` arms for extraction and deliberately do not contain a
`Plural-Forms` header. A translated PO catalogue must carry its own validated
locale header and the number of arms declared there. Editing a POT and loading
the resulting translated PO are separate operations; the latter cannot borrow
plural metadata from another catalogue or from the source language.

The locale provider must expose one structured plural resolution alongside the
singular lookup:

```rust
pub trait LocaleProvider {
    fn lookup(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<Option<String>, LocaleError>;

    fn resolve_plural(
        &self,
        id: &str,
        source_singular: &str,
        source_plural: &str,
        count: i64,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<PluralResolution, LocaleError>;
}
```

`PluralResolution` carries the optional translated template, matched locale,
context, and source key, matched arm, and deterministic candidate attempts. An
attempt records its candidate locale, context, source key, arm selected by
that candidate's validated header, and outcome. A provider-selected arm is
authoritative only when that exact catalogue entry supplied the translated
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

The provider and catalogue loader share one bounded gettext expression parser
and evaluator. A catalogue's validated `Plural-Forms` expression is the only
source of locale-specific arm selection; clients do not embed separate
locale-specific evaluators.
