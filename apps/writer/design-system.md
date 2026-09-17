# Writer design system

The writer uses warm reading surfaces, sage selection and focus, and distinct dialogue typography. UI text uses the system sans serif; dialogue uses a serif and
source uses monospace. Light and dark modes share the same structure.

`crates/freya/src/design` owns the implementation:

- `palette` maps semantic colours into Freya's theme, including source syntax.
- `tokens` owns the spacing and type scales, control geometry, and motion curve.
- `Button` owns pointer cursors, primary-button filtering, keyboard activation,
  focus, hover, disabled state, and quiet, secondary, and primary treatments.
  Icon buttons, navigation rows, connections, and checkboxes use this base.
- `Options` presents exclusive choices as a radio group built from `Button`.
  Arrow keys select an option; the dialog tabs through the selected option in each group.
- `Dialog` owns the scrim, bounded surface, title, scrolling body, fixed action
  row, Escape dismissal, and Tab containment. Its caller supplies focus order
  and restores focus on dismissal.
- `Splitter` owns pointer and keyboard resizing within caller-supplied bounds.
- `Reveal` animates a content layer at its final layout size.

Use quiet buttons in toolbars, outlined buttons for secondary actions, and a
filled button for the primary action in a dialog. Selection and keyboard focus
remain distinct from hover. Connection rows show direction, reply, and a
navigation chevron; their longer action name belongs to accessibility metadata.
Scenes form an accordion, with the active scene’s beats indented in source order.
Start and end labels provide orientation without inventing a branch hierarchy.
Scripts use numbered replies and horizontal separation; expanded destinations
use a named preview surface. Destination previews and route editing share a row.
The beat title edits in place (Enter applies, Escape cancels), beside a named
close icon. Compact script insets preserve a stable metadata row so passage
actions never move the prose when focus changes.
Graph cards retain their world-space geometry and editing behaviour.

Motion uses a 160 ms cubic ease-out. Drawer opening establishes its final width
once, then translates and fades its content; closing removes it directly.
Dialogs fade without scaling text. Graph emphasis and prose focus use the same
timing. Reduced motion makes these transitions immediate. During resizing, a guide follows the pointer while text keeps its current
layout; release applies the width once, and Escape cancels. Keyboard resizing
and camera movement apply immediately. Component tests check stable text geometry;
native frame pacing still needs desktop measurement.

The drawer is 180–360 logical pixels wide. The script pane is 320–800 pixels,
with both maxima constrained to retain at least 320 pixels for the map.
Dividers support dragging, Left/Right, and Home/End. Widths survive hiding and
reopening within the window; they are not yet saved across launches. Script
side is saved as `writer.pane_side = "left"` or `"right"` through `recite-config`.
Existing configurations default to the right. The native window minimum is
900 × 650 logical pixels.

