# Writer design system

The writer uses warm reading surfaces, sage selection and focus, and distinct
dialogue typography. Canvas, manuscript, inset fields and floating panels have
separate surface roles. Hover is neutral; selection retains its sage fill and
weight; focus has an independent outline. Light and dark modes share the same
structure, with colours chosen separately for each.

Open the native component specimen with:

```sh
mise exec -- just writer --design-system
```

It uses the production buttons, segmented controls, search field, destination
picker and graph-card renderer. Appearance and reduced-motion controls affect
only that window; the specimen does not open projects or save preferences.
The same treatments are applied to the ordinary writer, not confined to the
specimen. Inspect both appearances and exercise keyboard focus as well as hover.

`crates/freya/src/design` owns the implementation:

- `palette` owns semantic colours; its `theme` adapter configures Freya's inputs,
  menus, tooltips, scrollbars and source editor from those same values.
- `material` owns the restrained graph-card gradient and inset search field.
  Buttons, selection plates and reading panels use flat fills.
  `motion` owns interruptible interpolation and popup entry.
- `tokens` owns spacing, type scales and fallback families, control geometry,
  and the motion curve. UI prefers Inter/Geist with platform sans fallbacks;
  dialogue prefers Literata/Source Serif 4/Noto Serif with Georgia/serif fallbacks.
  Fonts are resolved locally, not downloaded or bundled. `ProseTypography`
  applies the same family chain to editing, localisation, preview and map excerpts.
- `Button` owns pointer cursors, primary-button filtering, keyboard activation,
  focus, hover, disabled state, and quiet, secondary, and primary treatments.
  Icon buttons, navigation rows, connections, and checkboxes use this base.
- `Segments` presents exclusive choices as a radio group built from `Button`.
  `Options` adds the setting label; workspace and view switches share the group.
  Arrow keys select an option; the dialog tabs through the selected option in each group.
- `Dialog` owns the scrim, bounded surface, title, scrolling body, fixed action
  row, Escape dismissal, and Tab containment. Its caller supplies focus order
  and restores focus on dismissal.
- `Splitter` owns pointer and keyboard resizing within caller-supplied bounds.
- `Reveal` animates a content layer at its final layout size.
- `BeatCard` owns graph-card presentation, also used in the native specimen.
  Scene interaction, selection, layout and camera state stay with the map.
  Selection uses a one-pixel accent border and a restrained surface tint.
  Excerpts use the shared 15-pixel scale and 1.28 line height at full zoom;
  speaker metadata sits with its excerpt. Writing text retains its 20-pixel scale.

Use quiet buttons in toolbars, subtly bordered inset buttons for secondary actions, and a
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

Button surface feedback uses a 100 ms cubic ease-out, retargeted from the current
frame. Press colour responds immediately; depth settles inward over 60 ms and
returns over 160 ms. Pointer and keyboard activation share this feedback.
Segmented selections use a 200 ms quartic ease-out, retargeting from the visible position
when interrupted. Labels and hit targets remain fixed. Pointer focus does not draw a keyboard
focus ring. Standard actions and fields share a 32-pixel minimum height; inset
clear actions and segment plates use 24 pixels inside that frame. Quiet hover
fades preserve the destination colour at zero opacity, avoiding a black flash.
Pickers enter with a
four-pixel translation and fade over 160 ms; dismissal remains immediate.
Picker footers appear only for empty results or Vim-mode guidance.
Reduced motion makes these changes immediate. Other motion uses a 160 ms cubic
ease-out. Drawer opening establishes its final width
once, then translates and fades its content; closing removes it directly.
Dialogs fade without scaling text. Graph emphasis and prose focus use the same
timing. Reduced motion makes these transitions immediate. During resizing, a guide follows the pointer while text keeps its current
layout; release applies the width once, and Escape cancels. Keyboard resizing
and camera movement apply immediately. Component tests check stable text geometry;
native frame pacing still needs desktop measurement.

The drawer is 180–360 logical pixels wide. The script pane is 320–800 pixels,
with both maxima constrained to retain at least 320 pixels for the map.
Dividers support dragging, Left/Right, and Home/End. Widths survive hiding, reopening and relaunching through `writer.presentation`. Script
side is saved as `writer.pane_side = "left"` or `"right"` through `recite-config`.
Existing configurations default to the right. The native window minimum is
900 × 650 logical pixels.

Native trackpad pinch remains a backend gap. Freya 0.5.0-rc.7's renderer does
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
Script/Map/Source controls; PO paths and catalogue actions sit beside the language.
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
detail, optional visible annotation, opaque value), cached search results, and the selection action. They own
validation and persistence. Opening a picker does not resize the dialog. Empty
searches show guidance; matching results scroll rather than stopping at eight.
The floating result surface is bounded to 400 logical pixels and the window.
Selection values are not automatically rendered: language tags use the visible
annotation, while completion indices remain internal. Source completion's trigger
is now bounded too; caret-anchored completion remains a separate editor change.
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

