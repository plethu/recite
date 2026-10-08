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

The likely implementation order is:

1. define the shared import report, provenance model, diagnostic namespace, and fixture
   expectations;
2. prototype a custom JSON/CSV importer because it validates mappings, reporting, and span fallbacks
   without inheriting another language's runtime semantics;
3. prototype a small Twee/Twine-style subset because passages and links map cleanly to blocks and
   choices;
4. add ink and Yarn Spinner inspection or subset importers only after the report model has proven
   useful for skipped and lossy constructs. Clyde remains on the compatibility-guidance path rather
   than becoming another importer family.

Importer follow-up issues should stay separate from the native language design. The branchable work
units are: shared import report/provenance model, custom JSON/CSV importer prototype, Twee/Twine
subset importer prototype, ink inspection or subset importer, Yarn Spinner inspection or subset
importer, and compatibility notes that document what must be migrated manually.

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

These are outcome milestones rather than a list of isolated implementation issues. The tracker may
split or reorder work inside a milestone, but a milestone is not complete until its exit evidence
exists. The dependency order is: 1 -> 2 -> 3; milestones 4, 5, and 7 can then proceed in parallel
from the milestone-2/3 contracts and fixtures; 5 -> 6; and 4, 6, and 7 -> 8 -> 9. Milestone 1
supplies maintainability and verification gates throughout. The current tracker milestones 17
through 25 correspond to these nine outcomes; issue links remain secondary to this specification.

### Milestone 1: Product Foundation and Maintainability

**Outcome:** the project has explicit ownership boundaries and a dependable verification baseline.

**Entry gate:** the current parser, compiler, runtime, CLI, LSP, schema, FFI, benchmark, and adapter
surfaces have been inventoried.

**Exit gate:** syntax, semantic lowering, schema, compiled wire data, snapshots, diagnostics, FFI,
and adapters have explicit owners. Structural ast-grep gates cover repeated equality cascades and
private-test placement. The maintainability gate compares handwritten file sizes against the change
base: unchanged or shrinking debt passes, while growth above 400 production/tooling lines or 500
test/support lines requires an exact, issue-linked, bounded exception in
`scripts/maintainability/exceptions.toml`. Generated paths are explicit exclusions. Compatibility
fixtures and the complete local gate remain the acceptance checks. Line count is a review trigger,
not a split rule.

### Milestone 2: Language, Schema, and Localisation Readiness

**Outcome:** authors can write stable, schema-checked, localisable source while the implementation
remains ready for more than one locale.

**Entry gate:** the foundation audit names the source, schema, compiler, and stable-ID authorities.

**Exit gate:** representative projects compile and validate deterministically; the source-owning
schema-authoring capability and fixtures define at least one source-owning, kernel-editable
declarative producer path suitable for GUI integration for standalone projects and producer-backed
edit/open-declaration actions for engine-owned schemas; generated manifests remain read-only and
unsupported producers are explicitly read-only. The shipped GUI realization is gated by milestone 6.
Schema manifests have producer provenance and stale/regeneration behavior; source IDs, extraction,
PO catalogue lookup, fallback, placeholders, markup, and translator context are tested. Dialogue
localisation is optional: source-only projects retain an unset locale, while projects that enable it
declare and test their default and fallback locale/catalog policy. Recite-owned authoring text still
requires complete Fluent-backed default `en-US` resources. English-only launch is explicit rather
than an architectural assumption, and exact standalone schema-source syntax follows the TOML
contract in §10.2.1.

### Milestone 3: Shared Authoring Kernel and Preview

**Outcome:** every authoring surface consumes one project/index/edit/diagnostic/ preview model.

**Entry gate:** source, schema, localisation, and runtime contracts from milestone 2 are stable
enough to expose operations.

**Exit gate:** a shared kernel provides project discovery, source-preserving edit transactions,
source-owning schema edits, and producer-backed declaration actions that open source,
invoke/regenerate through the producer, report stale output, and return structured failure/retry
outcomes; structured diagnostics/completion/navigation, schema and catalogue summaries,
cross-platform user configuration, typed watch/build freshness state, and deterministic
preview/traces. CLI, LSP, and fixtures use those operations; preview never executes game-side
effects, and generated manifests are never written by the kernel.

### Milestone 4: Editor Integration Parity

**Outcome:** VS Code/VSCodium, Neovim, and Zed are first-class text-authoring surfaces rather than
syntax-only examples.

**Entry gate:** the shared kernel and parity fixture set exist.

**Exit gate:** each editor has tested setup, highlighting/file detection, LSP diagnostics,
completion, hover, navigation, rename, code actions, and command integration for the supported
workflow. The same malformed, schema, ID, localisation, and UTF-16 fixtures produce equivalent
semantic answers.

### Milestone 5: Native GUI Strategy and Accessibility Proof

**Maintainer decision:** Freya is selected. Further candidate implementation, including the unrun
platform-native lanes, is parked. Accessibility and declared-platform acceptance remain outstanding.
Early workbench implementation is authorized alongside those checks, without claiming milestone
completion.

**Outcome:** the standalone workbench strategy is selected from comparable authoring and
accessibility evidence.

