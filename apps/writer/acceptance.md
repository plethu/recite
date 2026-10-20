# Writer acceptance

Writer is a preview. Native authoring, accessibility, and installed-package acceptance still need
host evidence. Automated tests and captured renders do not establish physical input, screen-reader
usability, GPU performance, or cross-platform support.

## Repeatable checks

Run from the repository root:

```sh
mise exec -- just writer check
mise exec -- just writer accessibility
mise exec -- just writer probe-accessibility
mise exec -- just writer probe-xilem target/writer-native/result.json
```

The [Writer gate](justfile) owns automated coverage, including focused accessibility tests. These
checks verify model transitions and accessibility metadata; native announcements, physical input and
usability still require the drills below. Use the [packaging matrix](packaging.md) for install,
upgrade, uninstall and desktop activation.

The Linux AT-SPI probe requires native Focus and Click to pass separately: focus must change without
activation, then Click must publish the changed action status. It needs a focused window; a headless
Wayland compositor also needs an active virtual keyboard seat. The local Freya core patch publishes
changed accessibility properties for stable nodes. Passing this probe does not establish
screen-reader usability.

## Known limits

The opt-in Xilem migration frontend has passed native Wayland checks in a private Cage compositor
for typing undo/redo, clipboard exchange with Unicode, checked saves, literal IDE arguments,
explicit reload with an independent recovery copy, document switching with retained drafts, native
undo isolation between equal-text fields, and multi-document close/reopen recovery. Source-language
preview also covers choices, typed condition answers, blocking-effect acknowledgement, stale-input
notices, restart refusal with an unapplied draft, and native typing history after hiding playback.
Structured authoring covers creating beats, lines and replies, guarding unapplied drafts, reading
conditional groups and effects, and editing previously empty beats through native controls. Its
accessible manuscript name and final layout are checked separately. Welcome/project creation,
retained temporary examples and native second-launch document selection also pass. Personal settings
cover live light/dark/monochrome colours, 100–200% scale, retained manuscript undo, validated
shortcut edits, persisted configuration and ordinary exit confirmation. Native keyboard checks cover
focused Apply, immediate Save, live rebinding and retained text undo/redo. The native settings drill
traverses and activates controls after scrolling; adapter regressions cover clipping and
registration ordering. This does not establish workspace parity: localisation and configured preview
inputs, schema and workspace command dispatch remain in progress. Physical IME, screen-reader
announcements, hardware rendering and Windows/macOS acceptance remain unverified for Xilem. The
portal scrollbar precedes close choices in the Tab cycle; the current native drill reaches Keep
editing in three presses and verifies that cancelling close retains text undo/redo. Automatic focus
return and the remaining keyboard workflow need acceptance before the default switches.

Shared scene/beat lookup, command ranking and Vim save/close aliases now drive both frontends. The
native lookup drill covers query autofocus, Escape focus return, retained manuscript undo/redo,
checked Save from command search, keyboard scene selection, Back/Forward and saved-project text
search. Result controls are paged and scroll independently from the query and pagination controls.
Placeholder scaling and serif reading typography have separate 100/200% render checks; those
captures do not establish assistive-technology usability.

Native Focus writing stashes surrounding panes without replacing manuscript widgets. Its Script, Map
and Split lifetime regression preserves native undo at 100/200%. The owned Wayland focus drill
covers undo/redo, nested lookup restoration, directional Vim panes, prefix cancellation, standard F6
cycling, window blur and revealing the previous focus target after outer scrolling. A host signal
delivery regression requires F6's visible and AT-SPI focus to update without additional input. These
Focus and prefix states have separate render review. Native n/N repeats accepted scene/beat lookup,
including project-wide targets; Freya retains its filtered scene-navigation surface. Both share
ordered wrapping/fallback selection. Native checked publication preserves the cursor after
cancellation, failed Apply or newer typing, and cancelling another picker preserves the query.
Saved-text results do not replace this cursor because individual text hits have no stable identity.

Native project tools load declarations/build targets on the navigation worker and retain unapplied
manuscript drafts. A fixed 1,026-declaration fixture mounts five result controls within 128 native
accessibility nodes at 100/200%; details and build reports also bound a single long Unicode line.
The owned Wayland drill covers declaration query focus, selected detail, returning with native text
undo, disk-only builds, ordered save-all/build, and cancellation retirement before closing. These
inspection/build states have separate render review, including scrolled 200% declaration detail.
Declaration source binding/editing and registered producer actions remain unfinished.

A queued accessibility action may outlive its native widget. Recite checks the live owned base layer
before dispatch; the stale-click regression also requires replacement focus/click actions to work
before another redraw. This bridge currently assumes one native window and its base layer.

