# Writer refinement proposal

Design proposal, 18 September 2026. Based on nine maintainer screenshots and
the current, partly uncommitted implementation in `apps/writer`. This is not an
implementation or an accepted replacement for the existing visual direction.

Implementation has begun with the shared component finish: see the runnable
native specimen and current contracts in the
[writer design system](../../apps/writer/design-system.md). The first pass applies
surface roles, typography, button feedback, grouped switches, bounded pickers,
and graph-card presentation to the existing app. The September 22 workspace pass adds standalone Script, command search,
source-owned caret completion and saved presentation settings. Native acceptance
remains separate from implementation; see the writer design system for current
behaviour.

Recite should be comfortable to write in for a whole afternoon. The manuscript
needs room, operations need predictable places, and opening a control should
not interrupt the reader's sense of where they are. Preserve the warm neutrals,
sage accents, and dialogue serif from the [visual language](../gui-visual-language.md).
The work is chiefly composition, interaction, and typography.

## What the current app tells us

| Evidence | Consequence | Proposed change |
| --- | --- | --- |
| Screenshots 1–2; `field_completion.rs` passes completion into `SearchPicker` | Completion occupies the editor width, moves input into another field, and shows internal candidate indices | A caret-anchored completion list driven by typing in Source; no permanent full-width completion control |
| Screenshots 5–7; `scene_map.rs` stacks action, zoom, legend, and search rows | Map controls consume substantial height before the graph begins | One compact map toolbar; contextual help; a bounded connection inspector |
| `scene_map/connections.rs` allocates 96 px per result and two 160 px lists | Two-line connections have large gaps and the inspector crowds the map | Shared list rows with content-appropriate metrics; collapsed connection summary by default |
| Screenshots 8–9; passage and destination controls expand across their containers | Routine metadata dominates the prose | Compact action menu; destination rendered as an editable link; expand the picker only on invocation |
| `workspace.rs` embeds Script beside Map | Writing depends on a graph layout even when the writer only needs prose | Full Script view, with an optional map split |
| `design/button.rs` gives hover and selection the same emphasized background | Moving the pointer can look like changing location | Separate hover, pressed, selected, and keyboard-focus tokens and treatments |
| `closing.rs` treats unfocused Escape as a close request | Repeated dismissal can unexpectedly quit the app | Escape dismisses the nearest interaction; quitting requires an explicit quit action |
| `tokens.rs` and `palette.rs` already exist, but callers retain local geometry and colour mappings | A token file alone does not ensure consistent components | Consolidate by visual role and component contract, then migrate complete workflows |

Screenshots show spatial problems; they do not establish frame-time, IME, or
screen-reader behaviour. Measure those separately. Skia is already the renderer;
changing the renderer is not needed to address the demonstrated layout problems.

## Workspace composition

Keep **Write / Localise** as the two activities. Within Write, offer **Script /
Map / Source** as views of the same working document. Script becomes the first-use
default; preserve a returning user's explicitly chosen view. Keep Project actions
and document identity reachable in the common shell.

The common header contains history, scene identity and saved state, the view
switch, and Try scene. Write/Localise remains a distinct activity switch rather
than becoming five equivalent tabs. Use a second row only when width or text
size requires it. Undo/redo remain accessible through menus and shortcuts;
whether to retain toolbar buttons should be checked in the native specimen.

Script uses a centred reading column, approximately 50–70 characters wide, with
comfortable side margins. The selected beat's heading, speaker, dialogue, reply
text, and destination must be distinguishable before reading them. Replace the
stack of bordered reply cards with spacing, restrained separators, and a stable
metadata line. Keep prose editing positions stable when actions appear.

Map gets the available canvas. A single toolbar groups Find beat, Add beat, and
arrangement; zoom/reset/fit sit together at the canvas edge. The percentage is
also the reset control, eliminating the two adjacent “100%” labels. Put gesture
help and the edge legend behind a named Help action, with an accessible route to
the same explanation. Do not rely on an unsolicited tooltip for essential help.

