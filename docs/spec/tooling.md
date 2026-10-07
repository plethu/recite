# LSP, editors and engine adapters

Part of the [production specification](../recite-production-spec.md). These are requirements;
implementation and release readiness require evidence from code, tests and the current GitHub
milestone. Section numbers remain stable.

## 14. LSP

The LSP must be excellent enough that text authoring feels safe.

The server advertises incremental UTF-16 document synchronization. Full replacements remain
accepted. Ranged changes in one notification apply sequentially and atomically; invalid batches and
stale versions leave the accepted text and version unchanged. Protocol text advances before
cancellable analysis, and queued updates contain full snapshots so coalescing cannot discard a
ranged edit's dependency. Positions inside surrogate pairs and nonexistent lines are rejected;
overlong character offsets clamp to the line end. CRLF, LF and CR are recognized as protocol line
endings. Parser, source indexes and query positions use the same boundaries while preserving
authored bytes.

Required capabilities:

- syntax diagnostics;
- schema diagnostics;
- unknown block diagnostics;
- duplicate ID diagnostics;
- missing ID diagnostics;
- unknown speaker diagnostics;
- unknown metadata key diagnostics;
- invalid metadata value diagnostics;
- unknown condition/effect function diagnostics;
- wrong arity/type diagnostics;
- inline markup diagnostics;
- completion for block references;
- completion for speaker IDs;
- completion for metadata keys;
- completion for metadata values where schema provides registries;
- completion for condition/effect functions;
- hover documentation from schema;
- go-to block definition;
- find references for block IDs;
- rename block;
- code action to add missing ID;
- code action to create block stub;
- code action to add schema entry for unknown metadata/effect/condition where appropriate.

Metadata value completions and diagnostics must use the same domain resolution rules as compiler
validation (§10.2). In particular, contextual metadata domains resolve `field:speaker` and
`metadata:<key>` selectors the same way in the LSP as in the compiler, including the schema-declared
missing-context policy. The LSP must not invent broader fallback behavior for convenience; if the
manifest says the result is diagnostic, empty, or fallback to a named flat domain, editor
completions and diagnostics must reflect that same result.

Nice-to-have:

- semantic tokens;
- inlay hints for parameter names;
- condition preview with fixture data;
- dialogue flow outline;
- graph preview export.

## 15. Editor Support

Editor integrations are first-class authoring surfaces. VS Code/VSCodium, Neovim, and Zed must all
use the same LSP and shared authoring-kernel semantics; their syntax grammars and commands are
host-specific projections, not alternate language implementations.

The first syntax highlighting implementation uses an editor-native strategy:

- VS Code and VSCodium start with a TextMate grammar and `.recite` language contribution.
- Neovim starts with `recite` filetype detection, documented LSP setup, and tested syntax
  highlighting through Tree-sitter or a named Vim regex fallback.
- Zed starts with its language-server integration, file association, syntax highlighting, tasks, and
  diagnostic surface supported by its extension model.
- LSP semantic tokens may later layer richer classification on top of syntax highlighting, but they
  are not the first highlighting path and are not required for basic highlighting.

This choice favors immediate adoption in common text editors while keeping semantic authority in the
parser, compiler, and LSP. TextMate is broad enough for Recite's line-oriented statement vocabulary
and works before an LSP starts, but it cannot faithfully model every indentation and recovery
boundary. Tree-sitter is a better fit for Neovim and future structural editing, but it must remain
an editor grammar only; it must not replace the rowan parser or perform schema, reference, ID,
condition, effect, markup, or match-exhaustiveness validation.

Highlighting grammars must classify source text using stable visual categories only. They must
tolerate incomplete or malformed buffers and defer all author-facing correctness to parser,
compiler, and LSP diagnostics. In particular, editor grammars must not decide whether IDs are valid
or unique, block references resolve, metadata keys or values are known, condition/effect calls
type-check, inline markup is balanced, or match arms are exhaustive.

Initial highlighting scopes and captures:

