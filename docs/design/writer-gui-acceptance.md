# Writer GUI acceptance

This is the final hands-on pass for the GUI milestone, excluding distribution.
Automated checks do not establish native screen-reader, input-method or physical
display behavior. Keep milestone #170 open until those requirements have evidence.

## Preview checkpoint and repeatable checks

The writer remains a preview while the library and text-editor integrations take
priority. Further substantial GUI design work should follow use by interested
writers. The maintainer does not need to become its primary usability tester.

Run `mise exec -- just check-writer-accessibility` for the focused automated pass.
These tests also run in `check-writer` and the existing CI writer gate. They cover
button names, modal metadata, forward and reverse keyboard focus, focus restoration
after Commands opens Settings, and keeping focused settings controls inside the
dialog at 100% and 200% UI scale in a 900 by 650 window. A writing workflow replaces
source, applies it and reads the result in Script without pointer input. The pass
also includes shortcut rebinding, pickers, palette scrolling, synthetic text
input, enlarged workspace layouts, contrast and reduced-motion component checks.
Status messages carry polite live-region metadata; errors carry assertive
metadata. Neither metadata nor synthetic input proves native announcements or
input-method behaviour.

The pinned upstream CodeEditor exposes an unnamed TextInput within Recite's named
Source editor group and does not present IME preedit text. Its API does not expose
an accessible-name property. These remain accessibility limitations; the button
name audit is not a claim that every input passes. The existing preedit probe
records that gap. Prose inputs have separate composition coverage.

On Linux, `mise exec -- just probe-writer-native-accessibility` launches only the
component specimen, with a private D-Bus session and temporary settings. It needs
a display (or Xvfb), Python GI/AT-SPI bindings and the AT-SPI daemons. It queries
the real accessibility tree and reports observed focus and action support as JSON.
Add `--require-actions` to fail unless both operations affect the app.

The September 23 native probe found the named button through AT-SPI, but focus
requests had no effect and the button exposed no action. Freya 0.5.0-rc.7's native
renderer ignores `ActionRequested`. This is a known accessibility blocker, not
an unperformed test. Keep it visible until the host handles native actions and
the strict probe passes; the normal probe reports `limited` for this result.

Native screen-reader and input-method testing, physical scrolling, and sessions
with prospective writers remain unverified. Record that evidence when testers
are available; passing this automated pass does not close those acceptance items.

## Keyboard, colour and press feedback

Settings offers monochrome colours in both light and dark themes, persistent
shortcut hints, and a Keyboard shortcuts page. The bindings list shows each action and its current keys. Select a row and press
a new combination, then save. Conflicts name the other action and require an
explicit replacement. “Choose keys separately” lets users select modifiers one
at a time and then press a single key. Each binding can be reset or unassigned.
Holding Ctrl (Command on macOS) reveals chips beside bound workspace actions.
In Vim mode, `:` opens Commands; enter `w`, `wa`, or `q` to use the existing
save, save-all, or close action. Close still observes unsaved-work protection.
`Ctrl+o` / `Ctrl+i` navigate history. `Ctrl+w`, followed by `h/j/k/l`, focuses
an available pane in that direction; a prompt shows the available destinations
and Escape cancels. These modifier chords also work while editing.

From navigation, `/` enters the scene/beat search, retaining its query; `n` / `N`
open the next/previous matching beat in the current scene. Search fields show
INSERT or NORMAL. Escape leaves insertion; `i`, `a`, or `/` resumes it.
`gg` / `G` jump to the first/last result in search lists and pickers. Literal
characters remain text in editors and insertion mode. Navigation actions are
also available in Commands and can receive alternative chords in Keyboard
shortcuts; the Vim gestures remain navigation defaults. Explicitly assigned
chords take precedence over the modified defaults. This is Vim-style navigation,
not full Vim editing or an Ex-command interpreter.

F6 and Shift+F6 move between workspace regions. Dialog buttons scroll into view
when focused; pressing a button moves its contents while keeping its hit target fixed.
Reduce animation also applies to this feedback.