Connections start as a compact summary for the selected beat: incoming and
outgoing counts, with an explicit Expand action. Expansion exposes the complete
keyboard-operable list in a resizable inspector. A list-only presentation must
remain available for people who cannot use the visual graph. Closing the
inspector restores its invoker; reopening remembers its size.

Selecting a map card identifies it; opening its script is an explicit action
(Enter/Open script, with double-click as a pointer accelerator). Show related
edges on hover or focus without changing selection or the inspector's subject.
This changes today's single-click editing behaviour and needs a short usability
trial. Do not silently change camera framing during inspection or editing.

Allow an explicit Script + Map split, retaining the existing pane-side preference.
Script remains usable without the split. At narrow widths or large text sizes,
show one working view and a clear return action instead of squeezing three panes.
Remember reading position, map camera, selected beat, and editor selection per
document; view changes must retain drafts and undo history.

Focus writing hides navigation and auxiliary panes while retaining a quiet saved
state and an obvious way to restore the workspace. It restores the prior layout
exactly. Preview remains a deliberate route, with Back returning to the same
passage and caret. Localise uses the same manuscript geometry and editing
components, adding bilingual content and entry review controls where needed.

## Controls that stay near the work

**Completion:** open at the native caret, normally 280–420 logical pixels wide
and bounded by the viewport. Typing continues in Source. Up/Down chooses a
candidate; Enter accepts only when a candidate is active; Escape dismisses and
leaves the caret intact. Tab retains ordinary editor navigation/indentation unless
the user explicitly enables Tab acceptance. Ctrl+Space remains an explicit entry
point, with platform conflicts tested and rebinding available. Show declaration
kind or provenance when useful, never the internal candidate index. Recompute
or dismiss on draft/caret changes; retain the existing replacement-span checks.
Caret geometry uses shaped glyph bounds with the editor scroll and gutter
offsets. The host adapter retains this calculation after the Freya rc.7 upgrade;
the vendored patch is removed. Native DPI and composition checks remain open.

**Destination:** show the current destination as a named link and a compact
Change action. Invocation opens the shared searchable picker at that row.
Separate “visit destination” from “change destination.” End conversation remains
an explicit option. Choosing a result preserves the current undoable edit path.

**Passage actions:** use a bounded menu aligned to its invoker, with stable space
reserved in the metadata row. Show the trigger on focus as well as hover, and
keep it discoverable without pointer movement. Common Add reply/Add line actions
remain available at natural insertion points and through commands.

**Scene navigation:** keep the existing source-order scene/beat outline. Reduce
row spacing without reducing target size below 24 logical pixels. Distinguish
the active scene's expansion from the selected beat. Search keeps its width and
layout on empty results; show the query and Clear search action. A small typo
should be recoverable; exact and prefix matches should outrank optional fuzzy
matches, with predictable source-order ties. Test matching separately from UI.

**Drafts and saving:** polish the presentation without quietly changing semantics.
Keep Apply draft and Save distinct wherever the current authoring model requires
them. Put pending actions beside the affected draft and name the remaining state
in the shell. A later decision about implicit apply needs explicit undo, invalid
source, recovery, navigation, and multi-document rules.

## Component finish and visual character

The first proposal under-specified this. Better layout alone would leave the
current default-widget appearance intact. The component finish is a primary
deliverable: the existing graph highlighting shows the level of care that the
rest of the interface needs to reach. Preserve that interaction and make the
surrounding controls feel equally intentional.

The proposed direction is a quiet editorial instrument: clear ink, carefully
set dialogue, solid surfaces with a little depth, and precise interactive
details. The current palette is a useful starting point, but retaining its exact
values is secondary to achieving this result. Avoid texture and decorative
effects that compete with the author's text.

### Surfaces and edges

The screenshots put much of the app on nearly identical charcoal surfaces,
then distinguish controls with conspicuous grey outlines. That gives the eye
many boxes to parse without a clear sense of depth. Establish four visibly
related surfaces: window ground, working surface, inset field, floating menu.
In dark mode, use small deliberate luminance steps; in light mode, use warm
white working surfaces against stone chrome. Validate both independently.