| Source category                                                                                                         | TextMate scope family                                                                                                        | Tree-sitter capture                                                   |
| ----------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| Comments                                                                                                                | `comment.line.number-sign.recite`                                                                                            | `@comment`                                                            |
| Statement markers and directives such as `::`, `>`, `?`, `!`, `->`, `:if`, `:else`, `:match`, `:case`, and plural pipes | `keyword.control.recite`, `punctuation.definition.*.recite`                                                                  | `@keyword`, `@keyword.conditional`, `@punctuation.special`            |
| Block names, line IDs, choice IDs, and divert targets                                                                   | `entity.name.section.recite`, `entity.name.label.recite`, `variable.other.reference.recite`                                  | `@label`, `@variable`                                                 |
| Reserved words and choice clauses such as `default`, `END`, `requires`, and `reason`                                    | `constant.language.recite`, `keyword.control.conditional.recite`, `variable.parameter.recite`                                | `@constant.builtin`, `@keyword.conditional`, `@property`              |
| Metadata keys and assignment punctuation                                                                                | `variable.parameter.recite`, `keyword.operator.assignment.recite`                                                            | `@property`, `@operator`                                              |
| Metadata values: symbols, strings, numbers, booleans, and arrays                                                        | `constant.other.symbol.recite`, `string.quoted.double.recite`, `constant.numeric.recite`, `constant.language.boolean.recite` | `@constant`, `@string`, `@number`, `@boolean`, `@punctuation.bracket` |
| Runtime interpolation bindings such as `$name`                                                                          | `variable.other.runtime.recite`                                                                                              | `@variable.builtin`                                                   |
| Condition and effect function calls and call punctuation                                                                | `support.function.recite`, `punctuation.section.arguments.recite`                                                            | `@function.call`, `@punctuation.bracket`, `@punctuation.delimiter`    |
| Localisable prose bodies                                                                                                | `string.unquoted.prose.recite`                                                                                               | `@string.special`                                                     |
| Inline markup tags and interpolation placeholders                                                                       | `entity.name.tag.recite`, `punctuation.definition.tag.recite`, `variable.other.placeholder.recite`                           | `@tag`, `@punctuation.bracket`, `@variable.parameter`                 |
| Malformed or incomplete syntax                                                                                          | `invalid.illegal.recite` only for obvious lexical errors                                                                     | `@error` only when the editor grammar emits it                        |

Filetype detection is intentionally narrow: `.recite` maps to the Recite source language/filetype,
while `.recitec`, `recite.project.toml`, schema JSON, gettext files, and generated artifacts keep
their own formats.

Follow-up implementation work should be split into at least two issues:

- VS Code highlighting: add the `recite-vscode` language contribution, `.recite` file association,
  TextMate grammar, representative grammar fixtures or snapshots, and a clear boundary between
  grammar highlighting and LSP diagnostics.
- Neovim highlighting: add filetype detection, documented LSP setup, and tested syntax highlighting
  through Tree-sitter or the named Vim regex fallback, without duplicating semantic validation.
  Record the selected implementation and any host limitation in the editor acceptance evidence.

### 15.1 VS Code and VSCodium

The VS Code/VSCodium extension must provide:

- TextMate syntax highlighting;
- LSP client wiring;
- commands for compile/validate/extract/watch;
- structured problem integration: project typed CLI diagnostics into the command-owned VS Code
  DiagnosticCollection. A line-oriented `problemMatcher` must not parse localized or nested
  versioned NDJSON text; task/workbench contributions may be added only with an equivalent
  structured contract;
- block outline;
- quick run/trace command;
- optional graph preview.

### 15.2 Neovim

Neovim support must include:

- documented LSP setup;
- tested syntax highlighting through a Recite Tree-sitter grammar, or a named tested fallback using
  Neovim's Vim regex syntax engine;
- `recite` filetype detection for `.recite` files;
- command examples for validation, extraction, watch, and preview;
- semantic parity with the other first-class editors, with any Neovim host limitations recorded
  rather than used to weaken the contract.

### 15.3 Zed

The Zed integration must provide:

- `.recite` language detection and syntax highlighting;
- LSP diagnostics, completion, hover, navigation, rename, and code actions;
- a named minimum task/command surface for validation, extraction, watch, and preview, with host
  limitations recorded if a command is unavailable;
- setup documentation that keeps the language server and project configuration explicit; semantic
  parity remains mandatory for the supported operations.

### 15.4 GUI Workbench

The GUI workbench is a standalone, source-first authoring surface. It is built against the shared
authoring kernel after the native GUI strategy and accessibility proof, not as a second semantic
implementation.

The workbench must:

