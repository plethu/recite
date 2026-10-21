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
registries, hover, definition/references, block rename and guarded source-preserving code actions.
Missing stable IDs are inserted explicitly; clients may configure invocation on save. Existing IDs
remain frozen. Schema actions edit source declarations or invoke their registered producer, never
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
[Writer acceptance](../../apps/writer/acceptance.md) owns repeatable human checks and known limits;
its [guide](../../apps/writer/guide.md) owns current workflows.

### 15.5 Native GUI Strategy and Accessibility Proof

Xilem is the selected migration target following bounded native input, accessibility and maintenance
experiments. The existing Freya application remains the default until workspace parity is verified.
Presentation and focus belong to the frontend; source editing belongs to the shared authoring model,
and file replacement, conflict protection and recovery belong to `recite-writer-session`. Language,
configuration, compiler and runtime ownership remains outside both frontends. Exercise regression
checks before upgrading the pinned toolkit.

The opt-in Xilem frontend covers retained project documents, beat/line/reply creation, paged Script
reading, manuscript editing, diagnostics, checked save, recovery and explicit IDE handoff/reload.
File switching preserves drafts, selection, accepted edit history and recovery leases; closing and
Save all consider inactive documents too. It uses an external editor for source repair rather than
embedding a second source editor. This does not establish parity for the remaining workspaces or
remove the existing Freya Source workflow.

Project-wide beat rename uses the shared reviewed plan and grouped history. Xilem moves the sole
project owner to a worker separately for Review and Apply, retaining a read-only writing projection
and mounted native fields. The durable grouped journal commits before the model swap; cancellation
after that point cannot discard the returned owner. Both frontends exclude newer edits, navigation
and Save until document recovery and the journal's final directory sync complete. Known checkpoint
failures offer Retry; an unexpected worker panic requires closing with recovery and reopening.
Global Undo/Redo reaches grouped history after local document edits. Save all includes the staged
source, references and manifest in the shared save sequence.

Standalone declaration TOML uses the same external-editor boundary in Xilem. A bound source can
restore an invalid recovered draft; export preserves it in a separate editable TOML file, and
explicit import validates its producer before staging it. The original source changes only on Save
and generate. Source replacement stays busy through its fixed recovery fence, while manuscript
typing and history remain independent. Malformed generated JSON is an explicit damaged-output state:
inspectors identify the last validated declarations as such, retain the observed bytes for conflict
checks, and allow the standalone source to regenerate the output.

Output and producer-registration reloads share that checked action owner, preserving newer TOML
typing. An invalid registration replaces the reviewed command with its error. Output changes retire
cached source-currentness claims; explicit draft validation compares the current buffer with the
loaded output, independently of project-analysis refresh and Save's own validation.

Native Map geometry, synopsis, spatial selection and personal-placement format have shared owners.
Its accepted projection is prepared on workers and checked against source identity before adoption;
placement writes reconcile even when presentation is cancelled. Camera and local layout never add a
source undo step. Stable pane slots retain Script's mounted text history. Visible detail cards stay
native controls; low-zoom overview paints geometry, retains complete outline/Go to/Connections
paths, and restores a readable focused card on selection. Focus requests reveal the whole card
through the outer scroll container. Collapsing focused cards transfers focus to the persistent map
viewport.

Both frontends use the same personal configuration, palette and command wording. Xilem's welcome,
temporary examples and project creation use shared launch/session owners. Its personal settings
apply theme and scale without replacing mounted editor history; shortcut edits use shared conflict
validation. Ordinary exit confirmation never weakens the unsaved-work guard.

Xilem's project settings workspace retains a conflict-checked opening baseline. Export creates a
separate TOML draft for the external editor; explicit Import validates the prospective project and
preserves every open document. Apply writes the exact review, revalidating under the manifest lock.
Publication and saved-baseline reload briefly exclude document membership and declaration changes;
manuscript typing remains independent. Successful writes reconcile before asynchronous analysis
refresh. Uncertain storage outcomes retain the last-known baseline for inspection, retire obsolete
producer authority, and require explicit baseline reload before another Apply. Failed refresh keeps
navigation guarded until a checked retry restores project context. Unstarted reloads preserve prior
authority; failed observations quarantine it, and cancellation cannot erase observed saved changes.

Configured native shortcuts use the same validated chord matcher in both frontends. Xilem routes
commands before text input, retains native text undo/redo, and scopes Apply to the focused
manuscript. Composition bypasses application shortcuts; document, screen and binding changes
invalidate queued commands. Unbound Control/Meta chords cannot become literal text. This does not
establish physical IME or keyboard-layout acceptance on every platform.

Writing Focus overlays the selected workspace without replacing its manuscript widgets or native
text history. Leaving Focus restores and reveals the previous valid native focus target, with a
writing-pane fallback when that target has retired. F6/Shift+F6 cycle visible panes; Vim's
directional pane prefix is scoped to its originating focus and keyboard context. Composition, focus
loss and context changes cancel it, and consumed terminal releases cannot activate underlying
controls. Host-owned command strokes retain their physical release after focus or command scope
changes, including picker submission returning focus to an action button. Pane focus, redraw and
accessibility updates must publish without requiring another input event.

