# Writer acceptance

Writer is a preview. Native authoring, accessibility, and installed-package acceptance still need
host evidence. Automated tests and captured renders do not establish physical input, screen-reader
usability, GPU performance, or cross-platform support.

The paired human comparison on 2026-10-21 rejected Xilem's presentation readiness. The functional
checks below did not establish interchangeability: Xilem still showed a form-like scaffold in place
of Freya's fixed scene rail, compact toolbar and whole-beat manuscript with inline editing. The
writing composition has since been corrected against Freya's existing design. A retirement
comparison needs equivalent rendered and interaction evidence for the same scene, selection,
viewport and scale, including pending edits, focus mode, Map, Split and the remaining project
screens. Narrow fixture or recovery render reviews do not satisfy that comparison.

The initial bounded paired review on 2026-10-22 recommended keeping both frontends available. A
fresh reviewer inspected 44 actual cold-route renders of both frontends at 100/200% scale, using the
same Relay Hub fixture and 1280×720 compositor (1280×690 application windows). This covered
declarations, builds, rename, disk routes, catalogue passage/queue/entry/source-update routes,
personal preferences, project settings and shortcuts. The shared project header was compacted while
retaining its actions and document identity. The subsequent bilingual workspace slice corrects the
three largest gaps from that review:

| Corrected gap                                      | Evidence and limits                                                                                                                                                                                                                         |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Localise now presents the whole bilingual beat.    | Source-ordered dialogue, replies, destinations, incoming context, status and independently keyed translation fields; structural records remain read only. Eight records mount per page, with plural forms paged separately.                 |
| Translation queue exposes its content immediately. | Inline search/filter controls and prose-first source/status rows; catalogue file administration is secondary.                                                                                                                               |
| Auxiliary screens retain scene navigation.         | Persistent scene rail and current document context, with a Scenes picker at scales where the rail collapses. Actual beat/file navigation keeps checked Apply; ordinary Localise opening preserves unapplied manuscript and source recovery. |

A fresh visual reviewer inspected all eight paired `paired-bilingual-v4` renders at 100/200%,
against the accepted Freya reference, and closed the original findings for this slice. Prose scales
with the controls. Native bilingual drills exercise independent Undo/Redo, focused-row Save, frozen
worker checkout, checked beat navigation and explicit missing-entry states. Mounted tests cover
plural arms, source ordering, catalogue replacement, hidden/stale callbacks and a constant native
field count for 32 versus 1,024 passages. These results establish this slice, not whole-product
interchangeability or human acceptance.

The auxiliary follow-up now corrects the surfaced composition gaps. Personal preferences include a
live manuscript specimen, compact reading/layout controls and a searchable shortcut list beside its
editor. Project settings identify the saved manifest and separate export, import, review, Apply and
reload recovery. Rename uses a declared-beat chooser, a compact retained proposal and source-font
before/after changes. Its staged result exposes grouped Undo; successful Save all retires staging
guidance without retiring history. Build identifies its saved project scope, selected output targets
and actual generated files. Native drills cover both rename scales, selected/all-output publication,
settings recovery and persistent preferences without replacing manuscript history. Fresh visual
review closed the surfaced settings, rename and build findings, including actual TOML bodies, 200%
lower preferences and enabled monochrome controls.

Populated Relay Hub source-update pairs now exercise an actual changed passage on both frontends at
100/200%. Explicit PO publication preserves the translation and translator note and does not write
source. The completed Xilem queue shows the changed-source review status and saved outcome. Compared
words use native bold and underline, including in monochrome. The native read-only Label used for
these comparisons retains complete ordered accessibility text and wrapping; it does not provide
Prose's pointer selection/copy interaction. Editable manuscript and translation fields retain their
native selection and history. This is a documented comparison-only capability limit.

Populated disk pairs exercise inspected Keep decisions at both scales while preserving changed disk
bytes and creating a complete recovery archive. Their first visual follow-up found an unresolved
choice form remaining after completion. The corrected presentation uses a typed, source-bound
resolved state, preserves late recovery/presentation warnings, and retires unchosen options. Failed
reinspection can retain a result for the same document; another document cannot inherit it. Final
resolved-state native renders received a separate review, which closed the completed-state finding
at both scales, including the visible outcome heading and preferred Return action at 200%.