- operate on source or a lossless source-preserving edit representation;
- never lock users out of text workflows or silently rewrite stable IDs;
- show block graphs with an equivalent accessible list or outline;
- edit lines, choices, metadata, conditions, and effects through explicit, undoable source edits;
- surface parser, compiler, schema, localisation, and freshness diagnostics;
- inspect schema domains, producer provenance, and explicit regenerate/stale actions without making
  a generated manifest a second source of truth; edit the standalone declarative schema source or
  open/edit declarations through the engine producer, while unsupported producers remain explicitly
  read-only;
- provide gettext PO catalogue browsing and editing as the required v1 editable dialogue-catalogue
  path, preserving comments, context, unknown data, stable IDs, placeholders, markup, and fallback
  visibility with safe atomic writes; other catalogue formats are explicitly read-only or
  import/export-only;
- preview localized text and deterministic effect traces through the runtime;
- integrate with schema completions and the same LSP features as text editors;
- expose keyboard, focus, screen-reader, IME, BiDi/RTL, zoom, text-scaling, high-contrast, and
  non-colour paths for every essential operation;
- handle external file changes and save conflicts explicitly;
- represent stale generations and cancellation, announce progress and status, offer failure/retry,
  retain or restore focus, support reduced motion, and provide manual assistive-technology
  verification where automation is insufficient.

The GUI workbench is required for serious v1. A fully general visual node editor with a separately
persisted or losslessly round-trippable graph format is not. Automatic layout is the default.
Viewport state is transient and local; an optional checked-in open sidecar may preserve layout keyed
by stable IDs. Graph layout and viewport state must never become dialogue semantics or make the GUI
the only authoring path.

### 15.5 Native GUI Strategy and Accessibility Proof

Freya is the selected native frontend. The parser, authoring kernel, configuration discovery,
compiler, and runtime remain authoritative; Freya owns presentation, focus, interaction, and
file-session orchestration. A future frontend port would replace those responsibilities without
replacing language semantics. Reconsider Freya for a concrete accessibility, text-input,
distribution, or maintenance blocker, and run repeatable regression checks before upgrading the
pinned release candidate. Linux, Windows, and macOS remain first-class v1 desktop targets for the
core CLI, LSP, editor integrations, and standalone workbench; selection does not establish
acceptance on those platforms.

The fixture must cover source editing, stable-ID insertion, diagnostics, completion, schema
inspection/editing, PO catalogue editing, localisation preview, graph navigation, undo/redo,
external changes, and runtime preview. The proof must cover keyboard-only workflows, focus order,
screen readers on each declared platform, IME composition, BiDi/RTL text, zoom/text scaling, high
contrast, non-colour meaning, progress/status announcements, failure/retry, focus
retention/restoration, save conflicts, reduced motion, and startup/memory/packaging evidence.

The decision record must name the selected strategy, supported platforms, fallback route, framework
versions, native dependencies, maintenance burden, known limitations, and reconsideration triggers.
No candidate is accepted on renderer reach or visual polish alone.

## 16. Engine Adapters

The normative adapter contract lives in `docs/engine-adapter-contract.md`. This section records the
product-level requirements that the contract expands.

The standalone GUI workbench is a first-class authoring surface. IDE and text editing remain
first-class and are expected primary workflows. Engine adapters are thin companions: they integrate
the compiled asset and shared authoring workflow into an engine's asset, schema, event, and editor
conventions, but do not become alternate dialogue authoring applications or runtimes. A companion
may provide host-native import, schema production, refresh, and presentation hooks; the standalone
workbench and text editors remain useful without an engine project.

Schema-manifest export, including resource-backed metadata-domain snapshots and stale-schema checks,
is part of that adapter contract (§7). Engine-specific sections here must not require Recite
compiler, CLI, LSP, or runtime code to scan host assets or execute game code.

### 16.1 Goals

The core runtime is engine-independent. Engine adapters are integration layers that make Recite feel
native in a host engine without changing the dialogue contract.

Adapters must:

- load compiled dialogue assets through the host's asset pipeline where possible;
- export or consume generated schema manifests through the shared contract in
  `docs/engine-adapter-contract.md` §7;
- define how compiled assets are imported or refreshed during authoring;
- store active dialogue session state in host-native resources, nodes, components, or services;
- expose dialogue lines, prompts, effects, endings, and errors through host-native events, messages,
  signals, or callbacks;