Use filled, softly rounded fields with a clear boundary. Secondary actions use
a restrained fill; ordinary toolbar actions have no resting frame. Reserve
stronger outlines for focus and controls that need them. Menus get a fine edge,
a restrained shadow and internal padding, so rows sit inside a floating surface
instead of forming a giant outlined form. Do not lower essential boundary
contrast to obtain a quieter screenshot.

Trial 6 px control corners, 8 px graph-card corners and 10 px floating-surface
corners, with straight pane boundaries. These are specimen values, not permission
to round every container. Check curve, inset and focus-ring geometry together.
Selected and hovered surfaces must retain readable text in both appearances.

### Typography, icons and optical alignment

The generic serif and UI defaults need an actual type specimen. Compare installed
families using real multi-paragraph dialogue, short replies, punctuation, italics,
and multilingual text. Choose and record a default family and fallback chain;
if fonts are bundled, verify redistribution rights. Let writers override reading
and source fonts. Judge the serif by sustained reading, including its weight on
dark backgrounds, rather than by an attractive heading.

Give controls an explicit text style instead of inherited toolkit defaults.
Align icon strokes, text baselines, chevrons, and shortcut columns optically.
Use medium weight for selected tabs and important labels; keep body prose
regular. Supporting text should be quieter through role and spacing while
remaining comfortably readable. A smaller font alone does not create hierarchy.

Use one coherent vector icon family, with a consistent optical size and stroke.
Replace text-glyph chevrons, close marks and drag handles in controls where font
fallback changes their shape or alignment. Keep prose punctuation as text.
Icon-only controls need names, tooltips and the same target geometry as their
neighbours. Avoid making every control a pill.

### What each component should feel like

| Component | Proposed finish |
| --- | --- |
| Search field | Inset surface, aligned search icon, clear button inside the field, balanced horizontal padding, calm resting boundary and distinct focus ring |
| View switch | A single grouped track with a clearly selected segment; consistent segment height and inset, rather than unrelated green buttons |
| Button | Deliberate text weight, optical centring, matched icon gap; hover changes surface, press visibly deepens it, focus stays separately legible |
| Outline row | Tight baseline rhythm, aligned disclosure icons, restrained scene grouping; selected beat has persistent emphasis that hover cannot imitate |
| Menu/picker | Bounded floating surface, padded rows, aligned optional detail and shortcut columns; selected row fills its inset cleanly; no internal IDs |
| Reply | Prose leads; speaker and routing metadata form a quieter editorial line; edit affordances belong to that line instead of framing the whole passage |
| Graph card | Distinct solid surface, restrained corners, carefully inset title/speaker/excerpt; text ends with intentional truncation rather than appearing cut off |
| Connection | Preserve the excellent route emphasis; tune stroke joins, dash rhythm, arrowheads and label backplates together at multiple zoom levels |
| Scrollbar | Consistent thickness, inset and thumb treatment; no collision with row borders or content; visible enough to communicate overflow |
| Dialog | Clear surface hierarchy and title rhythm, balanced margins, stable action group; the same fields and buttons used everywhere else |

Graph emphasis should have a short, interruptible transition between resting
and active routes. Preserve enough contrast to read unhighlighted structure.
Keep label backplates visually integrated with the canvas rather than looking
like rectangular holes cut into the lines. Selection and keyboard focus must
remain identifiable while another route is hovered.

### Response is part of the drawing

Pressed feedback starts immediately. Trial 80–120 ms for hover transitions and
120–160 ms for opening or emphasis, with reduced-motion equivalents. Transitions
must reverse from their current value when the pointer changes direction, without
queued animations or delayed activation. No button scaling, bouncing menus or
animated text reflow. The caret, selection highlight, scroll behaviour and focus
restoration deserve the same scrutiny as colours and corners.

Prove the finish before undertaking the broad layout migration. The next design
artifact should be a runnable native specimen containing a search field, view
switch, buttons, menu, destination picker, passage and graph fragment. Show each
at rest, hover, press, focus, selection, disabled and error, in light and dark.
Inspect actual font rendering and record short interaction captures. Then apply
the chosen treatment to one complete Relay Hub screen for review. An attractive
static mockup or a list of tokens alone does not establish this quality.

