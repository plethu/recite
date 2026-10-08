# Migration, scope and release acceptance

Part of the [production specification](../recite-production-spec.md). These are requirements;
implementation and release readiness require evidence from code, tests and the current GitHub
milestone. Section numbers remain stable.

## 20. Import and Migration

Full ink/Yarn/Clyde import compatibility is not a v1 goal. Importers exist to help teams inspect and
migrate existing content, not to make Recite execute another tool's runtime model. Clyde is a
guidance-only comparison target for v1; no Clyde importer or compatibility runtime is promised.

Importer design must follow these boundaries:

- importers produce ordinary Recite source and structured reports, not a second compiled format or
  compatibility runtime;
- generated source is validated by the normal Recite parser, compiler, schema, ID, localisation, and
  effect checks;
- native Recite syntax, runtime semantics, schema rules, and stable-ID policy do not change to
  preserve compatibility with an imported source format;
- unsupported constructs are reported explicitly instead of being silently dropped;
- lossy conversions are reported even when usable Recite source can be emitted;
- source provenance is preserved where practical so authors can review the original construct that
  produced each generated line, choice, effect, or report item.

The useful v1 migration surface is a best-effort assistant with honest limits:

- convert recognizable line forms such as `Speaker: text #id:x #portrait:y` into structured line
  records with speaker and metadata where the source format exposes that information;
- convert simple choices such as `+ Choice #id:x` or link-style options into structured choice
  records;
- convert direct jumps, diverts, or passage links into Recite block references when the target is
  statically identifiable;
- convert tags, headers, passage metadata, or export fields into ordered metadata entries;
- map external calls, commands, or engine hooks to typed effect declarations only when the user
  supplies an explicit schema-backed mapping;
- preserve source IDs when they already exist and are valid Recite IDs, and otherwise record an
  old-to-new ID mapping for review.

Importer output should include:

- generated `.recite` source files that are meant to be edited after import;
- a machine-readable import report containing diagnostics, skipped constructs, lossy conversions,
  generated IDs, old-to-new ID mappings, and unmapped source fields;
- human-readable summary counts by source file and construct type;
- provenance references for generated records and report items.

The report should use the same diagnostic shape as §18. Importer diagnostic codes should use a
dedicated namespace such as `RECITE_IMPORT001`. Each report item should include severity, source
family, source file, source location when available, source construct type, action taken, and help
text for the expected manual follow-up.

Source-location preservation is best-effort:

- plain text formats should report file, line, column, and end position where the importer parser
  can recover them;
- CSV imports should report row, column, and header name;
- JSON imports should report a JSON Pointer path and, when supported by the parser used by the
  implementation, byte or line/column ranges;
- if a source format does not expose stable spans, the report should still carry a stable record key
  such as node name, passage title, row number, or object ID.

Supported migration paths by source family:

- **ink**: importers may map knots, stitches, plain lines, simple choices, static diverts, tags, and
  explicitly mapped external function calls. They must report variables, tunnels, threads,
  glue/weave behavior, list operations, sequence/shuffle behavior, arithmetic, complex expressions,
  and runtime control flow as unsupported or lossy unless a later implementation issue defines a
  narrower safe subset.
- **Yarn Spinner**: importers may map nodes, dialogue lines, speaker prefixes, options, tags,
  headers, simple jumps, and explicitly mapped commands. They must report variable storage,
  expression semantics, command side effects, shortcuts with unsupported conditions, localization
  metadata that cannot be preserved, and runtime-specific behavior as manual migration work.
- **Twee/Twine-style source**: importers may map passages to blocks, passage text to lines, simple
  links to choices or diverts, and passage tags to metadata. They must report macros, widgets,
  JavaScript/CSS, story-format behavior, global state, and conditional/link syntax that has no
  direct Recite equivalent.
- **custom JSON/CSV exports**: importers may map conventional fields such as ID, speaker, text,
  choice text, target, condition, effect, and metadata when the mapping is explicit. They must
  report ambiguous nesting, multiple possible targets, embedded scripts, unmapped columns or object
  fields, and rows or objects that cannot produce valid Recite statements.

## 21. Non-Goals

Initial non-goals:

- executing game state mutations inside the runtime;
- embedding variable storage in the dialogue runtime;
- implementing a full scripting language;
- hidden arbitrary code execution;
- result-dependent branching from blocking effects;
- a full CLDR pluralization engine beyond the bounded gettext `Plural-Forms` rules;
- a fully general visual node editor or graph-first authoring format for v1;
- generated host-language bindings unless a later decision promotes them;
- tying the authoring model to one engine's scripting language;
- engine adapters that move game logic into the Recite runtime;
- network/cloud collaboration;
- AI-authored dialogue features;
- automatic ID renaming based on content changes;
- implicit localization variant selection by the runtime;
- `:elif` / `else if` sugar (deferred until real authoring pain is reported; nested `:else` + `:if`
  and `:match` cover the use cases);