Xilem saves, cold project loads and editor handoff use workers and retain newer typing;
save-and-close timing is recorded by the native probe. Preferred-application lookup, an
independently open editor, checked-save conflict refusal and missing-executable retry also pass the
owned native editor drill. File and selection navigation, Apply and applied history now prepare on
workers, with regression checks for cancellation, newer typing, outgoing recovery and deferred Save.
Structural creation and destination changes also prepare on workers, preserving history and
rejecting stale or cancelled publication. The owned native drills cover prose Apply before creation,
reply and continuation routes, separate applied undo/redo, stable IDs and explicit disk Save.
Dynamic preview resizing has a Portal scrollbar regression and a native scrolling drill. Reload now
prepares discovery, disk content and an independent recovery archive on a worker, with checked
adoption and separate recovery-cleanup failure reporting. Shared and native regressions cover newer
typing, cancellation, retained drafts/history, changed project roots and cleanup retry. Its fixed
10,000-passage workload shares source/context allocations during capture. The owned Wayland Reload
drill verifies complete recovery export, fresh native field history and post-reload typing across
restart; pending states have separate 100/200% render checks.

Source-draft repair now exports captured text plus a complete recovery backup, opens the preferred
editor and imports through checked authoring preparation. The owned Wayland drill verifies invalid
import refusal, close/restart recovery, literal editor paths and explicit Save after valid import.
Cancellation and newer-typing regressions preserve the current draft; unrepaired retained drafts
also block Save All and implicit Apply through links. The 100/200% recovery states have separate
visual review. This enables recovery without an embedded source editor.

Native reply-rule controls now use the shared source-preserving staging owner. Their owned Wayland
drill covers picker cancellation with native undo, Apply without replacing the argument widget or
writing to disk, invalid Save refusal, declared registry selection, checked Save with comments and
IDs preserved, and return to reply text. Focused regressions cover stale choices after effect
removal, repeated Apply followed by applied undo, and deferred Save through a disk conflict. A
1,026-effect workload mounts four argument fields and stays within 128 accessible nodes per page.
Rule fields are paged: native text undo survives continuously mounted fields; leaving a field page
ends its local text history. Applied undo remains available for accepted edits. Projection and
controls share one reply-session lifetime; reopening rejects old choice receipts. Renderers derive
the authoring surface from Workbench selection and compose resolved controls without changing page
state. Picker freshness stamps retain no compiler or project graph. Run this drill with `mise
exec -- just writer probe-xilem target/writer-native/rules.json rules`.

The native Map drill covers Map/Script continuity with native undo, spatial selection with native
focus, keyboard placement persisted outside dialogue source, Go to, Enter into Script and clean
close on owned Wayland. Short-window regressions reveal both the selected card and move handle at
100/200% UI scale. Capture survives dragging beyond the canvas; Escape discards the preview.
Placement failures keep the in-memory move for retry, and cancellation reconciles completed writes
without replacing newer text or opening a cancelled workspace. Script and Map share stable pane
slots. Independent 100/200% Map/Split render review covers controls, canvas guidance and manuscript
alignment.

The fixed 512/2,048-beat workloads each mount three detail cards and 234 accessibility nodes. Their
whole-scene Fit mounts zero prose cards and 173 nodes. Overview clicks and spatial navigation
restore a readable card; pan completion does not activate a beat. Removing focused cards transfers
focus to the persistent viewport, while toolbar focus stays put. These bounded checks do not
establish hardware rendering performance or physical trackpad acceptance. Run the Wayland drill with
`mise exec -- just writer probe-xilem target/writer-native/map.json map`.

The Writer dependency audit also remains red: the pinned graph includes unmaintained SVG/font and
D-Bus dependencies, and the existing image graph includes a license outside the current allowlist.
The Xilem addition introduces `rustybuzz` through `usvg`; the other reported package versions were
already in the Freya lockfile. No dependency-policy exceptions were added. The limits below describe
the default Freya application.