The local evidence index is `target/writer-evaluation/retirement-evidence.md`. It names completed
receipts, failed attempts, source/binary identities and carried-forward baseline evidence.
Diagnostic renders and workflow receipts do not establish physical input, assistive-technology
usability, installed-host lifecycle acceptance or a passing dependency policy. Both frontends remain
available for the final human evaluation; these corrections alone do not authorise Freya's
retirement.

The installed-package and broad native claims below describe signed baseline
`9ce876da7f6b0945a2a662eba4951a14fd74bd02`. They have not all been repeated after the inline
manuscript and shell correction. Treat current focused checks separately from that earlier full gate
and package evidence.

The writing correction now composes a fixed scene rail, compact toolbar and status bar, whole-beat
inline manuscript fields, reply destinations, a retained reference dock and bounded document
actions. Map and Split reuse the same retained writing fields. Preview occupies the editor
workspace, with separate persistent actions to return to writing or close its input controls. Its
run and staged inputs survive those transitions. An independent review of actual 1280×720 Wayland
renders at 100/200% UI scale closed the reported writing, Map/Split, reference, menu and Preview
composition findings. The final Preview follow-up inspected seven states at each scale, including
real Tab entry to a fully visible language field. These findings cover the writing and Preview
surfaces; they do not establish whole-product interchangeability, installed-package acceptance for
this correction or physical keyboard/screen-reader usability. The auxiliary and bilingual follow-ups
above extend this comparison; the evidence index records the exact checks and their limits.

The corrected Linux release binary was also packaged and checked for payload, ABI, licenses and CLI
behavior. Its extracted desktop entry passed cold/running-window link delivery and malformed-route
rejection in an owned desktop session without changing fixture bytes. This refresh covers the
extracted package; it does not repeat package-manager install/upgrade/uninstall or the earlier
installed Flatpak acceptance.

The local Xilem Linux release package passes payload, ABI and CLI inspection, native Wayland focus
checks, and actual desktop-entry delivery on owned X11 and Wayland hosts. Cold and running-window
links preserve fixture bytes; startup may return a retryable busy response until navigation is
ready. The native Nix package also builds and passes its headless CLI checks. Xilem's Flatpak builds
from locked offline sources, exports, installs and upgrades in an isolated profile. Installed
Wayland checks cover native text history, Unicode clipboard/save, file switching, recovery,
configured IDE handoff, producer publication/cancellation, preview and structural authoring. Native
links round-trip at 100/200% scale. Both manifests grant the specific screen-reader status service;
the installed accessibility tree uses Flatpak's separate AT-SPI proxy. These checks use an owned
compositor and private bus. Physical-host lifecycle acceptance and other native platforms remain
unverified; Freya's full Flatpak source build has not been repeated.

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