- general pattern matching beyond schema-declared enum dispatch (no destructuring, no tuples, no
  guards);
- a compatibility runtime for ink, Yarn Spinner, Clyde, Twee/Twine story formats, or engine-specific
  dialogue plugins;
- importer behavior that requires Recite to adopt another tool's syntax, variable model, expression
  language, runtime side effects, or localization pipeline;
- silent migration of unsupported source constructs.

## 22. Recommended Milestones

These are durable release outcomes. GitHub owns their task state and scheduling; completion requires
the relevant exit evidence, not a checked box or historical issue count. §23 is the final release
gate.

### Milestone 1: Product Foundation and Maintainability

Ownership of syntax, semantics, schemas, wire data, snapshots, diagnostics, FFI and adapters is
explicit. Versioned quality checks, compatibility fixtures and the complete local gate provide the
baseline. File-size thresholds are review triggers, not automatic split rules.

### Milestone 2: Language, Schema, and Localisation Readiness

Representative projects validate and compile deterministically. Stable IDs, schema producers,
source-only mode, explicit locale/fallback policies, extraction, placeholders and markup have
executable coverage. Generated manifests remain read-only; unsupported producers are identified as
such. Default `en-US` Fluent resources cover Recite-owned UI text independently of dialogue locales.

### Milestone 3: Shared Authoring Kernel and Preview

CLI, LSP and Writer share project discovery, source-preserving edits, diagnostics, configuration,
schema/catalogue state, freshness and deterministic preview. Schema actions edit source or invoke
its producer; preview never executes game-side effects.

### Milestone 4: Editor Integration Parity

VS Code/VSCodium, Neovim and Zed have tested setup and parity for their declared LSP and command
capabilities. Shared fixtures cover malformed content, schema, IDs, localisation and UTF-16. Host
limitations and missing platform evidence remain visible.

### Milestone 5: Native GUI Strategy and Accessibility Proof

Freya is selected. Acceptance still requires evidence for every claimed platform: keyboard, focus,
screen readers, IME, BiDi/RTL, scaling, contrast, non-colour paths, progress, cancellation, retry,
conflicts, reduced motion, startup, memory and packaging. Framework selection alone does not satisfy
this outcome; [Writer acceptance](../../apps/writer/acceptance.md) records the checks and limits.

### Milestone 6: GUI Workbench

The source-first workbench supports the complete authoring loop through the shared kernel, including
undo/recovery, safe saves, standalone schema source editing, engine-producer actions, lossless PO
editing and preview. Graph navigation has an equivalent accessible list/outline. Layout never
becomes dialogue semantics. General graph-format authoring and generated bindings are not required.

### Milestone 7: Engine Companions

Godot, Bevy and Unity meet the shared adapter contract through their real host surfaces. Evidence
covers loading, identities, conditions, effects, snapshots, localisation, errors and changed-asset
behavior. Native producers and package paths are documented without creating alternate runtimes.

### Milestone 8: Distribution, Adoption, and Migration

A new team can install, upgrade and evaluate the declared packages using reproducible instructions.
Bounded importers preserve provenance and report losses; migration guides distinguish supported
conversion from manual work. Signing and support claims match the shipped artifacts and platforms.

### Milestone 9: Serious v1 Release

The preceding outcomes and §23 pass. Compatibility boundaries are frozen for the release, artifacts
install and run, required review is resolved, and the release candidate is reproducible from its
stated toolchain. Performance and human acceptance evidence name the tested profiles and limits.

## 23. Acceptance Criteria for a Serious v1

