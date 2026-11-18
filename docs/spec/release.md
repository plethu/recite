# Migration, scope and release acceptance

Part of the [production specification](../recite-production-spec.md). GitHub owns task state; these
sections define release outcomes and the evidence required to claim them.

## 20. Import and Migration

Importers produce editable Recite source and structured diagnostic reports. They must preserve
available provenance and ID mappings, report unsupported or lossy constructs, and validate output
through the normal parser, compiler, schema, ID, localisation and effect checks. Native syntax,
semantics and stable-ID policy never change to accommodate another tool.

Imports do not execute source-language commands or emulate its runtime. External calls require an
explicit schema-backed mapping; unmapped conditions and effects need manual migration. Full Ink/Yarn
compatibility is outside v1, and Clyde has guidance only.

The [import inspection guide](../../docs-site/src/content/docs/migration/importer-boundaries.md)
owns report interpretation, source locations, supported subsets and adoption steps. The
[import model](../../crates/recite-import/src/model.rs),
[readers](../../crates/recite-import/src/lib.rs) and [fixtures](../../fixtures/import/) own
executable mappings and losses.

## 21. Non-Goals

v1 excludes:

- runtime-owned game mutations, variable storage, arbitrary code execution or a full scripting
  language;
- result-dependent branching from blocking effects;
- a full CLDR plural engine beyond bounded gettext `Plural-Forms` rules;
- a general visual-node or graph-first authoring format;
- generated host-language bindings unless a later decision promotes them;
- an authoring model tied to one engine's scripting language;
- network/cloud collaboration and AI-authored dialogue;
- automatic content-based ID renaming or implicit localisation variant selection;
- `:elif` / `else if` sugar until real authoring pain is reported; nested `:else` + `:if` and
  `:match` cover the current cases;
- pattern matching beyond schema-declared enum dispatch: no destructuring, tuples or guards; and
- compatibility runtimes, imported variable/expression/localisation models, or silent losses in the
  migration paths described in §20.

## 22. Recommended Milestones

GitHub owns scheduling and completion state. Each outcome requires its exit evidence; §23 governs
the final release. Milestones 5 and 6 gate Writer's own stable release, not serious v1.

### Milestone 1: Product Foundation and Maintainability