Escape dismisses the current interaction; it never requests application exit.
Native close and the platform quit shortcut retain the existing close protections.

## GUI quality pass

Destination changes use `SearchPicker` beside the reply or continuation, including
passage details. Results preserve source order and include an explicit End
conversation option. Selection remains an ordinary undoable document edit.

`SearchField` is the shared input for project search, the scene sidebar, and the
translation queue. It owns clearing, input focus restoration, Arrow/Enter result
navigation, and Vim INSERT/NORMAL boundaries. Clearing returns to text entry;
Escape clears in ordinary mode, while Vim Escape first leaves INSERT. Callers
retain result rendering, paging and context. Empty searches omit the clear button;
entered queries use the shared Button with its pointer and keyboard behaviour.
Placeholder text has its own contrast-checked colour, and the icon/input spacing
leaves room for the shorter scene-search prompt. `use_list_reveal` owns selection
reveal for command, project and scene search and source completion. It scrolls
when the query or keyboard selection changes; reactive layout bounds alone must
not override manual scrolling. Project search discloses its loaded
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

## Component finish checkpoint · 18 September 2026

The first native pass covers shared controls and their existing app callers.
Render captures were inspected at 1200×900 and 900×650 in light and dark, including
an open picker, graph cards and Relay Hub editing. Set
`RECITE_WRITER_CAPTURE_DIR` when running `tests/design_system.rs` to retain the
specimen captures. The ordinary example and map tests also accept that variable.
The native specimen was launched on Linux after the headless checks.

The writer workspace suite passed 189 tests, with two explicit timing workloads
ignored by its normal run. Workspace Clippy with all targets/features and warnings
denied, formatting, both 10,000-passage heap checks, test organisation, Git policy
and whitespace checks passed. The subprocess tests required execution outside the
sandbox because `wait-timeout`'s signal notification write was denied inside it.
Clippy was rerun after consolidating an identical button-style branch.

Palette tests check 4.5:1 for body/supporting text on reading, inset, floating,
selected, hovered and pressed surfaces, and 3:1 for input boundaries and focus.
These are opaque token pairings; they do not certify every rendered interaction
or replace native high-contrast and assistive-technology testing. Frame pacing,
physical IME and long-session comfort still require desktop trials.

The maintainability pass split native theme adaptation from palette ownership,
shared graph-card presentation, and reused the same segmented control in settings
and workspace switches. Button and picker modules remain above the 250-line
review trigger: each retains its component's interaction and focus contract;
surface animation is separate. The existing workbench entrypoint, scene map,
closing, localisation and rules modules received only composition, style or
bounded Escape changes; their feature ownership was retained. Their larger
restructuring belongs with the proposed workspace changes, not this visual pass.

The earlier material and motion pass introduced a moving selection plate; the
workspace refinement retains its motion with flat fills. Behaviour tests cover
intermediate selection positions, reversal, plate alignment, fixed hit targets,
reduced motion, and pointer/keyboard activation with one activation per gesture.
Contrast checks include gradient endpoints. The specimen capture test also saves
`selection-00.png` through `selection-17.png` when capture output is enabled.
These checks establish rendered state transitions, not desktop frame pacing.
The follow-up passed `just check-writer`: 191 tests, two ignored timing workloads,
Clippy, formatting and both heap budgets. Test organisation, Git policy and
whitespace checks also passed. The updated native specimen was reopened on Linux.

## Writing workspace

Script is the first-use view, showing the selected beat in a centred reading
column. Explicit Map and Source preferences remain valid. Map cards select on
single click; Open script, Enter or a double click opens the selected beat.
Script + Map is optional, and narrow layouts collapse it before reducing text.
Focus writing hides navigation and restores the preceding layout on exit.

Reading size (14–32 px), Source size (12–28 px), and UI scale (100–200%) are
independent validated user preferences. Defaults are 20 px, 15 px and 100%.
Commands and Go to scene or beat share actions with the visible controls.
Their shortcuts are Ctrl/Cmd+Shift+P and Ctrl/Cmd+P.

Freya is pinned to 0.5.0-rc.7. The vendored horizontal-scroll patch is removed.
The host still reveals shaped caret bounds after edits and diagnostic jumps.
Line numbers use an independent gutter: rc.7's shared gutter scroll controller
otherwise clamps the content's horizontal offset. Regression tests cover
long lines, caret movement, delayed layout and manual scrolling.

## Workspace refinement

Shared buttons and segmented choices use flat fills, with borders for secondary
actions and an accent fill for primary actions. Workspace navigation occupies two
rows; less frequent actions remain reachable through the Workspace menu at large
text sizes. Quiet actions stay beside the content they affect. Script uses a
subtle field boundary without changing geometry on focus.