Developer previews use numbered SemVer prereleases; a candidate's prepared component versions,
source commit and verified artifacts form one immutable release identity. Stable promotion changes
that identity and requires fresh validation.
[The contributor release workflow](../../CONTRIBUTING.md#preparing-and-publishing-releases) owns
preparation, tags and publication. Preview publication does not imply the acceptance criteria below
have passed.

The project is not production-credible until all of the following are true:

- A dialogue scene can be compiled, validated, run, snapshotted, and replayed headlessly in
  source-only mode or with an explicitly configured locale; enabled localisation has tested default
  and fallback locale/catalog policy.
- All runtime outputs are structured and deterministic.
- Effects are schema-checked and never executed by the runtime.
- Blocking effects can pause and resume across save/load, including re-emission of the pending
  effect with the same `EffectRequestId`.
- Choice IDs are stable and selection by ID is supported.
- Stable anchors survive prose and label edits; ID insertion is an explicit, guarded authoring
  action. Changed prose still requires translation review.
- Metadata supports repeated ordered keys.
- Inline markup is preserved and validated.
- POT extraction produces translator-usable context.
- The LSP catches common mistakes before runtime, including guarded actions to fill missing IDs.
  Clients may invoke those actions on save.
- CI can verify compiled assets are fresh relative to source and schema.
- The [serialization compatibility decision](../serialization-compatibility.md) is accepted, and the
  compiled asset, snapshot, FFI, and condition-payload migration boundaries are documented per
  artifact.
- The CLI, LSP, editor integrations, standalone GUI, and preview surface use the shared authoring
  kernel for project discovery, diagnostics, edits, schema/localisation state, and freshness; they
  do not maintain competing semantic models.
- VS Code/VSCodium, Neovim, and Zed are first-class supported text-authoring surfaces with tested
  setup and parity for the declared LSP and command workflow.
- A standalone, source-first GUI workbench supports source editing, stable-ID actions, diagnostics,
  source-owning editing for the standalone declarative schema producer, producer-backed schema
  edit/open-declaration actions, schema inspection and stale/regeneration reporting, required
  gettext PO catalogue editing with comments/context/unknown-data preservation and safe atomic
  writes, and deterministic preview. Generated manifests are never edited directly; unsupported
  schema producers are explicitly read-only.
- The standalone workbench handles stale generations and cancellation, announces progress and
  status, supports failure/retry, retains or restores focus, handles external-file and save
  conflicts, supports reduced motion, and has manual assistive-technology verification where
  automation is insufficient. It also provides keyboard, screen-reader, IME, BiDi/RTL,
  zoom/text-scaling, high-contrast, and non-colour paths for every essential operation, with an
  equivalent accessible list/outline for graph navigation.
- The native GUI strategy has a checked-in bake-off decision and accessibility evidence for every
  declared platform, including its fallback path and known maintenance limits.
- Linux, Windows, and macOS are first-class desktop platforms for the CLI, LSP, editor integrations,
  and standalone workbench. Engine companions may publish narrower engine/platform matrices.
- Authors have a fast documented loop from source edit to LSP diagnostics, explicit stable-ID
  insertion, `recite watch` rebuild, engine adapter import/refresh, and scene restart or documented
  active-session behavior.
- Shared authoring configuration resolves an explicit `$RECITE_CONFIG` override or the platform
  strategy location, stays separate from project and generated files, and behaves consistently on
  Linux, macOS, and Windows.
- Large-project fixtures exercise compile, validate, run, trace, localisation extraction, and
  snapshot restore at narrative scale comparable to serious dialogue-heavy games.
- Performance and memory characteristics for authoring, preview, GUI startup, editing, and runtime
  are measured on named profiles, documented, and protected by regression smoke checks.
- Godot, Bevy, and Unity adapters can load compiled assets, traverse dialogue, evaluate conditions,
  emit effects without executing them, and participate in save/load workflows.
- Each v1 adapter has a documented asset refresh/import workflow, an explicit active-session
  behavior for changed compiled assets, and coverage against the shared adapter conformance
  contract.
- The adapter contract is stable enough that additional engines can be implemented without changing
  core runtime semantics.
- Public docs and examples demonstrate headless CLI workflows and real Godot, Bevy, and Unity
  integration paths.
- Adoption and migration guidance makes a credible case for teams evaluating Recite against
  established tools such as Dialogue System for Unity, Dialogue Manager, Dialogic, Clyde, Yarn
  Spinner, and Ink; Clyde is explicitly guidance-only and has no v1 importer or compatibility
  runtime.

Shipping a credible v1 means more than proving the core can run headlessly. The core runtime, shared
authoring kernel, CLI, first-class text editors, accessible source-first GUI, schema/localisation
workflow, scale proof, adapter contract, Godot/Bevy/Unity companion paths, and adoption
documentation must work together well enough for a serious narrative-heavy game team to evaluate
Recite as a practical replacement for established dialogue tooling. A fully general visual node
editor and generated host bindings remain post-v1 options unless a later decision explicitly
promotes them.

## 24. Design Summary

The [product contract](product.md) owns Recite's purpose, glossary and invariants. Serious-v1
acceptance covers the complete authoring-to-preview-to-engine workflow in §23, rather than a second
summary checklist.
