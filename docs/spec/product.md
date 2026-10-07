# Product and invariants

Part of the [production specification](../recite-production-spec.md). These are requirements;
implementation and release readiness require evidence from code, tests and the current GitHub
milestone. Section numbers remain stable.

## 1. Purpose

Recite is an open-source deterministic dialogue compiler, runtime, editor, and tooling suite for
narrative-heavy games.

Its primary audience is developers building games where dialogue must be:

- testable through programmatic fixtures and snapshot tests;
- deterministic across replay, save/load, and CI;
- integrated with explicit game state boundaries rather than ad hoc runtime callbacks;
- localisable through stable gettext-style workflows;
- portable across engine integrations without tying the dialogue model to one engine's scripting
  language;
- authorable through excellent text tooling, with a visual editor as a structured companion rather
  than the only workflow.

Recite should be a credible replacement for existing dialogue tools when their tradeoffs do not fit
a project's narrative, tooling, or architecture needs. The motivating pain points are specific:

- localisation workflows that depend on unstable text or ad hoc IDs;
- editor tooling that cannot catch enough content mistakes before runtime;
- dialogue scripts that can call directly into engine scripting or mutate game state;
- one-off authoring languages whose concepts do not travel well outside that tool;
- runtime behaviour that is difficult to replay, test, save, load, or inspect deterministically;
- asset-store or engine-specific packaging that makes the dialogue model feel less portable than the
  game needs.

This is not a claim that ink, Yarn Spinner, Godot-native tools, or other dialogue systems are bad
fits for all projects. Recite is specifically for projects that value portable narrative-system
thinking, strict architectural boundaries, reproducible execution, schema-checked integration, and
tool-assisted content validation.

## 2. Core Invariants

The following invariants define the project and must not be weakened for convenience:

1. Dialogue traversal is deterministic.
2. The runtime never performs game-side effects.
3. Game-side effects are emitted as typed, schema-checked effect requests.
4. Dialogue state is serialisable and deserialisable.
5. Dialogue files can be validated without running the game.
6. Localisable strings use stable IDs that survive nearby edits.
7. Runtime data surfaces speaker, line, choice, metadata, and effect information as structured
   values, not conventions parsed from prose.
8. Tooling is part of the product, not an optional afterthought.
9. Stable IDs are author-visible and never silently rewritten by tooling. Renames go through an
   explicit code action.

## 3. Terminology

- **Dialogue source**: Human-authored text file in the dialogue DSL.
- **Compiled dialogue asset**: Binary or structured compiled representation consumed by runtimes and
  adapters.
- **Block**: Named unit of dialogue execution, equivalent to an ink knot or Yarn node.
- **Line**: Atomic localisable dialogue/narration output.
- **Prompt**: A line, optional line, or UI state that presents choices.
- **Choice**: Player-selectable option with stable ID, localisable text, availability, metadata, and
  optional echo policy.
- **Condition**: Pure query against game state, evaluated through a caller-provided context.
- **Effect request**: Typed intent emitted by dialogue. The runtime does not execute it.
- **Deferred effect**: Effect collected and returned when the scene ends.
- **Immediate effect**: Effect yielded immediately while traversal may continue.
- **Blocking effect**: Effect yielded immediately and requiring explicit acknowledgement before
  traversal continues.
- **Metadata**: Ordered key/value annotations attached to lines, choices, blocks, or scenes.
- **Inline markup**: Markup embedded inside localisable text, such as `[slow]...[/slow]`.
- **Scene manifest**: Project-level mapping from scene IDs to dialogue assets, start blocks,
  participants, and presentation hints.

## 4. Product Shape

The reference implementation should be delivered as a Rust workspace for the language, compiler,
runtime, schema, localisation, CLI, LSP, and shared authoring kernel. Rust is the implementation
language for those core capabilities, not a requirement that users build Rust-first games.

The product also includes a standalone GUI workbench. Its frontend technology is a deliberate
decision made by the native GUI strategy and accessibility proof, not a premise of the language or
runtime. A selected strategy may use a single cross-platform frontend or platform-appropriate
frontends, but all frontends must call the same authoring kernel and preserve the same source,
schema, localisation, preview, and diagnostic semantics.

Linux, Windows, and macOS are first-class v1 desktop platforms for the core CLI, LSP, editor
integrations, and standalone workbench. Engine companions may declare narrower supported
combinations of engine version, host platform, and toolchain; v1 does not require testing every
Cartesian product of desktop and engine targets.

The workspace should contain:

- `recite-core`: AST, identifiers, value model, diagnostics, schema model.
- `recite-parser`: DSL parser and source mapping.
- `recite-compiler`: compiler, validator, POT extractor, compiled asset writer.
- `recite-runtime`: deterministic runtime with no engine dependencies.
- `recite-cli`: project CLI, exposing the `recite` binary.
- `recite-lsp`: language server.
- shared authoring-kernel and configuration capabilities used by the CLI, LSP, editor clients,
  preview, and GUI workbench. These may begin in existing crates and become a crate only when the
  ownership boundary is proven.
- engine adapter crates as integrations mature, such as `recite-godot`, `recite-bevy`, or
  `recite-unity`.
- editor integrations for VS Code/VSCodium, Neovim, and Zed.
- the standalone GUI workbench and any platform-specific frontend projects selected by the bake-off.

The GUI workbench is source-first. It may show a graph and provide safe structured edits, but
source, comments, unknown metadata, and stable IDs remain authoritative. A general lossless
visual-node authoring format is not required for v1. Generated host-language bindings are also not
part of the v1 product shape.

The shared configuration contract covers user-owned UI preferences and cross-platform paths. Project
content and generated schema manifests remain separate from those preferences. Neovim support ships
as editor-native LSP and highlighting integration, not a Rust crate.
