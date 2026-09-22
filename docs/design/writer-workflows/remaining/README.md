# Remaining writer workflows · study 05

Open [index.html](index.html) directly in a browser. No server, package install or
build is needed. The study controls above the application switch between tasks;
they are not proposed permanent app navigation. Reload resets all sample edits.
French text is illustrative and unreviewed, not a published translation.

This is one coordinated design pass over the remaining feature areas. It extends
[study 04](../README.md), preserving the manuscript, contextual controls, routed
workspaces, and paper/sage palette. It does not replace the native implementation
or establish platform/accessibility acceptance.

## Revision 2: quieter workspaces

This revision follows the screenshot review. It removes explanatory sidebars,
vertical accent borders and developer-facing copy from the proposed app surface.
Sample failure/ownership controls now live in the study toolbar. Conversation,
plural metadata, provenance and build inputs are disclosed where needed.

- Translation keeps source, wording, forms and review in one working area.
- Availability and effects sit together; selecting an effect expands its own
  arguments in place. Nested backgrounds express actual logical grouping.
- Preview groups game inputs and language choices, with Restart beside them.
  The trace starts collapsed; fallback is indicated alongside the delivered text.
- Declarations lead with description/signature. Source ownership is available
  beneath it, with contextual editing and generation actions.
- Source uses document tabs, a completion popover and a problems list. The
  referenced document remains in the same editor layout.
- Build keeps the target, output and primary action visible. Save and build is
  primary when writing is unsaved; progress occupies a stable result area.
- Radios, checkboxes, focus, pending jobs and action groups share visual rules.
  Reduced motion retains a static pending indicator and the stage text.

### Shared comparison presentation

`primitives.js` owns the comparison renderer and pending-job presentation.
The comparison accepts labelled versions and aligned rows; the host owns draft
state, navigation, layout preference and the consequential action. Source updates,
external changes and rename all use it here. The earlier study remains a historical
reference; the new Source updates route demonstrates its revised presentation.

Rows show inline changes, explicit missing/removed states, and optional unchanged
context. Unified view and Next change are shared controls. Source updates retain
catalogue-specific consequences; external conflicts return both chosen plural
forms to editing; rename previews both references before applying them.

This is a presentation contract for native reuse, not a production diff algorithm.
The fixture highlights the changed middle between a common prefix and suffix;
production needs the project's appropriate diff implementation and real version
provenance. Completion placement is illustrative rather than measured from a
native caret. Browser controls are a design reference for the Freya primitives.

## What to review

| Task | Entry point and proposed interaction | Recovery to try |
| --- | --- | --- |
| Plural and variant translation | Localise / Tickets. Choose a variant; edit all its catalogue-defined forms together. Save/review applies to that PO entry, not to each arm independently. | Navigate away with drafts; remove `{count}`; simulate missing plural metadata. Drafts survive and saving explains the refusal. |
| Locale and variant preview | Try this passage. Choose explicit locale, count and variant. Source-only remains available. | Missing Canadian French falls back to `fr`, then missing formal wording falls back to the default entry. The trace identifies each decision. Fallback never marks an entry translated. |
| Conditions and effects | The reply owns availability and the statements in its chosen branch. Nested All/Any grouping, typed arguments, negation and effect order are visible. | Discard structured edits; preserve an unsupported expression through Source rather than silently flattening it. |
| Runtime trial | Inputs sit beside delivered dialogue, with trace below. Each run takes a snapshot; changing inputs does not silently rewrite a run in progress. | Raise the standing requirement, restart, choose the now-available reply, acknowledge the blocking request. Immediate/deferred requests never execute game actions. |
| Schema ownership | Project / Declarations, also reached from a condition or request. Standalone source can be edited; an engine declaration opens its owner. Generated output is read-only in all cases. | Switch to an unsupported producer; simulate stale output and failed generation; retry without replacing the last usable manifest. |
| Source repair and navigation | Source keeps diagnostics with the editor. A diagnostic reveals/selects its location; completion explains declaration provenance. Rename has a full review screen. | Visit another document and return with the draft intact. Native scroll-to-line and cross-document navigation remain requirements. |
| Build scope | Project / Build scenes. Explicit scene selection describes input, schema and output; the selected writing passage does not set the build target. | Cancel a build or fail validation. The previous output remains identifiable; “Save changes and build” stops when a save is blocked. |
| External changes | Catalogue identity / Files & changes. Watchers report changes; the same conflict guard runs at save time even with watching disabled. | Compare draft and disk on a full screen. Choose a starting version, return to editing, then explicitly save/review. “Save then open” must not launch after a failed save. |