- preserve choice selection by stable `ChoiceId`;
- preserve blocking-effect acknowledgement semantics;
- let users drive dialogue UI and presentation however they want;
- avoid requiring dialogue files to call directly into engine scripts.

### 16.2 Adapter API Shape

Every adapter should expose host-native equivalents of these operations:

- start dialogue from a compiled asset, optional block, and optional locale; `None` selects
  source-text-only mode;
- select a dialogue choice by `ChoiceId`;
- acknowledge a blocking effect by `EffectRequestId`;
- observe structured dialogue output:
  - line;
  - prompt with optional line and choices, preserving choice availability and structured unavailable
    reason trees;
  - effect request;
  - end with deferred effects;
  - structured error.

The concrete API should feel idiomatic for the host engine. A Bevy adapter may use resources and
events/messages. A Godot adapter may use nodes, resources, C# APIs, and signals. A Unity adapter may
use C# packages, imported assets, events, and editor import hooks. The semantics must stay
equivalent.

### 16.3 Active Sessions

Initial adapter scope may maintain one active dialogue session per declared adapter owner. Each
adapter must document whether that owner is a singleton service/resource, node, component, scene
service, or equivalent host-native object.

Attempting to start a second scene on the same owner while one is active must emit an error, not
panic.

Each adapter must document and test what happens when a compiled asset changes while a session is
active. The shared policy names are `reject_refresh_until_session_ends`,
`reload_for_next_session_only`, and `restart_required`. Silent mid-session mutation is not
acceptable because it can break deterministic traversal, save/load identity, previous prompt choice
validation, and pending blocking-effect semantics.

Adapters should make the edit-source -> LSP diagnostics -> on-save IDs -> `recite watch` rebuild ->
engine import/refresh -> restart scene loop practical for their host engine. A richer mid-session
patch reload can be explored after v1, but it is not required for the serious v1 gate. The
standalone workbench must expose the same source, schema, localisation, freshness, and preview
states without requiring an engine to be open.

Future versions may support multiple sessions keyed by entity/session ID.

### 16.4 Conditions and Effects

Adapters should support:

- registering condition handlers through the host's normal extension points;
- emitting generic effect requests;
- optional generated typed effect events, signals, or records from schema may remain post-v1; v1
  adapters may provide handwritten typed wrappers, but must always expose the generic structured
  effect-request contract;
- test fixtures independent of the host engine runtime where possible.

Conditions must remain pure queries. Effects must remain typed requests emitted to the game. Adapter
convenience APIs must not move game-side mutation into the Recite runtime.

### 16.5 Initial Adapter Targets

Godot, Bevy, and Unity are v1-facing adapter targets. This is a settled product scope decision, not
a ranking of engine value. The serious v1 gate requires all three adapters to be production-quality
and to pass the engine-independent conformance coverage in `docs/engine-adapter-contract.md` §13,
including contract-aligned asset refresh and active-session behavior.

No adapter may weaken the engine-independent core contract.

The source-tree adapter packages are acceptable while Recite is pre-release, but the release path
must not leave engine users integrating from ad hoc repo paths. Before declaring 1.0, the v1-facing
adapters must have store- or ecosystem-native distribution plans: Godot Asset Library/addon
packaging for Godot, Unity Asset Store or Unity Package Manager-friendly distribution for Unity, and
crates.io plus Bevy plugin/example packaging for Bevy. Those bundles should include the runtime
assets, editor/import tooling, examples, version compatibility notes, and conformance evidence
needed for a game team to install or upgrade Recite without reverse-engineering the repository
layout.

Unreal and GameMaker remain post-v1 evaluation targets.

### 16.6 Shared Conformance Artifacts

Adapter conformance scenarios are published in `fixtures/adapter-conformance/v1/` with:

- a versioned scenario manifest;
- a manifest schema;
- a stable operation/result schema.

Those fixtures are adapter-consumable contracts, not private Rust-only test support. They define
operation sequencing, capability gates, changed-asset policy declarations, projection capability
declarations, and expected structured outcomes/errors. Projection-capable adapters must expose
projected affordance records with stable IDs, target identity, deterministic ordering, label
template provenance, localized text when resolved, and structured fields; they must not expose only
host-rendered strings.

`.recite` source fixtures still belong under `fixtures/recite/`; conformance manifests reference
those sources instead of duplicating parser/compiler/runtime snapshot expectations.