Explicit subsystem ownership, versioned quality checks and compatibility fixtures pass the
[complete gate](../../CONTRIBUTING.md#maintainer-setup). File-size thresholds trigger review, not
automatic splits.

### Milestone 2: Language, Schema, and Localisation Readiness

Representative projects exercise the source, schema and localisation contracts. Default `en-US`
Fluent resources cover Recite-owned UI independently of dialogue locales.

### Milestone 3: Shared Authoring Kernel and Preview

CLI, LSP and Writer share discovery, edits, diagnostics, configuration, schema/catalogue state,
freshness and deterministic preview. Preview never executes game effects.

### Milestone 4: Editor Integration Parity

Declared VS Code/VSCodium, Neovim and Zed capabilities pass the
[parity contract](../editor-parity-contract.md), with host and platform limits visible.

### Milestone 5: Native GUI Strategy and Accessibility Proof

Xilem is selected and Freya is retired; platform acceptance still needs the native checks tracked in
issue #237. Selection alone is insufficient.

### Milestone 6: GUI Workbench

The source-first workbench completes the [authoring contract](tooling.md#154-gui-workbench),
including safe saves/recovery, schema producer actions, lossless PO editing and preview.

### Milestone 7: Engine Companions

Godot, Bevy and Unity meet the [adapter contract](../engine-adapter-contract.md) through real hosts,
with native producers and package paths.

### Milestone 8: Distribution, Adoption, and Migration

A new team can install, upgrade and evaluate declared packages. Migration limits are explicit;
signing and support claims match shipped artifacts and platforms.

### Milestone 9: Serious v1 Release

§23 passes, compatibility boundaries are frozen, required review is resolved, and the candidate is
reproducible from its stated toolchain. Performance and human evidence name tested profiles.

## 23. Acceptance Criteria for a Serious v1

Numbered SemVer previews do not imply serious-v1 acceptance. The
[release workflow](../../CONTRIBUTING.md#preparing-and-publishing-releases) owns immutable candidate
identity, preparation, tags and publication; stable promotion requires fresh validation.

Serious v1 covers the compiler, runtime, CLI, LSP, editor integrations and engine companions. Writer
ships beside it as a preview on its own version line until its criteria below pass; a v1 candidate
records Writer's current limits rather than waiting for them.

Published previews make one promise before 1.0: dialogue source and author-visible IDs written for a
published preview keep compiling, or the release that changes them ships an automated migration.
Compiled assets, snapshots, the Rust API, the C ABI and Writer may change between minor versions;
readers refuse unsupported versions under the
[serialization policy](../serialization-compatibility.md).

A serious-v1 candidate requires all of the following:

- The complete gate and
  [integrated workflow](../../docs-site/src/content/docs/examples/headless-cli.md) pass on the
  candidate. Representative scenes validate, compile, extract, run, trace, save and replay
  headlessly in source-only and configured-locale modes, including default/fallback policy and
  blocking-effect re-emission with the same request ID. The source/runtime contracts govern
  structured deterministic output, stable IDs, ordered metadata and markup preservation.
- `just supply-chain` passes for the root, Zed and Writer Cargo graphs: no vulnerable or yanked
  crate, unmaintained direct dependency, unapproved licence or unknown source. The release issue
  records each unmaintained transitive dependency that `just release dependency-review` reports,
  with its upstream path and fix condition; none is waived silently.
- [Serialization policy](../serialization-compatibility.md) is accepted, with compiled-asset,
  snapshot, FFI and condition-payload migration boundaries documented per artifact.
- CLI, LSP, text editors, Writer and preview use the shared authoring kernel rather than competing
  semantic models. VS Code/VSCodium, Neovim and Zed have tested setup and parity for declared
  capabilities.
- Linux, Windows and macOS are supported for the CLI, LSP and editors. Engine companions may declare
  narrower engine/platform matrices. Shared configuration resolves explicit `$RECITE_CONFIG` or its
  platform location consistently and stays separate from project content and generated files.
- Authors can follow a documented edit → diagnostics → explicit ID insertion → watch rebuild →
  engine import/refresh → restart loop, including each adapter's active-session policy.
- Narrative-scale fixtures exercise compilation, validation, traversal, traces, extraction and
  snapshot restore. Authoring, preview and runtime performance and memory are measured on named
  profiles and protected by regression smoke checks under
  [§19](quality.md#19-performance-and-benchmarks).
- Godot, Bevy and Unity pass applicable conformance through their real hosts: assets, identities,
  conditions, effect requests, saves, localisation, errors and refresh. The adapter contract
  supports additional engines without changing core traversal semantics.
- Public examples demonstrate headless CLI and real engine workflows. Adoption/migration guidance
  supports evaluation against established dialogue tools and distinguishes conversion from manual
  work; Clyde remains guidance-only.

Writer's stable release additionally requires all of the following:

- The complete [workbench contract](tooling.md#154-gui-workbench) works through source editing,
  guarded ID actions, diagnostics, standalone schema source editing, producer-backed navigation and
  regeneration, lossless gettext PO editing and preview. Generated manifests and unsupported
  producers remain read-only. Graph navigation has an accessible list/outline equivalent.
- Native evidence covers every essential operation's keyboard, focus, screen-reader, IME, BiDi/RTL,
  scaling, high-contrast and non-colour paths. It covers cancellation, stale generations,
  progress/status, failure/retry, focus restoration, external-file/save conflicts and reduced
  motion. The GUI strategy decision, fallback and maintenance limits are recorded; automation does
  not replace hands-on acceptance (#237).
- Linux, Windows and macOS are supported for the workbench, with GUI startup/editing performance and
  memory measured on named profiles.

## 24. Design Summary

The [product contract](product.md) owns purpose, glossary and invariants. §23 requires the complete
text-authoring-to-preview-to-engine workflow; headless runtime success alone does not establish v1.