## A design system with useful owners

Extend the existing `design` module. Keep the source of truth in Rust; Freya's
theme is an adapter to those values. Browser studies illustrate them and do not
become another independently maintained production theme.

| Owner | Contract |
| --- | --- |
| Semantic palette | Canvas, manuscript, inset, overlay; primary/secondary text; separator/control border; hover, pressed, selected, focus; attention and error |
| Typography | UI, metadata, heading, dialogue, source; family, size, leading, weight and fallback together |
| Metrics | Spacing scale, minimum targets, control density, content insets, list row variants, overlay bounds, pane limits |
| Motion | Immediate input feedback; brief optional overlay transitions; reduced motion; no animation of text reflow |
| Shared controls | Button, icon button, segmented view switch, search field, list row, menu, picker, dialog, splitter, feedback and empty result |
| Workspace layout | View composition, pane persistence, narrow layout, focus restoration and region traversal |
| Commands | Identity, label, shortcut, scope, enabled reason and one execution path |

Use one palette definition for both native widgets and custom painting. In
particular, remove duplicated colour tuples between theme construction and helper
functions. A decorative separator can be quiet; a necessary control boundary
needs adequate contrast. Give selection a persistent marker, hover a neutral
fill, and keyboard focus an independent outline. Do not equate sage with success.

Start the specimen at UI 14–15 px, supporting text 13–14 px, dialogue 20 px with
roughly 1.5 leading, source 15 px, and controls 32–36 px. These are logical sizes
to validate, not screenshot-derived physical pixels. Add independent reading
and source font preferences and a UI text scale. Larger
text must grow controls and overlays rather than clip into fixed row heights.
Choose concrete system fallback families and inspect actual non-Latin rendering.

Share behaviour at the appropriate level. SearchPicker and source completion
can share overlay placement, list presentation and selection navigation; they
should not share input ownership. A menu, form picker, and editor completion
have different focus contracts. Extract those contracts explicitly instead of
adding many boolean variants to one universal component. Likewise, a navigation
row should not inherit the visual border treatment of a secondary action button.

Keep graph world geometry with the map; design tokens describe its presentation.
Do not replace every meaningful number with a global constant. Migrate repeated
visual policy and retain algorithm-specific values beside their algorithm.

## Learning the app without changing apps

All ordinary actions have visible entry points. Menus and tooltips show shortcuts
from the same command definitions. A searchable command palette offers the same
actions, filtered by current context, with disabled reasons where helpful.
Users learn accelerators through the actions they already use; there is no
separate “expert mode” that rearranges the application.

Proposed initial bindings, subject to platform and existing-binding review:

| Action | Binding |
| --- | --- |
| Save / Undo / Redo | Platform-standard primary-modifier bindings |
| Find in current view | Primary+F |
| Find across project | Primary+Shift+F |
| Go to scene or beat | Primary+P |
| Commands | Primary+Shift+P |
| Completion in Source | Ctrl+Space, configurable for platform conflicts |
| Apply the active draft/form | Primary+Enter, retaining the current explicit boundary |
| Next/previous workspace region | F6 / Shift+F6 |
| History | Existing Alt+Left / Alt+Right, with platform alternatives reviewed |
| Dismiss nearest transient interaction | Escape |

Primary means Ctrl on Linux/Windows and Command on macOS. Do not blindly assign
every platform an identical physical binding. Resolve commands from the focused
editor/control, through its transient surface, to the workspace. Exactly one
handler executes an action; composition and ordinary text input take precedence.
Escape never quits. Keep the existing Vim navigation option explicitly bounded:
normal-mode list/map commands do not capture letters in typing fields. Show the
mode where relevant, and preserve the existing INSERT/NORMAL dismissal sequence
until a deliberate migration is agreed.

For v1, prioritise command search, go-to, focus writing, and reliable region
navigation. Rebinding can follow the shared command definitions without a second
execution system. Defer macros, arbitrary docking, plugin APIs and a theme editor
until there is a concrete use case. These are maintenance commitments, not polish
prerequisites.

