# LSP, editors and engine adapters

Part of the [production specification](../recite-production-spec.md). Section numbers remain stable.
[Release §22–23](release.md#22-recommended-milestones) distinguishes product requirements from
verified support.

## 14. LSP

The LSP projects shared parser, compiler and authoring capabilities into editor protocols. It owns
transport, URIs, versions, position conversion and stale-result mapping, not another language model.
The [editor parity contract](../editor-parity-contract.md) owns document synchronization,
cancellation, result freshness and structured commands; its machine-readable fixture records tested
capabilities.

Required authoring operations include syntax/schema diagnostics, completion from declarations and
registries, hover, definition/references, block and stable-ID rename and guarded source-preserving
code actions. Missing stable IDs are inserted explicitly; clients may configure invocation on save.
Existing IDs remain frozen except through an explicit stable-ID rename (§5.4.2), which also edits
the project's catalogues and is refused with a localised `RequestFailed` reason when unsafe. Schema actions edit source declarations or invoke their registered producer, never
generated manifests. Metadata completions use the compiler's domain and missing-context rules.

## 15. Editor Support

VS Code/VSCodium, Neovim and Zed are first-class text surfaces. Their native clients own setup,
activation, commands and syntax-only highlighting. TextMate and Tree-sitter tolerate incomplete
buffers and do not validate IDs, references, schemas or traversal. `.recite` is the source filetype;
compiled assets, TOML, schema JSON and gettext files retain their own formats.

The [parity fixture](../../fixtures/editor-parity/contract.json) owns capabilities and executable
evidence. Package READMEs own host setup and limits; source grammars and their tests own capture
names. Installed-host evidence does not establish other platforms or assistive-technology usability.

### 15.1 VS Code and VSCodium

Use the [extension guide](../../editors/vscode/README.md). Structured command diagnostics populate a
command-owned `DiagnosticCollection`. A line-oriented problem matcher cannot safely parse nested or
localized NDJSON; any additional task integration must preserve the structured contract.

### 15.2 Neovim

Use the [native package guide](../../editors/recite-neovim/README.md) for LSP setup, Tree-sitter,
commands and host checks. Host limitations do not weaken semantic parity for supported operations.

### 15.3 Zed

Use the [extension guide](../../editors/zed/README.md). Static terminal tasks are distinct from a
structured command controller. Tasks needing asset, block or fixture arguments require explicit
project inputs rather than guessed defaults.

### 15.4 GUI Workbench

Writer is a standalone source-first surface over the shared authoring kernel. It must preserve text
workflows, comments, unknown metadata and stable IDs; edits are explicit and undoable. Graphs have
an equivalent accessible list/outline. Automatic layout is the default; local viewport or stable-ID
sidecar layout never becomes dialogue semantics. The beat outline uses accepted compiler summaries,
including recognized headers in incomplete source; selecting an entry still uses the normal Script
validation and requires one unambiguous local declaration. Recovery and history may still restore an
incomplete selection without authorizing a structural edit. Its default and END badges describe
authored syntax, not reachability. Unapplied Source drafts do not change this accepted outline.

The workbench supports project opening, editing, diagnostics, completion, search/navigation,
recovery, conflict-aware saving, schema inspection and producer actions, lossless gettext PO
editing, and deterministic runtime preview. Standalone schema editing owns declaration TOML;
engine-backed actions navigate or regenerate through the producer. Generated manifests and
unsupported producers remain read-only. Other catalogue formats are read-only or import/export-only
in v1.

Keyboard, focus, screen-reader, IME, BiDi/RTL, scaling, high-contrast and non-colour paths must
cover every essential operation. Cancellation, stale generations, progress, failure/retry and focus
restoration are observable states. Reduced motion and external-edit protection are required.
Hands-on acceptance is tracked in #237.

### 15.5 Native GUI Strategy and Accessibility Proof

Xilem is the Writer frontend. The `recite-writer` package owns presentation, focus and interaction;
source editing belongs to the shared authoring model, and file replacement, conflict protection and
recovery belong to `recite-writer-session`. Language, configuration, compiler and runtime ownership
remains outside the frontend. Toolkit fixes live on pinned forks; exercise regression checks before
moving a pin. Software rendering is required wherever no usable GPU is available.

The Writer holds these contracts across every workspace; the crate tests are their executable form:

- **Drafts and history survive.** Navigation, file switches, workspace changes, reloads, catalogue
  work and project tools never discard an unapplied field draft or native text history. Moving away
  from a draft applies it through the ordinary checked path and stops on refusal, cancellation or
  newer typing. Apply never saves; only explicit Save writes files, and Save all follows the shared
  ordered save sequence across sources, catalogues and staged project edits.
- **Work is checked before publication.** Expensive work (projections, previews, reloads, disk
  comparison, builds, schema generation, catalogue operations) runs on workers against captured
  inputs. A result publishes only into the same document, baseline and destination it was prepared
  for; stale, cancelled or failed results return exclusive owners and change nothing visible.
  Results that already wrote durable state still report that outcome after cancellation.
- **Composition is protected.** IME preedit holds the writing surface until commit or cancellation;
  application shortcuts never consume composition, and unbound Control/Meta chords never become
  text.
- **External editors stay external.** Source repair, standalone declarations and project settings
  round-trip through explicit export/import with validation; the Writer never embeds a second source
  editor or silently rewrites authored source. Disk changes are observed and offered for explicit
  comparison; detection never reloads or applies anything.
- **Cost is bounded by what is visible.** Manuscript pages, map overscan, search results, catalogue
  entries, trace records and diagnostics mount a fixed budget of native widgets regardless of
  project size, and camera movement or typing reuses retained content.
- **Assistive paths are complete.** Every operation is reachable by keyboard with visible and
  accessible focus, and focus is restored after dialogs, pickers and workspace changes. Controls
  keep complete accessible names when visually abbreviated, queued accessibility actions are checked
  against current widget identity, and colour is never the only cue.
- **Preview is a trial.** Runtime preview uses accepted source and captured settings, never executes
  game effects, persists trial state or applies drafts, and keeps its run and provenance until an
  explicit restart succeeds.

Linux, Windows and macOS remain first-class desktop targets, not implied platform acceptance. Claims
require native evidence for authoring, assistive technology, physical input, startup, memory and
installed packages on each declared platform. Renderer reach or visual polish is insufficient.

## 16. Engine Adapters

The [adapter contract](../engine-adapter-contract.md) owns common runtime, asset, schema, save/load,
localisation, refresh and conformance behavior. Companions integrate those operations with host
conventions; they do not become alternate runtimes or authoring applications. Writer and text
editors remain usable without an engine project.

### 16.1 Goals

Native asset import, schema production, event delivery and presentation must preserve the shared
contract. Core compiler/LSP/runtime code never scans engine resources or executes game code.

### 16.2 Adapter API Shape

Expose host-native start, select-by-ID, acknowledge-by-ID, output, snapshot/restore and disposal
operations. [Contract §4](../engine-adapter-contract.md#4-runtime-operations) defines their
observable behavior; package guides own concrete APIs.

### 16.3 Active Sessions

Each adapter declares its session owner and changed-asset policy under
[contract §3](../engine-adapter-contract.md#3-session-ownership) and
[§11](../engine-adapter-contract.md#11-changed-compiled-assets). No silent replacement of a running
session is permitted. The authoring loop is edit, validate, rebuild, import and explicitly restart
or follow that policy; live patching is not required for v1.

### 16.4 Conditions and Effects

Pure conditions query caller state; effects request game-side work. Handwritten or generated typed
host wrappers preserve the generic structured interface. Generated binding tooling remains post-v1.

### 16.5 Initial Adapter Targets

Godot, Bevy and Unity are v1-facing targets and must pass the shared contract through their own
surfaces. Release packages need ecosystem-native installation and upgrade paths: Godot addon/Asset
Library packaging, Unity Package Manager or Asset Store distribution, and crates.io plus Bevy
examples. Include native runtime dependencies, import tooling, version support and useful samples.
Unreal and GameMaker remain post-v1 evaluation targets.

### 16.6 Shared Conformance Artifacts

[Adapter fixtures](../../fixtures/adapter-conformance/README.md) own operation/result schemas,
scenario sequencing, capabilities and expected outcomes. Each adapter runs applicable scenarios
through its native surface. A reference-runtime pass cannot establish host behavior. Shared Recite
source remains in `fixtures/recite/`, rather than being copied into each adapter's tests.