The default Xilem frontend has passed native Wayland checks in a private Cage compositor for typing
undo/redo, clipboard exchange with Unicode, checked saves, literal IDE arguments, explicit reload
with an independent recovery copy, document switching with retained drafts, native undo isolation
between equal-text fields, and multi-document close/reopen recovery. Source-language preview also
covers choices, typed condition answers, blocking-effect acknowledgement, stale-input notices,
restart refusal with an unapplied draft, and native typing history after hiding playback. Structured
authoring covers creating beats, lines and replies, guarding unapplied drafts, reading conditional
groups and effects, and editing previously empty beats through native controls. Its accessible
manuscript name and final layout are checked separately. Welcome/project creation, retained
temporary examples and native second-launch document selection also pass. Project replacement
refuses translation drafts and succeeds after Save while the catalogue remains open. Automatic disk
notices pass native source and PO replacement workflows at 100/200% scale, retaining pending drafts
and source text undo/redo through explicit comparison. Focused regressions cover save races,
checkout, same-path catalogue reopening, queued stale actions and changes during an observation.
Idle watching posts no UI work; notice rendering mounts a fixed number of widgets with 1,026 or
10,260 retained notices. Focused cold-load tests also reject publication after closing begins.
Personal settings cover live light/dark/monochrome colours, 100–200% scale, retained manuscript
undo, validated shortcut edits, persisted configuration and ordinary exit confirmation. Native
keyboard checks cover focused Apply, immediate Save, live rebinding and retained text undo/redo. The
native settings drill traverses and activates controls after scrolling; adapter regressions cover
clipping and registration ordering. Catalogue editing now covers native review/save, retained
manuscript undo, inspected PO conflict decisions, source-plus-catalogue Save All and close/reopen
recovery. Separate native checks exercise command search and catalogue editing at 200% UI scale with
32px reading text. Catalogue creation and source refresh additionally cover explicit language
selection, observational review, stale-source refusal, PO-only publication and mounted manuscript
retention. Configured preview and trace inspection pass owned Wayland workflows at 100/200% UI
scale, including typed arguments, translated/plural output and record retention across typing and
failed restart. Successful restart resets the trace cursor. Shared render regressions bound
retained-history and large-value cost; separate visual review covers both frontends' record and
value pages. A consumer iterator regression also covers reachable scrollbars following pruned
siblings. Command dispatch is exhaustive, and linked destinations cover every shared screen. Shared
workflow checks and all four Writer production-line coverage floors pass. Dependency-policy
resolution remains required. Physical IME, screen-reader announcements, hardware rendering and
Windows/macOS acceptance remain unverified for Xilem. The portal scrollbar precedes close choices in
the Tab cycle; the current native drill reaches Keep editing in three presses and verifies that
cancelling close retains text undo/redo. Freya remains an explicit fallback through `just writer
freya`, native packaging's `--frontend freya`, the Freya Flatpak manifest and Nix's
`recite-writer-freya` output. Selecting Xilem does not waive the dependency audit or establish
installed-host acceptance.

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
Registered producer startup, checked output publication and project-analysis refresh run on workers.
The owned Wayland producer drill covers completion after returning to writing, retained native
typing and undo, cancellation retaining published output, and recovery close after terminal
retirement. Shared/native regressions also cover foreign and repeated receipts, failed dispatch,
publication conflicts, cancellation before/after writing, and refresh failure followed by retry. A
staged Rules draft survives a schema change with its validation policy and external-repair route
retained. Producer ready/busy states have 100/200% render review; command details and reports are
paged. Source binding, invalid TOML recovery, export to a separate IDE artifact, explicit import and
standalone generation pass a separate owned Wayland drill while retaining manuscript typing and
undo. Source/recovery states have independent 100/200% render review. Producer-registration reload
replaces the reviewed command or its error without execution; generated-output reload rejects
malformed JSON and supports a valid retry. Shared/native regressions preserve newer TOML typing
through these metadata reloads. Output changes clear cached source-currentness until explicit draft
validation.

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

The fixed 512/2,048-beat workloads each mount three detail cards and 237 accessibility nodes. Their
whole-scene Fit mounts zero prose cards and 185 nodes, within the unchanged 240-node cap. Overview
clicks and spatial navigation restore a readable card; pan completion does not activate a beat.
Removing focused cards transfers focus to the persistent viewport, while toolbar focus stays put.
These bounded checks do not establish hardware rendering performance or physical trackpad
acceptance. Run the Wayland drill with `mise exec -- just writer probe-xilem
target/writer-native/map.json map`.

The `linked-destinations` native scenario exercises cold passage links, existing-window stable PO
entry links, and cold/forwarded Source links at 100% and 200% UI scale. Source activation must
finish when the configured editor starts, while that editor remains open. Cold launch must resume
when its workers become ready without additional user input. Private native tests also cover all
twelve linked screens, catalogue replacement refusal, cancelled and failed owner returns, logical
source-review pages, recovered Source repair, and source-handoff failure/teardown paths.

The `clipboard` scenario checks Unicode project/scene paths and catalogue queries at 100/200% UI
scale, preserving unapplied manuscript text. The independent `specimen` scenario checks production
controls, search, native text undo, appearance and scale without reading or changing personal
preferences. Checkbox/radio indicator geometry and strokes follow UI scale; a mounted-control
regression checks reflow, focus retention and actions through 100→200→100%. Reading/source text
sizes remain independent preferences. Actual native renders receive separate visual review.

The `localisation-final` scenario checks translation-field Ctrl/Cmd+S, ordinary catalogue history
with Alt+Left/Right, and explicit discard of multiple PO drafts through close/reopen at 100/200% UI
scale. Source bytes remain unchanged. Focused regressions cover stale field targets, restored
query/filter/page state, cancelled history, direct return during a pending history visit, and
recovery failures both before and after discard. Project/catalogue headers reuse the common
Back/Forward controls.

