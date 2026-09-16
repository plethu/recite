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