**Entry gate:** the kernel, reusable editor-parity fixtures, and preview loop can be reused without
reimplementing language semantics in each candidate; completed editor clients are not a
prerequisite.

**Exit gate:** the selected frontend has evidence for every claimed platform. The frontend
assessment names support, dependencies, maintenance burden, known limits, and reconsideration
triggers. Keyboard-only, focus, screen-reader, IME, BiDi/RTL, zoom/text scaling, high-contrast,
non-colour, stale-generation, cancellation, progress/status announcement, failure/retry, focus
retention/restoration, external-file/save conflict, reduced-motion, startup, memory, and packaging
evidence exists for every declared platform.

### Milestone 6: GUI Workbench

**Outcome:** writers can use an accessible standalone source-first workbench for the complete v1
authoring loop.

**Entry gate:** the strategy decision and accessibility proof are accepted.

**Exit gate:** project open, source editing, search/outline/graph navigation, diagnostics,
completion, stable-ID actions, undo/redo, atomic save and conflict handling, source-owning
standalone schema editing, producer-backed schema-declaration actions, schema
inspection/provenance/staleness actions, required gettext PO catalogue editing with safe atomic
writes, and deterministic preview are all available through the kernel. Graphs have an equivalent
list/outline path, with automatic layout by default, transient/local viewport state, and optional
stable-ID open sidecar state only. Full general visual node authoring and generated bindings are not
required for this exit.

### Milestone 7: Engine Companions

**Outcome:** Godot, Unity, and Bevy companions integrate with the same compiled asset, schema,
localisation, and refresh contracts without becoming alternate authoring runtimes.

**Entry gate:** adapter, schema, localisation, and FFI contracts plus kernel preview/runtime
semantics are stable. The GUI is not a prerequisite.

**Exit gate:** each companion has conformance and integration coverage for loading, stable
identities, conditions, typed effects, save/load, localisation, errors, asset freshness,
import/refresh, and changed-asset session behavior. Host-native schema producers and package paths
are documented; companions stay thin and the runtime remains game-side-effect free.

### Milestone 8: Distribution, Adoption, and Migration

**Outcome:** a new team can install, learn, evaluate, and migrate toward Recite with honest
boundaries.

**Entry gate:** the authoring loop and companion conformance contract are stable.

**Exit gate:** CLI/LSP/GUI/editor integrations and companion artifacts have reproducible package,
upgrade, signing, and support instructions; examples and guides cover the source-first loop; bounded
subset importers for custom JSON/CSV, Twee/Twine, Ink, and Yarn Spinner preserve provenance and
report losses; Clyde, Dialogic, Dialogue Manager, Dialogue System for Unity, and related tools
receive compatibility and migration guidance under the import-report work without an unowned
importer promise; known limits and alternatives are published without implying compatibility that
does not exist.

### Milestone 9: Serious v1 Release

**Outcome:** Recite can make a bounded, supportable compatibility promise.

**Entry gate:** milestones 1–8 have passed, the compiled format and snapshot policy are frozen for
the release, and all required reviews are resolved.

**Exit gate:** §23 passes; all declared platforms have Rust, CLI, LSP, editor-integration, GUI, and
companion verification; accessibility, scale, memory, preview, watch, and adapter evidence has named
profiles and regression policies; release artifacts install and run; known limits, migration
boundaries, and active-session rules are published; and a clean release candidate is reproducible
from the stated toolchain.

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
- Stable IDs survive author edits to source text. Renames happen only via the explicit code action.
- Metadata supports repeated ordered keys.
- Inline markup is preserved and validated.
- POT extraction produces translator-usable context.
- The LSP catches common mistakes before runtime, including auto-filling missing IDs on save.
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
- Authors have a fast documented loop from source edit to LSP diagnostics, on-save stable ID
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

Recite's core value is a deterministic dialogue/effect protocol with a humane, inspectable authoring
workflow:

- authored in a small domain language that names narrative structure directly;
- validated before runtime;
- compiled into deterministic assets;
- run as a pure state machine;
- integrated with games through explicit typed effects;
- authored through one shared kernel by CLI, LSP, VS Code/VSCodium, Neovim, Zed, and an accessible
  standalone source-first GUI;
- ready for schema-backed localisation even when the first release ships English-only content;
- previewed through the same deterministic runtime loop before an engine is involved;
- accompanied by thin Godot, Unity, and Bevy integrations rather than parallel semantic runtimes;
- tested with normal programmatic assertions.

The source format is small. It is a way to describe dialogue structure, not a second general-purpose
scripting layer. Conditions are pure queries. Effects are typed requests. Game logic stays in the
game. The GUI is source-first and never becomes a second source of semantic truth; a general visual
node editor and generated host bindings are deliberately left for post-v1 evaluation.

The native GUI strategy is a product decision earned through comparable authoring, accessibility,
performance, packaging, and maintenance evidence. Cross-platform user configuration is explicit and
separate from project content. The release promise is therefore about a complete, reproducible
authoring-to-preview-to-engine loop, not just a runtime library.

That is the standard the project should optimise for.