The Commands list now renders rows at the same height used by its virtual scroll
model. Its regression test uses small scroll steps across row boundaries as well
as checking that idle layout does not undo manual scrolling.

`mise exec -- just check-writer` includes semantic colour ownership linting and
contrast tests for light, dark and monochrome palettes. Tests cover supporting
and error text, syntax tokens, primary actions, gradients, focus indicators and
map edges. They use the WCAG AA thresholds of 4.5:1 for normal text and 3:1 for
required non-text indicators ([WCAG 2.2](https://www.w3.org/TR/WCAG22/)). These
checks do not establish whole-app conformance: actual rendering, clipping,
assistive-technology announcements and platform behaviour still need inspection.

For the native pass, try the Commands list with both a wheel and trackpad; rebind
Commands to F8 and invoke it while editing Source; use settings entirely from the
keyboard in a short window; and check the two monochrome themes with enlarged
text. Test modifier hints with the operating system's Sticky Keys as well as the
always-visible option. Check screen-reader names, shortcut announcements, focus
restoration and the shortcut-save status message on each supported platform.

## September 22 workspace test pass

The writer now uses Freya 0.5.0-rc.7 without the vendored scroll patch. Script is
an independent selected-beat view, with optional Map split and Focus writing.
Existing explicit Map and Source preferences remain valid. Settings saves
reading size, Source size, UI scale and pane widths. Commands and Go to scene or
beat are available through Ctrl/Cmd+Shift+P and Ctrl/Cmd+P.

For the hands-on pass, run `mise exec -- just writer` and select Script:

1. Edit a line, switch scenes, return, then Undo and Redo.
2. Toggle Script + Map and Focus writing. Check that exit restores the layout.
3. In Map, single-click a card to select it; use Enter, Open script or a double
   click to open it. Pan, switch to Source, and return to check the camera.
4. Use Commands and Go to scene or beat; cancel each and check focus returns.
5. In Source, navigate a long line, scroll away manually, and invoke completion
   with Ctrl+Space. Type to filter, select with arrows, accept with Enter or
   dismiss with Escape. Completion changes the draft; Apply and Save stay explicit.
6. Change reading and Source sizes independently, then try 200% UI scale and a
   narrow window. Reopen to check the saved presentation preferences.

Headless captures cover Script at 900, 1280 and 1600 pixels wide, and Script and
Source with 200% UI scale, 32-pixel reading text and 28-pixel Source text. These
checks supplement the native acceptance work below. Source caret and selection
restoration are handled by the host. Physical IME, screen-reader and display
checks remain manual.

### TBD: prose caret and selection restoration

Exact prose caret and selection restoration after Input remounts is deferred
pending [Freya #2308](https://github.com/marc2332/freya/issues/2308). This includes
returning to a passage after scene or view changes that recreate its Input.
Continue the rest of GUI completion and acceptance with this limitation recorded;
it is not a blocker for the current pass.

Revisit the session bookmarks and remount regression tests once upstream confirms
a supported approach or provides an API. Text and draft retention remain part of
current acceptance; only exact prose caret and selection restoration is deferred.

The workspace pass passed 210 writer tests, Clippy and both 10,000-passage heap
checks. The repository run passed 1,456 core tests and the editor/adapter lanes;
a writer regression was fixed and its complete gate rerun. Documentation and
benchmark smoke lanes also passed.

## Visual and interaction follow-up

The follow-up consolidates the toolbar into two rows and provides a Workspace
menu at every text size. Localisation no longer shows writing-layout actions.
Shared buttons and segmented controls use flat surfaces. Script keeps its prose
font, with tighter passage spacing, subtle field boundaries, adjacent destination
actions and a labelled reference action. Standalone Script has no close-pane icon.

Map reply labels appear for hovered beats or focused connections; other routes
are quieter. The connection list remains available for keyboard inspection.
Source completion rows align left, diagnostics use a bounded panel, and source
submission actions share a background. Settings use compact value steppers.
Condition editing names the all/any semantics and puts structural actions behind
a disclosure. Translation queues combine filter/count controls and identify the
attention filter as untranslated or awaiting review; catalogue headers resolve
registered language names when available.

Performance is part of this pass: retain the measured search, projection and
parser allocation work in the same integration commit. The
[performance report](writer-performance-findings.md) distinguishes CPU gains from
allocation reductions; these measurements do not establish native frame pacing.

The September 23 follow-up passed 211 writer tests, Clippy, formatting and both
10,000-passage heap budgets. The repository gate reached the writer lane; its
failures were fixed and the writer gate rerun successfully. Documentation and
benchmark smoke passed separately, completing the remaining verification lanes.

The final maintainability review kept menu focus handling in its own module.
The larger button, condition and queue modules each retain one component or
screen responsibility; shared input, argument and navigation policy stays in
the existing helpers.

The post-commit screenshot critique is a separate design review. Passing tests
and bounds checks does not settle visual quality or physical-device acceptance.

## Screenshot-review corrections

The four follow-ups from the independent screenshot review are implemented:

- Narrow split-map panes place action pairs, search and zoom controls on separate
  rows. Help stays on one line, and the search field stays inside the map pane.
- Rule argument errors appear once beside their field and name the condition or
  effect. A collapsed effect retains its error summary. Parameter labels still
  come from the schema when available.
- Ordinary beat selection keeps surrounding routes visible and avoids accenting
  the whole return-route bundle. Strong fading belongs to connection inspection.
- Translation queues use the same unsaved-status label as passage editing; saving
  removes the unsaved indicator without changing the review-state rules.

A recovery regression exposed during verification is also fixed: dropping the
recovery owner explicitly releases its lock, even while a duplicate descriptor
exists. The duplicate-descriptor regression failed before the fix and passes
with it; the existing exclusive-owner checks remain in place.

Regression coverage checks narrow map bounds, inline and collapsed-effect errors,
and unsaved queue status before and after saving.

The map toolbar now owns its responsive composition in a separate module. The
remaining scene-map module composes map state and rendering; the condition,
queue and rules-screen modules retain their existing component responsibilities.

## Implemented in the final pass

- Closing identifies translation and declaration drafts and provides save,
  discard and direct navigation. Running jobs can be cancelled from the close
  dialog; the window stays open until they finish.
- PO and standalone declaration drafts use the shared locked, atomic recovery
  worker. Recovery retains the original baseline and refuses to overwrite
  external changes. Catalogue replacement and closing wait for cleanup.
- Static control wording moved into the shared Fluent catalogue.
- The translation edit, failure, navigation and save flow runs at 900 by 650.
- The explicit 1,000-beat workload mounted four cards. Its headless first-usable
  time was about 452 ms; navigation samples were about 10–14 ms. These are
  development-profile measurements, not native display frame pacing.

## Automated evidence for this pass

The full repository gate, `mise exec -- just check`, passed, including 1,453
workspace tests, editor/adapter checks, documentation and benchmark smoke.
The writer gate passed 199 tests, with two timing workloads normally ignored.
Both were also run explicitly and passed. Clippy, formatting and the writer heap
checks passed. The 900 by 650 translation workflow passed and its active editor,
queue and close-dialog screenshots were inspected. The shared Fluent contract
tests passed.

The 10,000-passage recovery workload reopened the latest of 100 drafts; queue
calls were below 0.1 ms and the final durable flush took about 61 ms in this run.
These observations are not crash-at-every-write or physical-display guarantees.

## Start here: a real writing session

Use a copy of a real project. Launch from the repository with
`mise exec -- just writer`, then open that project.

Spend 30–45 minutes doing ordinary work:

1. Create a beat, write several lines and replies, change destinations and use
   previews. Navigate with the sidebar, graph and history.
2. Undo and redo, switch Map/Source, edit source, apply it, save and reopen.
3. Open a PO catalogue, translate several passages, mark one reviewed, use the
   queue, and leave drafts in more than one beat.
4. Quit. The dialog should name those drafts and offer working navigation,
   save-all and discard-all actions. Cancel once, then save and quit.
5. Reopen and verify the saved text and destinations.

Report anything that made you hunt for a control, lose your place, or wonder
whether work was saved. Include the steps and the expected behavior.

## Recovery and conflict drill

Use disposable copies and one writer window per file.

1. Change a translation without saving. Wait two seconds, terminate that writer
   process forcibly, then reopen the same catalogue. The draft should return.
2. Bind a standalone declaration TOML source. Leave incomplete TOML, wait, force
   termination, and reopen and bind the same source. The incomplete text should
   return. Source association itself is not persisted.
3. Repeat with a manuscript field draft.
4. With a recovered draft open, change the corresponding file externally.
   Saving must report the conflict and retain the draft, not overwrite the other
   editor's work.
5. Discard a recovered draft, reopen again, and confirm it stays discarded.

Recovery is asynchronous: a process killed before the next snapshot reaches disk
can lose the latest edit. It must not corrupt the last durable snapshot.

## Interaction corrections from desktop screenshots

Writing-layout actions now live in Workspace, without duplicate Split and Focus
buttons in the toolbar. Map zoom controls sit together beside beat search and
move below it in a narrow pane. Hovering a card reveals replies on both its
incoming and outgoing connections.

Scene search uses a shorter, dimmer placeholder and tighter icon spacing. Its
clear button appears only when there is text to clear and retains the shared
Button's pointer and focus behaviour. Settings uses a six-tooth cog and closes
on backdrop clicks through the shared Dialog dismissal callback.

Command-palette scrolling no longer resets on layout updates. Command, project
and scene searches and source completion share the same guarded selection-reveal
hook. The palette regression fails with the old effect and passes with the guard.
The control review found ordinary actions already using Button or IconButton;
remaining direct pointer handlers cover editor focus, graph gestures, splitters
and propagation boundaries.

For desktop testing, scroll well down the command list and leave it idle, then
filter it and navigate with the arrow keys. Check both appearances, the empty and
filled scene search, incoming reply labels, Workspace layout actions, and clicking
inside and outside Settings. Test captures and headless input checks do not
establish physical trackpad or window-system behaviour.

## Keyboard, size and motion

At a normal window size and about 900 by 650, repeat the important steps using
only the keyboard. Test Tab/Shift+Tab, Enter, Escape, the displayed shortcuts,
search, destination selection, menus and every close-dialog action.

Expected: visible focus, no focus behind a modal, a way out of every menu, and
focus returning somewhere useful. Repeat with the Vim keymap; typing in an editor
must not trigger navigation shortcuts.

Try both appearances, reduced motion, 200% display scaling and long paragraphs
with long scene names. Controls must remain reachable and text must not cover
actions. Reduced motion should remove movement without hiding state changes.

## Native accessibility and input

On each platform claimed by the milestone (Linux, Windows and macOS), record OS,
display scale, and the input/accessibility tools used:

- Screen reader: navigate the sidebar, edit text, select destinations, open a
  dialog and provoke an error. Names, selected states, errors and focus changes
  should be announced. Check that graph navigation has a usable textual route.
- IME: compose and commit text in a manuscript field, source and translation.
  Candidate windows should follow the caret; composition should not submit a
  dialog or invoke shortcuts.
- Mixed RTL/LTR: edit a real Arabic or Hebrew sample with punctuation and Latin
  identifiers. Check caret movement, selection, copy/paste and reopened text.
- High contrast and display scaling: verify selection, focus, errors and disabled
  states remain distinguishable.

Unavailable hardware or an unfamiliar input method is an untested item, not a
pass. Send screenshots or a short recording for visual/focus problems, plus exact
reproduction steps. Do not send private project content unless intended.

## Acceptance record

For each session, record date, OS, build/commit or dirty-checkout identity,
window size/scale, result, and reproduction steps for failures. Automated gate
results are reported separately in the implementation handoff. No packaging,
installer, signing or distribution acceptance is implied.
