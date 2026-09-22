# Native implementation against study 05

The revised study is approved. This checklist tracks native behaviour separately
from the browser fixture. Approval covers the full sequence in the study README;
an implemented row is not acceptance of the whole GUI milestone.

## Implemented in this working tree

- Shared labelled comparison rows, inline text changes, explicit absent values,
  unified/side-by-side presentation, and automatic stacking below 640 pixels.
  Source updates and external catalogue changes use the same primitive.
- Routed translation entry editing for singular/plural entries and variants.
  Forms come from the catalogue; examples are evaluated using its plural rule.
  Variants retain independent drafts. Saving/reviewing validates every form and
  writes the PO atomically. Unknown plural rules prevent saving.
- External catalogue comparison has an explicit starting-version choice. Both
  choices return unreviewed editable drafts; neither silently saves. Resolution
  rechecks the inspected file fingerprint. Changed plural source or changed
  plural rules cannot silently rebase a retained draft.
- Queue selection opens the entry editor; Back restores search and pagination.
  Translation, comparison and trial locations participate in history.
- Preview retains its compiled asset and live runtime session. Pending conditions
  can be answered and blocking effects acknowledged or reported failed, without
  executing game effects. Deferred requests remain visible at conversation end.
- Translation and comparison submission reuse the guarded action and shortcut
  hint. Modified Enter does not also activate a focused checkbox/button.
- Opening another catalogue clears comparison and refresh state.
- Locale/variant/count trial controls capture catalogue bytes and interpolation
  values on Restart. Saved PO and explicitly included draft translations are
  separate choices. Fallback traces report attempted locales and variants and the
  selected plural form. Staged inputs survive navigation.
- Translation entries disclose nearby source and notes and offer a contextual
  trial entry point. Nearby text is labelled as source order, not runtime order.
- Source completion queries the unapplied draft through the compiler, uses its
  replacement span, and refuses stale replacements. The shared searchable picker
  supports Ctrl+Space and Vim list navigation. Source Apply uses Ctrl/Cmd+Enter.
- File switching, browser history, search and diagnostic links retain document
  drafts, applied undo history and recovery ownership. Save-and-close includes
  retained files and stops on a failed save or external change.

- Offscreen source diagnostics and inserted completions reveal the caret on both
  axes through a temporary, documented rc.4 code-editor patch. Pre-layout
  requests are retained; manual scrolling does not snap back to the caret.

- Routed reply rules edit availability with nested All/Any groups, negation,
  removal and schema-backed additions. Typed values use declaration names and
  registry/enum choices. Incomplete values remain in the recoverable source draft;
  Apply and Save validate before accepting them. Applying is one undoable edit.
- Effects belong to the reply's same-document destination, not to nested choice
  statements (the compiled format does not support those). The screen names that
  destination and states that changes affect every route into it. Direct effects
  can be edited; ordering/addition/removal are limited to a simple effect sequence
  ending in a divert. Other statements and cross-file destinations remain in Source.
- Rule edits preserve IDs, comments and source outside the changed condition or
  effect slots. Changed condition expressions use canonical grouping; untouched
  rules are byte-identical. Drafts survive Rules/Source history, and undo refreshes
  the structured projection. Lightweight argument checks run while typing; full
  project validation runs on Apply/Save.

## Continued implementation

- Declaration browsing and explicit standalone TOML source association, editing,
  validation and generation. Generated files remain read-only. Source reload keeps
  a recovery copy; normal closing refuses to lose an unsaved declaration draft.
- Engine producer registration, explicit generation, source file/line navigation,
  cancellation, bounded failure output and validated publication. See
  [the registration contract](../../../schema-producer-registration.md).
  Commands never run during discovery or registration loading.
- Document tabs retain drafts and history, support arrow keys and Vim j/k, and
  require saving before closing a dirty tab. Tabs use the shared button primitive.
- Reviewed project-wide rename includes source references and manifest scene entry
  points. The shared comparison shows every affected file. Apply is a grouped
  undoable edit; Save project includes all affected documents and the manifest.
  A project recovery record covers interrupted multi-file checkpoints. Disk saves
  remain individually atomic; a failed save retains the remaining unsaved files.
- Manifest-selected builds reuse the CLI's build coordinator and publisher, with
  background progress, cancellation and a choice of saved source or save-first.
  A failed save prevents build startup; source changes during a run are reported.
- Advisory manuscript and catalogue file watching, explicit comparison and checked
  saves. Preferred-application handoff saves first and runs outside the UI thread.
- Comparison change navigation, optional unchanged context, and monospace source
  rendering are shared by source conflicts and rename review.

Freya 0.5.0-rc.7 replaces the vendored horizontal-scroll patch. The host reveals
source-editor caret bounds. Exact prose-caret restoration after remount remains
TBD pending upstream support; see the GUI acceptance record and Freya #2308.

Packaging and physical platform/accessibility proofs remain separately deferred,
as recorded in the study README. No milestone closure follows from this checkpoint.