## Decisions proposed for the whole pass

- Keep Write and Localise as the only modes. Declarations, Build and file changes
  live behind Project or contextual links. The study selector is outside the app.
- Keep temporary choices near their passage. Use full routes for comparisons,
  schema browsing, rename review and build scope; never stack large dialogs.
- Plural form count/examples come from the current PO rule. Do not apply an
  English two-form model to every locale or silently copy another catalogue's
  plural header. Variants and counts are separate axes.
- Review belongs to the whole PO entry. Changing any arm clears review intent;
  a different variant retains its own draft and review state.
- Use an explicit Apply step for structured rule edits, grouped into one source
  undo operation. Project Save remains separate. This is a proposal to review,
  not a new source-format contract.
- Preview uses the shared runtime and explicit test inputs. Show the actual
  source/schema/catalogue snapshot and lookup trace. New input values take effect
  on Restart; old results remain visibly stale rather than being overwritten.
- Preserve arbitrary source expressions. Supported structured edits must not
  erase unsupported conditions, reorder effects, or rewrite unrelated trivia.
- Open schema declarations through their producer. Save source before generation;
  stale output, generation failure, retry and unsupported capabilities are normal
  visible states. Never offer an editor for the generated manifest.
- Keep external editor configuration to registered applications or the system
  default. Launch paths as arguments, not shell interpolation. File watching is
  advisory; fingerprint checks at save remain authoritative.

Native progress is tracked in [implementation.md](implementation.md). The full
design is approved; that checklist distinguishes implemented behaviour from
remaining work.

## Implementation sequence after design review

One overall feature pass, delivered as testable local slices:

1. Shared contracts: plural/variant edit targets, producer capabilities, preview
   inputs/snapshots, build jobs and external-change state. Reuse the authoring
   kernel, PO editor and current GUI primitives; do not port the prototype state.
2. Plural/variant editing and locale/fallback preview, including review flags,
   placeholders, malformed metadata and external-file protection.
3. Structured conditions/effects plus runtime input, trace and blocking-request
   controls. Round-trip source preservation and effect ordering are gates.
4. Schema browsing, source-owned editing and producer-driven generation;
   completion/navigation/rename consume that same declaration provenance.
5. Manifest-specific builds, file watching and preferred-editor handoff, with
   cancellation, stale-result rejection and draft preservation.
6. A combined native usability/maintainability pass and the full writer gate.

The coordinator owns integration and acceptance throughout. Packaging, URL-handler
registration, native file-dialog proofs, screen readers, IME/BiDi, physical input
and Windows/macOS validation remain deferred to the packaging stage as agreed.
These are separate from missing application features and are not declared passed.

## Limits of this study

This is a disposable HTML/JS fixture. It performs no filesystem operations,
gettext extraction, schema generation, compilation, runtime traversal or external
application launches. Timed generation/build results are simulations; cancellation
invalidates the pending sample result.

The rule editor demonstrates a nested conjunction/disjunction and a negated
condition, not an arbitrary condition builder. Registry and enum selectors,
additional statement kinds and every schema declaration shape must use real
schema metadata in implementation. The sample plural rule has two arms; native
layout must handle the catalogue's actual arm count and long translations.

The source completion and rename examples operate on fixed sample text, not a
parser. A production rename needs a current affected-reference preview, version
checks, one undoable transaction, and no changes to stable IDs. The browser study
cannot resolve the pinned native editor's missing scroll-to-line API.

Native failure paths also need durable recovery, actual file fingerprint races,
failed saves across multiple files, producer cancellation and source changes while
jobs run. This study makes those contracts explicit without claiming their proof.

## Study verification

Checked in Chromium with light and dark appearances. Browser interaction checks
covered draft retention, placeholder refusal, missing plural metadata, locale and
variant fallback, stale previews, nested-rule inputs, blocking acknowledgement,
producer ownership/failure, cancelled and failed builds, external-change recovery,
diagnostic selection, completion, two-reference rename, discard after Apply, and
Ctrl+Enter submission with editor focus retained. Ten routes were checked for
horizontal page overflow at 1440, 1000, 700 and 375 pixels. JavaScript syntax and
Git whitespace checks passed. These checks apply only to the browser fixture;
they do not establish native keyboard or assistive-technology acceptance.

Revision 2 also checked shared comparison layout toggling, keyboard focus through
Next change, live rename previews, both-form external resolution, selecting and
editing different effects, and the stable pending build region. Light/dark and
middle/narrow screenshots were inspected. The earlier workflow checks still pass.
