# GUI bake-off evidence · 2026-09-07

This records observations, not milestone acceptance. The experiment uses Linux,
Rust 1.96, Freya 0.5.0-rc.4, gtk-rs 0.11.4 and native GTK 4.22.4. Both windows
launched on Hyprland Wayland. The advertised X11 display refused connections;
that is an environment result, not a compatibility verdict.

## Freya release assumption

Continue evaluating Freya on the Recite maintainer's assumption that its
CodeEditor preedit bug will be fixed by 0.5 final. The pinned RC still exhibits
the bug. Replace the negative probe with a positive regression check when the
fix is available; revisit the assumption if it persists in the release.

Marc has picked up [#2248](https://github.com/marc2332/freya/issues/2248).
Our upstream fix work is paused. The [handoff](freya-upstream-handoff.md) keeps
the exact reproduction, contribution-policy audit and follow-up context.

Our initial assessment missed `Input::multiline(true)` in this same RC.
Script now uses that control, which supports wrapping and preedit; Source
retains CodeEditor. The earlier claim that Freya lacked a wrapping prose
control was wrong.

## Current comparison

| Capability | Freya / Linux | GTK / Linux |
| --- | --- | --- |
| Whole-scene reading and in-place editing | Rendered; app interactions tested | Rendered; native widget interactions tested |
| IDs behind details; highlighted Source | Implemented; Source/Script round trip tested | Implemented; Source/Script round trip tested |
| Light/dark palette | Both captured | Both captured |
| Prose wrapping | Input layout test confirms wrapping without inserted newlines | TextView WordChar enabled; capture inspected |
| Unicode input | Input and CodeEditor component tests | Shared model tests; physical input pass outstanding |
| IME preedit | Input positive component test; CodeEditor release assumption above | Native mechanism available; physical IME pass outstanding |
| Keyboard/focus | Input Tab escape tested; earlier desktop Source activation | Native focus recovery tested; earlier desktop Source activation |
| Draft protection | Apply and refused navigation tested in app | Apply and refused navigation tested through native widgets |
| Undo | Shared applied history tested; widget undo across fields open | Shared applied history tested; widget undo across fields open |
| Screen reader / editable-text actions | Unproven | Unproven |
| Narrow window / 200% text / system appearance | Open | Open |
| Saving / external-change recovery | Outside this slice | Outside this slice |
| Conditions / effects / schema UI / localisation | Outside this slice | Outside this slice |

## Verification

Seven shared authoring tests cover Unicode/CRLF preservation, guarded edits,
effective speaker metadata, stable choice IDs, real runtime preview, and draft
recovery. Six Freya tests cover the text controls and application interactions.
One additional GTK test requires a display and runs separately with `--ignored`.
It sends native widget signals and checks focus targets; it is not a physical
keyboard or IME test. A passing CodeEditor negative probe records the known gap,
not successful composition support.

Workspace formatting, strict Clippy, 13 ordinary tests, the additional native
GTK test and both builds pass. Git policy and test-organisation checks also pass.
The native GTK test required desktop access outside the sandbox.
No production Rust source or root dependency lock changed. Independent read-only
review identified duplicate-speaker handling, adapter status and focus issues;
those were corrected and re-reviewed.

[Matching captures](visual-review.md) show 1200 × 800 views. Freya uses its test
renderer; GTK uses native child-widget snapshots on the application palette's
canvas. The capture script now isolates GTK in a uniquely matched floating
window on a silent workspace and checks its actual allocation. Freya is
offscreen. The previous GTK forced child allocation was removed; background rendering is enabled only for the
capture window so GTK completes its normal layout and theme updates. Regeneration passes with GTK
warnings treated as failures. Neither includes desktop chrome or proves
accessibility conformance.

## What remains

Freya's multiline Input makes it a stronger writing candidate than our first
comparison suggested. No toolkit is selected. Clipboard behaviour, widget undo
across passage changes, physical IME/BiDi input, screen readers, narrow layouts
and text scaling still need evidence. Keep source preservation, recovery,
keyboard escape, editable-text accessibility and usable IME as gates.

See the [Rust candidate findings](rust-candidates.md) for the subsequent Floem,
GPUI, Xilem/Masonry and Slint pass. macOS and Windows native lanes remain queued. This GTK entry evaluates Linux; it does not establish cross-platform
suitability. Among candidates that pass the gates, compare writer task
completion and assistance needed, reading and editing comfort, then maintenance,
packaging and resource use. Unmeasured cells receive no scores.

## GPUI writer pass · 2026-09-07

GPUI now includes whole-scene in-place editing, speaker/destination controls,
details with stable IDs, Add choice, applied undo/redo, guarded Source/Script
switching, diagnostics and real runtime preview. Input Change events update
draft state and stale-preview feedback; commands refuse active preedit.

Four GPUI tests, build, formatting and all-target Clippy pass. The writer test
covers speaker/destination outcomes, stable/new IDs, applied history and
reactive draft/preview state. The composition regression checks that Apply
during preedit leaves the authoritative document unchanged. Independent review
is complete after fixes. Test-organization and Git-policy scripts pass.

Four native screenshots were captured on silent floating Hyprland workspaces,
with fixed 1200 × 800 logical allocation and recorded fractional-scale pixels.
See [visual review](visual-review.md#gpui). These are rendering evidence, not
physical input or accessibility acceptance. Keyboard traversal, widget undo
shortcuts, installed IME, screen readers, narrow/scaled layouts and other
operating systems remain open. No production code, commits, pushes or upstream
posts form part of this pass.

## Interaction pass · 2026-09-07

The current focused run passes 21 tests: seven GPUI, seven Freya and seven
shared authoring tests. This count includes the existing negative Freya
CodeEditor preedit probe; it does not mean that gap is fixed.

New GPUI evidence covers mock clipboard paste through Ctrl+V, Ctrl+Z,
isolation from an earlier passage with undoable applied edits, rejected prose
recovery, and runtime traversal of both fixture branches. A Root-mounted test
found that the default multiline Tab binding indented prose. The adapter now
maps Tab/Shift+Tab to focus traversal only beneath its Script prose context;
the regression checks escape, unchanged text and reverse traversal. Source
keeps its existing bindings. This is not full keyboard-route acceptance.

Freya now has an explicit prose Ctrl+Z test and app-level applied Undo/Redo
assertions following a Script/Source round trip. Its existing wrapping,
Tab escape and composition tests still pass.

Both candidate builds, focused all-target Clippy with warnings denied,
formatting, test organization and Git policy pass. The manual launcher was
smoke-tested for each native app at 1200 × 800 on silent floating workspaces,
without desktop input. It cleans up its process group and temporary rule.

The [hands-on trial](interaction-pass.md) owns the remaining Linux system
clipboard, physical IME, full keyboard/focus/scroll routes, screen-reader and
macOS evidence. Those results are unrun; this pass does not select a framework.

## Freya selection and file lifecycle · 2026-09-08

The maintainer selected Freya and ended further candidate implementation.
See the [decision](../../docs/decisions/gui-framework.md). The other entries
remain reference evidence. Platform/accessibility acceptance is still open.

The selected entry now has an explicit --project mode using recite-config
discovery, file selection, Script/Source editing and Save. Save applies the field
draft, checks observed disk bytes under a cooperative lock, writes/syncs a
same-directory temporary file, retains the previous bytes as a hidden backup,
then atomically replaces the target. Unix directory synchronization is retried
even when source bytes are unchanged.

Review found the fixture-only document identity was unsuitable for real files.
Workbench/Document now accept actual project-relative identities, including
the temporary document used by Add choice and the input to file preview.
A regression verifies distinct new IDs at equal positions in two files.
Invalid/unsupported documents can open directly in Source.

The focused run passes eight shared authoring and twelve Freya tests. New
coverage includes disk conflict refusal, lock failure/retry, prior-byte backup,
symlink replacement refusal, and the actual GUI open/edit/refused-file-switch/
save/conflict path. No user's project files were edited during verification.

The Freya root renderer is 362 lines and remains the cohesive owner of its
existing hooks and scene composition. New project file operations and controls
were split into project.rs and files.rs. Further extraction of the existing
passage controls can accompany full project-context work; no persistence policy
was added to the renderer.

This starts the retained slice, not production readiness: preview/diagnostics
are still file-local without project schema, close protection and unsaved-draft
recovery remain missing, and platform/accessibility gates remain unaccepted.

## Recoverable project writer · 2026-09-14

The retained Freya writer now checkpoints applied source and field drafts,
restores them on reopen, protects window closing, and retains a recovery copy
before loading an external edit. A per-file OS lock prevents two writers from
replacing the same recovery snapshot. The shared kernel receives all discovered
sources and the configured schema; refresh preserves local drafts and undo.
Preview compilation uses those effective inputs and supports cross-file jumps.

The focused authoring/Freya suite passes 28 tests. New coverage exercises corrupt
and future snapshots, recovery ownership, external edits made while closed,
failed file switches, schema refresh, cross-file preview, keyboard save, and
close-dialog focus cycling/restoration. Clippy with warnings denied, formatting,
test organization, and Git policy pass. These changes are confined to the
isolated GUI workspace and its documentation; the root workspace gate is not
claimed by these checks.

Save mechanics were split into `project/save.rs`; recovery storage and the
native close dialog have separate owners. The root composes the header,
navigation, script field, and optional preview; field and preview rendering have
their own modules, with file lifecycle and persistence kept outside them. The [writing trial](freya-workbench.md#writing-trial) records the
remaining native and human acceptance work. Conditions/effects still require
runtime fixture input integration; the workbench milestone remains open.

## Crash and visual correction · 2026-09-14

The line-details toggle held a state read borrow while writing the same state.
A scene test reproduced the native panic; reading the next value before writing
fixes it. The regression opens and closes details. GUI interaction checks now
also reject clipped or vertically squeezed action labels and exercise opening,
advancing, and closing preview.

The writer follows `docs/gui-visual-language.md`: compact chrome, neutral light
and dark surfaces, serif dialogue, a continuous script with leading sage and
peach rules, and details disclosed on demand. Project controls are behind
Project and preview opens beside the script only when requested. Rendered
captures were inspected in both themes and with the preview open. This is local
visual verification, not human acceptance of the writing experience.

## Compact writer interactions · 2026-09-14

Enabled buttons, navigation, menu actions, and close-dialog actions now use a
pointer cursor. The scene sidebar can be hidden, and script sections can be
folded independently without discarding a field draft. Fold state is scoped to
the document and section. The reading surface can use more of the available
width; passage actions share the speaker row instead of adding a separate row.

Secondary active-passage actions use an anchored Freya menu. Opening focuses its
first action; arrows/Home/End move focus, Escape/Tab dismiss and restore the
trigger, and outside clicks dismiss. Primary draft and save actions remain
visible. Tests check pointer feedback, reclaimed width, retained drafts, menu
position stability, keyboard navigation and dismissal. All 29 focused tests,
Clippy, formatting, test organization and Git policy pass. Light/dark and folded/
menu captures were inspected. Native screen-reader acceptance remains open.

The 360-line root remains the composition/hook owner; menu interaction is isolated
in `passage_menu.rs`. The 367-line scene integration test retains the related
writer interaction scenarios and shared click/capture setup; neither file owns
persistence or language semantics.