The compatible Tokio patch release removes the yanked 1.54.0 pin without changing other package
versions. The locked Flatpak source archive follows the same checksum. The Writer dependency audit
also remains red: the pinned graph includes unmaintained SVG/font and D-Bus dependencies, and the
existing image graph includes a license outside the current allowlist. The Xilem addition introduces
`rustybuzz` through `usvg`; the other reported package versions were already in the Freya lockfile.
No dependency-policy exceptions were added.

Selected-package production/build graphs distinguish the retirement benefits from the shared work:
Xilem avoids Skia and Freya's default image codecs, including AVIF. On Linux it also avoids `paste`;
macOS retains `paste` through Metal. SVG/font maintenance findings remain on all three desktop
targets, and Linux retains the D-Bus findings. The combined all-feature audit remains the enforced
gate. The NCSA finding belongs to a conditional fuzzing dependency in the AVIF graph, not to a
fuzzer shipped in the runtime.

The font finding also reaches Xilem through winit's Linux window-decoration graph. The D-Bus
findings span AccessKit, portals and desktop preferences; replacing one crate does not migrate all
consumers. These require coordinated compatible upstream updates. No additional compatibility fork,
lint waiver, advisory exception or license exception was added to clear the audit.

The limits below describe the Freya fallback.

- The pinned CodeEditor exposes an unnamed inner TextInput and does not display IME preedit.
  `probe_records_missing_code_editor_preedit_presentation` records the gap; a passing test does not
  mean IME support passes. After upgrading Freya, replace it with a positive assertion once fixed.
  Multiline Input has a separate positive composition test. The existing upstream report is
  [Freya #2248](https://github.com/marc2332/freya/issues/2248); the maintainer asked us to pause
  duplicate upstream patch work.
- Exact prose caret/selection restoration after Input remounts is deferred pending a supported
  upstream approach; see [Freya #2308](https://github.com/marc2332/freya/issues/2308). Draft
  retention and Source caret restoration remain required.
- An owned Wayland recovery comparison observed advertised AT-SPI objects that returned
  `UnknownObject` in Freya's full writing workspace. The native drill could not reach the restored
  prose field reliably. Its snapshot was visibly restored, and focused regressions verify retained
  prose and Source drafts across saved view preferences; this does not establish native
  accessibility or a complete native cross-frontend recovery pass.
- The backend does not forward native trackpad pinch. Ctrl+scroll zoom tests do not establish
  physical gesture support or smooth trackpad scrolling.
- ARM Linux, native macOS/Windows, screen readers, physical IME/BiDi, display scaling and sustained
  large-project sessions still need recorded host evidence.

The AT-SPI probe uses a private D-Bus session and temporary preferences. It needs Linux
AT-SPI/Python bindings and a display or Xvfb. Neither it nor package inspection installs the app or
changes the user's desktop associations.

## Start here: a real writing session

Use separate copies of a real project for the two frontends. Launch Xilem with `mise exec -- just
writer xilem`, then repeat with `mise exec -- just writer freya`. Close the first before launching
the second: they share single-instance routing, preferences and recovery. Use separate
`RECITE_CONFIG` paths if you want to compare their settings independently.

Spend 30–45 minutes doing ordinary work:

1. Create a beat, write several lines and replies, change destinations and use previews. Navigate
   with the sidebar, graph and history.
2. Undo and redo, switch Script/Map/Split, save and reopen. In Xilem, use **Open in editor**, make a
   source edit, then compare or reload it while retaining recovery. In Freya, also edit in Source
   and apply it. Compare the effort of these two source workflows.
3. Open a PO catalogue, translate several passages, mark one reviewed, use the queue, and leave
   drafts in more than one beat.
4. Quit. The dialog should name those drafts and offer working navigation, save-all and discard-all
   actions. Cancel once, then save and quit.
5. Reopen and verify the saved text and destinations.

Report anything that made you hunt for a control, lose your place, or wonder whether work was saved.
Include the steps and the expected behavior. After the separate runs, keep an unsaved recoverable
draft in one frontend, close it, and reopen that same project in the other. Verify the draft and
stable IDs, then repeat in the other direction. Keep both only for a named capability or usability
gap; record what would close that gap before retiring the fallback.

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
