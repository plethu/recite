# Writer workflow study · 04

The [remaining-workflow study · 05](remaining/index.html) extends the unresolved
forms, rules, preview, schema, source and project flows in one linked review.

Open [index.html](index.html) directly in a browser. No build or server is needed.
The study is disposable, uses sample data, and changes no project files. Edits
survive navigation within the study; reload resets them. The strip above the app
contains prototype controls, including failure scenarios and dark appearance.
French dialogue is an illustrative, unreviewed fixture, not a published locale.

This study follows the bilingual manuscript in [study 03](../localisation/README.md).
It replaces that study's modal queue direction with routed workspaces and extends
the remaining feature flows. The source-refresh direction was approved with the revisions below and is now
being implemented natively. The other feature flows remain wireframes; this study
is not evidence that those features or platform acceptance are complete.

## Audience and tasks

A localiser may translate a passage, return after the writer changes it, review
someone else's work, or alternate between Recite and another PO editor. They need
the surrounding conversation and the reason for a change, not just entry IDs or
coverage numbers. A writer needs to express availability and actions without
having to think like a compiler, while retaining the ability to edit source.

Keep the scene navigator and Write / Localise modes. Within a mode, the manuscript
is the ordinary working surface. Batch source review and the translation queue get
full screens with browser history. File-specific actions stay near catalogue
identity. No additional permanent inspector or second scene tree is proposed.

## Walk through these tasks

1. **Understand source updates** — Start on the source review screen. Inspect the
   changed departure time, new reply and removed timetable. Expand the surrounding
   exchange and update scope. Leave without updating, then return. Updating the
   catalogue retains translations and marks changed source for review; it never
   approves their meaning. The operation covers the whole extracted catalogue,
   not a misleading selection of three independently synchronised pieces.
2. **Translate related forms** — From the manuscript, open Tickets. Edit both
   French forms, keeping `{count}`. Save without marking reviewed, then review and
   save. Edit again: review intent clears. Switch to the formal variant. Navigate
   away and back: drafts remain. Form labels show examples, not an English-centric
   universal “singular/plural” assumption.
3. **Understand fallback** — Preview in Canadian French. The fixture visibly falls
   back to French (France). Try another ticket count and switch language. Unsaved
   translation input is identified. A missing formal variant visibly uses default
   wording. Preview does not silently count fallback text as a finished translation.
4. **Resolve an external edit** — Choose “PO edited elsewhere” in the study strip.
   Attempt Save, compare the two versions on a full screen, and choose a starting
   point. Returning to the passage does not silently approve or save the choice.
5. **Handle a stale review** — Choose “Source changes during review”, then attempt
   Update. Nothing is saved. Recheck, inspect and retry. Production must regenerate
   the actual change list; the fixture repeats the same three sample entries.
6. **Author availability and actions** — Open Write. “When is this reply available?”
   and “What happens when selected?” belong to the reply, below its incoming speech.
   Change the standing requirement, apply, and try it in Preview with the gate key
   enabled. Raising the requirement changes availability. Turn off the action and
   apply: selecting the available reply in Preview no longer requests unlocking.
7. **Build the intended scene** — Choose Build scenes and a manifest. The scope
   description follows the choice. Build saved source, navigate away and return.
   The selected scope remains. Build success is explicitly a simulation.

## Decisions proposed

- Separate three actions: update catalogue structure, save translation text, and
  attest that its meaning has been reviewed. Never make “accept update” stand in
  for all three.
- Show previous/current source and the retained translation together. Surrounding
  dialogue is available in place; navigation must return to the same change.
- A source refresh applies to a complete extraction. Removed entries are retained
  as history; source-change review remains outstanding afterward.
- Put form selection within the passage. Preserve draft text across variants and
  navigation. Give the current form's save action a stable position; no auto-next.
- Preview inputs are temporary and explicit. Show which catalogue/variant supplied
  the text, whether fallback occurred, and whether unsaved translations are used.
- Structured controls edit the same source as Source view. Unrepresentable
  expressions remain visible and source-editable. No “simplify” action may erase
  them implicitly. Game actions remain requests in preview.
- Build manifests define scope; selecting a scene for writing does not implicitly
  change the build target. State which unsaved changes are excluded before Build.

## Questions for design review

- Is global source synchronisation followed by passage review clear enough, or
  do localisers need a deferred-change list before updating the catalogue?
- Is the old/new/translation comparison readable at the native minimum window
  size? Should unchanged surrounding dialogue be expanded by default?
- Are variants best chosen inside the passage, or shown as named sibling rows
  for projects where several variants are routinely translated together?
- Is “Apply to source” an understandable intermediate state before project Save?
  Should ordinary structured edits instead join the existing source undo history
  immediately, with no additional Apply step?
- Does the build flow need a dedicated screen in ordinary use, or should this
  become an expandable scope summary beside a normal Build command?

## Deliberately unresolved

Plural categories and example counts must come from real catalogue rules. The
fixture uses two French examples; it is not a universal plural model. Combined
plural-plus-variant editing, missing plural metadata and category expansion need
another fixture. The source-change comparison does not yet render word-level diffs.

The rule fixture covers one conjunction and one request. Nested conditions,
argument editors, effect ordering, blocking effects, unknown schema members,
read-only producers and mixed structured/source edits need follow-up wireframes
before implementing the general editor. Source details are labelled illustrative
meaning, not invented Recite syntax.

Preview is a demonstration, not a runtime. Real branching, trace inspection,
start/reset/resume, complex typed inputs and preview freshness need further design.
Build progress, cancellation, failed saves and diagnostics are described but not
simulated. No real files, gettext, compiler, runtime or OS handler are invoked.

Queue search only filters fixture source text; the native queue's broader search
contract remains. No search-result ranking decision is implied here. Native
screen-reader/IME/platform evidence is still separate from browser checks.

## Implementation gate

Review the task flows and settle the questions above first. Then implement one
accepted workflow through shared kernel operations, with native interaction tests
for its success, cancellation, stale-source and external-file paths. The
wireframe's in-memory JavaScript is not an implementation model to port wholesale.

## Verification

Checked in local Chromium: source-update review and stale retry, revised-source
translation editing/review, draft retention across navigation, independent variant
review state, placeholder refusal, external comparison, visible locale fallback,
applied rule inputs, empty search, and keyboard entry into the workspace.
Source review, forms, preview, rules and build screens have no horizontal page
overflow at 1440, 1000, 700 and 375 pixels. Visually inspected wide light source
review, wide dark translation forms, 900-pixel dark source review and narrow build
scope. This is browser interaction/layout evidence, not native or assistive-
technology acceptance.

Static views: [source review](screenshots/source-review.png),
[translation forms](screenshots/translation-forms.png),
[reply rules](screenshots/reply-rules.png).

## Approved source-refresh revisions

Use passage excerpts instead of beat names alone in the change list; collapse the
scene tree during project-wide review. Keep the header compact, highlight source
changes, and show essential context directly. Notes must identify their provenance;
never generate narrative intent. Synchronisation preserves translations and history
without requiring per-passage approval. Translation review follows as a separate task.