Scene/beat search repeats resolve current accepted navigation facts in discovery/rank order. Native
lookup retains only its accepted query and semantic scene/beat target; each invocation captures a
fresh location and advances the cursor only after checked navigation succeeds. Cancelling another
lookup leaves that accepted search intact. Saved-project-text lookup does not replace it.

Native project-tool screens retain the mounted writing surface and its text history. Declaration
inspection is paged; generated schemas remain read-only. Build saved uses disk inputs, while Save
all and build completes the shared ordered save sequence first. Build jobs and terminal polling
outlive the selected screen. Cancellation keeps close and project replacement guarded until the
terminal result is collected.

Xilem opens ordinary PO catalogues through the shared localisation owner. Exclusive catalogue
inspection returns that owner on failure or cancellation; queue and entry presentation are bounded
independently of catalogue size. Translation typing remains editable during a captured save, and
newer drafts survive its receipt. Save all publishes catalogue drafts before the normal source save
sequence. Close waits for both layers and their recovery; cancelling close preserves dispatched
writes. Unexpected catalogue failures quarantine editing and close using only admitted recovery.
Disk decisions require the exact inspected local catalogue and external fingerprint. Keeping drafts
preserves compatible translations for review against the adopted PO metadata; using disk explicitly
discards those drafts. Catalogue creation and source refresh share extraction and review policy with
Freya. Both native owners are checked out for source operations and returned on cancellation, failed
dispatch and unwind. Unresolved current or retained source fields refuse extraction. Refresh Apply
re-extracts source and checks the exact reviewed catalogue before publication. A completed PO write
remains an applied result even if recovery subsequently fails; both frontends surface that warning.
Configured localised preview remains migration work.

Registered schema generation retains one shared activity owner through command validation, process
execution and checked output publication. Native publication receipts are reconciled even after
leaving Declarations or requesting cancellation; cancellation after a successful write cannot undo
its new schema baseline. Project discovery and analysis refresh run separately on a worker. Refresh
adopts analysis without replacing live field drafts, selection, preview or undo history. A failed or
stale refresh keeps the published schema and offers a retry. Stale reply controls retire while their
staged source draft and validation policy remain available for external repair. Command details and
terminal reports are paged, including individual long lines.

Queued accessibility actions are checked against current native widget identity before dispatch;
retired controls cannot crash the host or act on a replacement widget. The current host bridge owns
one native window and its base layer.

Xilem prepares file/selection navigation and applied edit history on workers. File switches retain
field drafts; linked navigation applies captured prose first and stops if newer typing remains.
Publication requires the same document and exact outgoing recovery checkpoint. A Save requested
during explicit Apply waits for successful publication and retirement, then uses the normal checked
save path; cancellation or failure clears that request. Save All takes precedence over repeated
Save.

Xilem prepares explicit disk reload on a worker, including discovery, accepted projections, saved
search inputs and a complete independent recovery archive. Adoption requires the same project root,
current selection/draft, baseline and retained session inputs. Completed archives remain available
even when cancellation or newer typing prevents adoption. Recovery cleanup is queued before an
infallible owner swap and joined off-thread; a later cleanup failure reports an accepted reload with
failed cleanup, without rolling back adopted content. Subsequent typing retains its own recovery.

Disk comparison observes the accepted source and unapplied field separately, preserving selection
and history. Keep Writer retains the exact model and rebases its saved inputs to the inspected disk;
Use Disk adopts that source as one history step. Both archive the full prior recovery and recheck
the exact local session and disk before admission. Once recovery submission succeeds, publication
completes despite cancellation; a late durability failure reports adoption with incomplete recovery.
Xilem composes this workflow through the existing exclusive project worker. Its comparison mounts
one bounded changed passage at a time. A separate IDE merge draft materializes pending prose without
applying it to the live document; Import validates against the captured context and inspected disk,
then adopts one undoable source replacement. Only explicit Save writes the source file.

The Xilem editor handoff saves accepted source and recovery in the background without accepting the
active field draft. It uses the OS preferred application or literal configured arguments. Cancelling
the launch still reconciles dispatched writes; conflicts prevent launch. An external editor's
lifetime stays independent of the Writer window, and completion names its source document.

Its source-language runtime preview prepares immutable accepted inputs off-thread and publishes only
into the still-current document and requested destination. Cancelling or hiding preparation
invalidates publication immediately; an active compiler phase may finish before another starts.
Playback uses the shared runtime for choices, condition inputs and explicit blocking-effect
acknowledgements. It never executes game effects, persists trial state, applies field drafts or
silently replaces a running snapshot after edits. Restart is explicit, and failed preparation
retains the previous run. Accepted sibling source participates; unapplied sibling fields do not.

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