## Delivery and acceptance

1. **Native design specimen and shared controls.** Use real Freya controls and
   the Relay Hub fixture to compare light/dark, interaction states, menus,
   typography, and compact/detailed rows. Consolidate palette ownership and
   control metrics. Fix the oversized picker presentation and internal-index
   exposure across callers. Remove Escape-to-quit. Preserve existing edits.
2. **Complete the writing workspace.** Add Script as a full view, compose the
   common header, refine passage/destination controls, and retain an optional
   split. Make narrow layout and focus restoration part of this batch. Migrate
   persisted view preferences deliberately; Map/Source users keep their choice.
3. **Map and Source.** Consolidate map controls and connection inspection;
   validate selection/open behaviour. Implement true caret completion only once
   native geometry and focus are proven. Preserve camera and draft state.
4. **Commands and extended sessions.** Consolidate action routing, then add
   command search, go-to, focus writing, and shortcut discovery. Persist useful
   layout preferences through `recite-config`, retaining one authoritative owner.
5. **Consistency and performance pass.** Apply the shared contracts to Localise,
   rules, preview, comparisons, declarations, build and settings. Remove replaced
   implementations within each completed workflow; inspect the final diff for
   duplicated state, unrelated abstractions and stale documentation.

Each batch must be runnable and independently reviewable. Keep layout changes
separate from source semantics. The authoring kernel, file conflict guards,
recovery, stable IDs, and runtime effect boundaries retain their current owners.
No commits, pushes, or forge changes are implied by this proposal.

Capture the same representative states before and after at 900×650, 1280×800
and 1600×1000 logical pixels, light/dark, and 100%/200% text. Include long names,
large scenes, multiline replies, expanded labels, and mixed-direction text. Check
that essential actions remain reachable, overlays stay on-screen, focus is visible,
and prose does not jump when controls open. Save/conflict/error states matter as
much as the empty or ideal screen.

Behaviour tests should cover draft/caret/history retention across views,
one-action shortcut dispatch, composition-safe completion, Escape scope,
keyboard-only creation and route editing, restored focus, and the accessible
connection-list alternative. Use the documented writer tests, Clippy, formatting,
test-organisation and Git-policy checks; run the complete repository gate at
integration. Snapshot dimensions alone do not establish interaction correctness.

Measure typing-to-paint, input-to-map-motion, and frame times while panning,
opening menus and resizing on the actual desktop. Record device, scale, scene
size, refresh rate, median and tail latency. At 60 Hz, use the 16.7 ms frame
interval as a diagnostic budget; investigate repeated missed frames and stalls.
Do not attribute jank to Skia or promise frame pacing from headless tests.

Finally, perform an uninterrupted writing trial: create a beat, revise dialogue,
add and retarget replies, inspect a return path, try the scene, repair a diagnostic,
translate an entry, and resume after closing/reopening. Record lost places,
surprising focus, accidental actions, assistance needed and visual fatigue.
Native screen-reader, IME, physical-input and platform acceptance remain separate
evidence. The design is ready when these tasks feel coherent as well as look
coherent.

### Workspace review after component refinement

The September 18 follow-up inspected Relay Hub at 69% zoom, the split map/script
at 1200×800, an expanded passage inspector, and a generated long paragraph at
1400×1000 in both appearances. Captures are reproducible with
`RECITE_WRITER_CAPTURE_DIR` through the examples, map-navigation, scene and
reading integration tests.

The rendered specimen resolves Noto Serif Regular, weight 400, at 20 logical
pixels in both themes, with synthetic emboldening disabled. A regression check
compares the actual Skia font, not just requested CSS-like font properties.
The long editable paragraph also retains its layout dimensions across themes.
The perceived dark-mode weight difference is not evidence of a fallback or
weight switch; do not compensate with a different font weight without native
rendering evidence.

The remaining pressure is in workspace composition:

- Map tools occupy several rows, and permanent gesture instructions wrap when
  the inspector opens. Consolidate the toolbar and make gesture help available
  on demand while retaining keyboard discoverability.
- The persistent Connections pane competes with the graph at short window
  heights. Make it collapsible while preserving its accessible navigation and
  route highlighting.
- Every reply shows its destination plus a separate change-destination control.
  Combine destination display and editing so ordinary reading has fewer repeated
  controls; retain an explicit, keyboard-accessible edit action.
- Keep the writing scale and grouped metadata. Preserve draft, caret, focus and
  camera state while changing these layouts.

These are findings for the next workspace pass, not claims that these layout
changes or native frame-pacing and long-session acceptance have been completed.


### Structural simplification follow-up

The production map now uses two compact control rows. Gesture and edge guidance
lives behind Help; the current zoom value also resets zoom. Connections starts
collapsed and retains its route highlighting and navigation when expanded.
Reply destinations keep their preview and a quiet Change action on one row;
the searchable destination picker appears only while editing, with Escape
returning focus to Change.

Personal settings groups appearance, editing and behaviour. The configuration
path is disclosed on request, Vim guidance appears only for that keymap, and
immediate preferences close with a quiet Done action. Project settings retain
their explicit Apply action. These changes reuse the shared controls and preserve
source edits, map navigation and preference persistence.

### Writing and reference hierarchy

Pinned snapshots now have a bounded, independently scrolling reference area,
with a visible divider, read-only label and collapse action. Inline destination
previews use a neutral inset and leading rule; their smaller reading type keeps
the editable passage prominent. Reply-rule actions appear for the hovered or
selected reply and remain reachable by focusing its text with the keyboard.

Source completion is requested from the source action bar or Ctrl+Space. Its
results appear beside the caret without consuming editor layout space. Typing
continues in Source; Escape dismisses and Enter accepts a selected candidate.
Caret placement uses shaped glyph measurement and the editor scroll offset;
ordinary source navigation no longer presents a completion field above the text.


### Translation workspace hierarchy

Localisation keeps the beat title and source/translation headings above the
shared passage scroll. Incoming prompts start collapsed for each beat and use a
bounded scroll when expanded. Read-only source passages and their destination
previews stay in the left column; editable translations align alongside them.
Edit source returns to the writing workspace without discarding translations.

Sidebar activation in Localize opens the selected beat, and entering Localize
uses the current map selection. The highlighted beat follows the displayed
manuscript. Reply destinations use compact disclosure buttons; whitespace
separates replies without repeated enclosing borders.


### Active translation controls and queue rows

Translation review controls share a compact footer with Save and Discard draft.
The review checkbox receives the remaining flex width rather than claiming the
whole row; unsaved status sits alongside review status above the input.
Queue rows use reading type for passages, muted human-readable beat captions,
one translation status, and subtle separators. Reply rules lives in Passage
actions alongside details and adding a choice; destination controls remain direct.

### Actionable translation close protection

Closing with unsaved PO translations opens a resolution dialog listing the
catalogue, language, affected passages and draft previews. Each row opens its
entry; Keep editing opens the first draft. Save all and quit performs one atomic,
validated catalogue write. A failure remains in the dialog and retains every
draft. Discard names the number of drafts lost. Other unsaved project work still
passes through the existing source/recovery close policy.

## Final GUI readiness pass

The final pass extends actionable closing to declaration drafts and running jobs,
and gives PO and declaration drafts the same locked atomic recovery worker as
manuscripts. Recovery preserves original disk baselines. Catalogue replacement,
save, discard and close observe persistence failures instead of relying on a
background destructor. The declaration editor synchronizes external discard and
reload actions so its retained buffer cannot resurrect discarded text.

Another 51 static UI phrases now use the shared Fluent catalogue. Constrained
translation editing and closing passed at 900 by 650. The writer gate passed 199
tests, Clippy and heap checks; both explicit timing workloads passed separately.

See [GUI acceptance](writer-gui-acceptance.md) for the real-project writing trial,
recovery drill and remaining native accessibility/input/platform evidence. This
pass does not declare those human checks complete or close the milestone.