Native trackpad pinch remains a backend gap. Freya 0.5.0-rc.4's renderer does
not forward Winit's pinch event and its plugin interface does not expose raw
window events. Winit's 0.31 development line adds Wayland gestures, alongside
changes to window, event-loop, and pointer APIs; this is a backend migration,
not a compatible version substitution. See the
[Winit release notes](https://github.com/rust-windowing/winit/releases/tag/v0.31.0-beta.1).
Acceptance requires native gesture delivery, viewport routing, scale and phase
handling, cancellation, and physical trackpad checks. Ctrl+scroll remains a
separate interaction, not evidence that pinch works.

Localisation uses a full-width bilingual beat manuscript beside the existing
scene tree. Source structure, replies, conditions and destinations come from the
same paged script renderer. The workspace switch sits above writing-specific
Map/Source controls; PO paths and catalogue actions sit beside the language.
Translation draft controls belong to their passage. Review state is stored as
gettext's `fuzzy` flag, and a review checkbox change remains explicitly pending
until saved. The queue has its own routed screen. Its search, filter and page survive a
round trip to the manuscript through Back/Forward. New localisation UI text comes from the shared Fluent resource.

Catalogue setup is disclosed in context: **Start localisation** is the primary
empty-state action, with **Open PO catalogue** as the quieter existing-work path.
Once connected, **Add language** lives behind the catalogue filename. Setup asks
for a language through a searchable dropdown, shows the locale-derived filename
and extraction scope, and returns
to the same beat. No permanent language-administration panel occupies the script.

The mode switch uses verbs: Write / Localise (Localize in US English). Workspace
Back/Forward and Copy link are compact, named controls in the common header.
History covers modes, scenes, passage locations and queue state. Failed navigation
keeps the current location and reports what must be saved or corrected. Settings
and short setup/file operations remain dialogs; browsing the catalogue does not.

## Search pickers and dialog submission

`SearchPicker` owns a stable single field, an anchored overlay that flips above
when necessary, virtualised result rows, pointer dismissal, and keyboard mode.
Callers provide structured `PickerOption` values (title, optional native/context
label, stable value), cached search results, and the selection action. They own
validation and persistence. Opening a picker does not resize the dialog. Empty
searches show guidance; matching results scroll rather than stopping at eight.
Language search prioritises exact names and tags, and keeps its normalised ISO
index cached. Rows omit empty or repeated native names.

Arrow keys navigate in either keymap. With Vim enabled, the search opens in
INSERT: j/k are text. Escape enters NORMAL, j/k navigate, and i or / resumes
INSERT. Another Escape dismisses the picker; the following Escape dismisses the
dialog. Enter chooses a result without submitting the surrounding form. Tab
leaves the picker and follows the dialog's focus order.

Every `Dialog` declares a `DialogAction`. Its primary button and Ctrl+Enter
(Cmd+Enter on macOS) invoke the same enabled action. The button displays the
platform modifier and return symbol; disabled actions cannot be submitted or
entered through the dialog's tab order. Plain Enter retains control-specific
behaviour. Text inputs and the project settings editor pass the submit shortcut
to the dialog without inserting a newline.

| Existing surface | Reuse decision |
| --- | --- |
| Start localisation / Add language | Shared picker, compact scope and derived-path text, Create catalogue action |
| PO catalogue dialog | Open action; disabled for an empty path or unresolved comparison; comparison decisions remain explicit |
| Personal settings | Preferences apply immediately; the dialog action is Close settings |
| Project settings | Apply project changes is the dialog action; Close remains separate |
| Exit confirmation | Close Recite or Save and close is the primary action; recovery remains a separate choice |
| Passage actions | Shared j/k and arrow navigation policy; remains a short action menu |
| Two-choice settings | Shared Vim list navigation; remains a radio group |
| Scene/project search and translation queue | Retain their workspace layouts and context; no dropdown conversion |

Native interaction tests cover dialog geometry, picker pointer selection, Vim
modes, invalid/no selection, and submit routing from inputs and buttons. The
keyboard glyph chooses the build target; macOS device/VoiceOver verification
remains part of the packaging/platform proofs.

## GUI quality pass

Destination changes use `SearchPicker` beside the reply or continuation, including
passage details. Results preserve source order and include an explicit End
conversation option. Selection remains an ordinary undoable document edit.

`SearchField` is the shared input for project search, the scene sidebar, and the
translation queue. It owns clearing, input focus restoration, Arrow/Enter result
navigation, and Vim INSERT/NORMAL boundaries. Clearing returns to text entry;
Escape clears in ordinary mode, while Vim Escape first leaves INSERT. Callers
retain result rendering, paging and context. Project search discloses its loaded
count and can extend beyond the initial 100 matches. Scene search expands the
active scene while filtering so collapsed beats remain discoverable.

`TranslationStatus` owns both passage labels and queue attention membership. An
unsaved review remains pending and visible in the attention queue until saved.

`PathField` provides manual entry, Enter submission and asynchronous native
Browse for project folders and PO files. Browsing is parented to the window;
cancellation preserves the typed path and selection never implicitly opens it.
The existing draft guards and file validation still own opening.

`Feedback` distinguishes information from errors and offers dismissal and an
optional recovery action. Operation results are independent of notice visibility.
Notices render once in their active context; dialog notices contribute their
controls to the focus order. Diagnostic rows retain code/file/line/column and can
place the source cursor at a diagnostic in the current document. The source view
keeps a bounded diagnostics list beside its editing surface. Passage menu items
use the shared Button interaction and MenuItem semantics.