- The pinned CodeEditor exposes an unnamed inner TextInput and does not display IME preedit.
  `probe_records_missing_code_editor_preedit_presentation` records the gap; a passing test does not
  mean IME support passes. After upgrading Freya, replace it with a positive assertion once fixed.
  Multiline Input has a separate positive composition test. The existing upstream report is
  [Freya #2248](https://github.com/marc2332/freya/issues/2248); the maintainer asked us to pause
  duplicate upstream patch work.
- Exact prose caret/selection restoration after Input remounts is deferred pending a supported
  upstream approach; see [Freya #2308](https://github.com/marc2332/freya/issues/2308). Draft
  retention and Source caret restoration remain required.
- The backend does not forward native trackpad pinch. Ctrl+scroll zoom tests do not establish
  physical gesture support or smooth trackpad scrolling.
- ARM Linux, native macOS/Windows, screen readers, physical IME/BiDi, display scaling and sustained
  large-project sessions still need recorded host evidence.

The AT-SPI probe uses a private D-Bus session and temporary preferences. It needs Linux
AT-SPI/Python bindings and a display or Xvfb. Neither it nor package inspection installs the app or
changes the user's desktop associations.

## Start here: a real writing session

Use a copy of a real project. Launch from the repository with `mise exec -- just writer run`, then
open that project.

Spend 30–45 minutes doing ordinary work:

1. Create a beat, write several lines and replies, change destinations and use previews. Navigate
   with the sidebar, graph and history.
2. Undo and redo, switch Map/Source, edit source, apply it, save and reopen.
3. Open a PO catalogue, translate several passages, mark one reviewed, use the queue, and leave
   drafts in more than one beat.
4. Quit. The dialog should name those drafts and offer working navigation, save-all and discard-all
   actions. Cancel once, then save and quit.
5. Reopen and verify the saved text and destinations.

Report anything that made you hunt for a control, lose your place, or wonder whether work was saved.
Include the steps and the expected behavior.

## Recovery and conflict drill

Use disposable copies and one writer window per file.

1. Change a translation without saving. Wait two seconds, terminate that writer process forcibly,
   then reopen the same catalogue. The draft should return.
2. Bind a standalone declaration TOML source. Leave incomplete TOML, wait, force termination, and
   reopen and bind the same source. The incomplete text should return. Source association itself is
   not persisted.
3. Repeat with a manuscript field draft.
4. With a recovered draft open, change the corresponding file externally. Saving must report the
   conflict and retain the draft, not overwrite the other editor's work.
5. Discard a recovered draft, reopen again, and confirm it stays discarded.

Recovery is asynchronous: a process killed before the next snapshot reaches disk can lose the latest
edit. It must not corrupt the last durable snapshot.

## Keyboard, size and motion

At a normal window size and about 900 by 650, repeat the important steps using only the keyboard.
Test Tab/Shift+Tab, Enter, Escape, the displayed shortcuts, search, destination selection, menus and
every close-dialog action.

Expected: visible focus, no focus behind a modal, a way out of every menu, and focus returning
somewhere useful. Repeat with the Vim keymap; typing in an editor must not trigger navigation
shortcuts.

Try both appearances, reduced motion, 200% display scaling and long paragraphs with long scene
names. Controls must remain reachable and text must not cover actions. Reduced motion should remove
movement without hiding state changes.

## Native accessibility and input

On each platform claimed by the milestone (Linux, Windows and macOS), record OS, display scale, and
the input/accessibility tools used:

- Screen reader: navigate the sidebar, edit text, select destinations, open a dialog and provoke an
  error. Names, selected states, errors and focus changes should be announced. Check that graph
  navigation has a usable textual route.
- IME: compose and commit text in a manuscript field, source and translation. Candidate windows
  should follow the caret; composition should not submit a dialog or invoke shortcuts.
- Mixed RTL/LTR: edit a real Arabic or Hebrew sample with punctuation and Latin identifiers. Check
  caret movement, selection, copy/paste and reopened text.
- High contrast and display scaling: verify selection, focus, errors and disabled states remain
  distinguishable.

Unavailable hardware or an unfamiliar input method is an untested item, not a pass. Send screenshots
or a short recording for visual/focus problems, plus exact reproduction steps. Do not send private
project content unless intended.

## Scale and performance

Use the
[maintained workloads and profiling commands](../../docs/profiling-and-optimisation.md#writer-workloads).
Model latency and bounded headless rendering do not establish native GUI performance. Discovery,
indexing and cold validation scale with corpus size; high-fan-out changes may invalidate many files,
and topology changes still affect a whole scene. Source view opens the whole document.

Before claiming RPG production scale, measure branching/fan-out, cycles, cross-scene references,
long prose, diagnostics-heavy projects and prolonged editing. Functional coverage is not performance
evidence for every combination. Measure end-to-end open/discovery, scene switching, edit-to-paint,
search-to-passage reveal, memory over hours and crash recovery on declared platforms. Include real
input/rendering, screen-reader navigation across virtual rows and IME composition.

Proposed targets on named hardware remain 16.7 ms camera/guide frames, p95 edit-to-paint below 50
ms, and warm search results below 100 ms. Current results do not establish those targets.

## Acceptance record

For each session, record date, OS, build/commit or dirty-checkout identity, window size/scale,
result, and reproduction steps for failures. Automated gate results are reported separately in the
implementation handoff. No packaging, installer, signing or distribution acceptance is implied.