## Verification

The earlier comparison/translation checkpoint passed the complete writer gate: workspace tests, all-target/all-feature Clippy,
formatting and the 10,000-passage heap check. Canonical UI resource tests, test
organisation, Git policy and whitespace checks passed. Subsequent wording and
trace-borrowing refinements were checked with the UI suite, targeted preview and
translation tests, and all-target/all-feature Clippy. Native interaction coverage
includes variant draft retention, all-form review/save, modified-Enter submission,
queue history, condition answers, blocking acknowledgements and restart. The
translation editor was rendered and inspected at 1400 by 1000 logical pixels.

Maintainability review kept the catalogue file as one owner of file identity,
drafts and atomic writes, and the entry component as one whole-entry interaction.
The small per-arm component owns input state; splitting its internal callbacks
would obscure their shared review boundary. Comparison presentation owns no file
mutation. SubmitAction is shared by dialogs and routed editors. Preview borrows
its event trace instead of copying the entire trial on every step. Larger existing
router, workbench and inventory files retain their existing responsibility; these
changes add routes, runtime commands and canonical IDs within those boundaries.

The continued pass adds native tests for locale/count fallback, source completion
and history navigation without saving, plus model tests for fixed trial inputs,
draft completion, retained undo history and retained-file save conflicts.
The complete writer gate passed again, including the 10,000-passage heap check.
Subsequent retained-context and snapshot refinements passed focused model/native
tests and all-target/all-feature Clippy. UI resource, test-organisation, Git-policy
and whitespace checks passed. The native trial was rendered and inspected at
1400 by 1200 logical pixels. A sandbox-only rerun of the broad unit suite hit the
known gettext/wait-timeout signal-handler limitation; the full gate ran successfully
with the required host access, and the changed tests also passed individually.

Reply-rule verification includes model tests for declaration-backed additions,
invalid values and delivery modes, stale revisions, grouped undo, Unicode/CRLF,
effect ordering boundaries and additional instances of existing diagnostics.
Native tests cover Apply/Discard, history with an incomplete typed value, disabled
submission, undo, condition removal and editing/reordering expanded effects. Light
and dark renders were inspected at 1400 and 1000 pixels wide. The complete writer
gate passed after the grouping and buffer-ownership refinements, including the
10,000-passage heap checks. The final shared input-shortcut adjustment passed all
three native rules tests and all-target/all-feature Clippy. Canonical resource
contracts, test organisation, Git policy and whitespace checks passed.

Rule controls remain feature-owned: the argument field is reused by conditions
and effects, while buttons, searchable pickers, exclusive choices and submission
are shared UI primitives. The rules page coordinates one source-draft transaction;
expression rendering, effect rows, arguments and declaration selection have
separate components. The authoring model owns projection, validation and patches.


The final project-tools checkpoint passed the complete writer gate (workspace
unit/integration tests, all-target/all-feature Clippy, formatting, and both
10,000-passage heap workloads). Configuration and UI suites, the CLI build adapter
and publisher tests, root config/CLI Clippy, test organisation, Git policy and
whitespace checks passed. Subsequent presentation refinements were checked with
native workflow tests and Clippy. Engine-owned producer coverage uses a temporary
exporter and editor to verify explicit invocation, publication, declaration refresh
and exact file/line/column arguments. No real external editor was launched.

Native checks also cover schema validation and external conflicts, source draft
close protection, rename review/save/grouped undo, interrupted project checkpoint
recovery, file watching and unsaved comparison resolution, and save-before-build
failure. Light and dark screens were inspected at 1400 and 1000 pixels wide.
The sandbox's existing wait-timeout signal-handler failure required host access
for process-running tests; the host runs passed.

Final maintainability review keeps producer registration in recite-config and
process execution in the GUI host. It reuses the CLI build coordinator/publisher
rather than creating a second publication implementation. Declaration source
editing, producer execution, source links and browser presentation have separate
modules. Background watchers and job polling remain mounted independently of
visible toolbars and update shared state only when results arrive.

Size-triggered modules remain below 400 lines. The project file chrome (280),
project owner (253), build page (287), rename transaction (298) and declaration
source controls (264) were reviewed as cohesive units: respectively project UI
composition, retained file ownership, one build interaction, grouped edit/undo,
and one source-editor session. Splitting their local callbacks further would
separate the state they coordinate. Process, persistence, comparison and source
link concerns already have separate owners. The upstream vendor files retain
upstream structure, with the small local changes documented for eventual removal.

The approved native feature pass is implemented. Packaging, OS URL/file
associations, physical accessibility/platform proofs, and replacing the temporary
Freya patch after upstream ships equivalent behaviour remain separate. Standalone
declaration drafts and PO translation drafts now use the shared atomic recovery
store. Reopening the same PO catalogue restores its drafts. Rebinding the same
standalone TOML source restores its draft, including incomplete text. Recovery
retains the original disk baseline so external edits still require resolution.
The source association itself is not persisted.
