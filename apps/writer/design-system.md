# Writer design system

Writer uses warm paper, ink and plum, with separate roles for canvas, manuscript,
inset fields and floating panels. Selection, keyboard focus and hover must remain
visually distinct. Light, dark and monochrome appearances share the same layout.
The [Recite identity](../../assets/identity/README.md) supplies the wordmark and
packaged app icon; the runtime wordmark uses the current text colour.

Inspect the production controls without opening a project:

```sh
mise exec -- just writer --design-system
```

The specimen's appearance and reduced-motion settings affect only that window.
It exercises real buttons, segments, search, destination pickers and graph cards.

## Ownership

`crates/freya/src/design` owns reusable presentation and interaction:

- `palette` is the source of truth for colours. Its theme adapter supplies the
  same colours to Freya controls and source syntax. Contrast tests cover text,
  placeholders, boundaries, focus and interaction states in all appearances.
- `tokens` owns spacing, type scales, control geometry and font fallbacks.
  UI uses Inter/Geist with platform sans fallbacks; dialogue uses
  Literata/Source Serif 4/Noto Serif with Georgia/serif fallbacks. Fonts are
  resolved locally. `ProseTypography` keeps editing, localisation, preview
  and map excerpts consistent.
- `Button` owns pointer filtering, keyboard activation, focus, hover, press and
  disabled feedback. Icon buttons, navigation rows and checkboxes reuse it.
  Use quiet actions in toolbars and a filled primary action in dialogs.
- `Segments` presents exclusive choices as a radio group; arrows change the
  selected option and Tab moves between groups. `Options` adds a setting label.
- `Dialog` owns the scrim, bounded scrolling body, fixed actions, Escape and
  focus containment. Callers provide the order and restore the invoker's focus.
- `SearchPicker` owns its anchored, bounded, virtualised popup and keyboard
  selection. Callers own values, cached results, validation and persistence.
  Opening or filtering a picker must not resize its containing dialog.
- `SearchField` owns clear/focus behavior and result navigation. `use_list_reveal`
  scrolls when the query or keyboard selection changes, not when unrelated
  layout updates occur. Manual scroll position must survive idle rerenders.
- `SubmitAction` provides one action for the button and platform submit chord.
  Disabled actions must stay disabled through either route. Enter in prose
  inserts a newline; Ctrl+Enter or Command+Enter submits a containing dialog.
- `PathField` owns typed paths and asynchronous Browse. Cancel preserves text;
  selection never implicitly opens a project or file.
- `Splitter` owns bounded pointer and keyboard resizing. Pointer dragging
  previews a guide; release reflows once, Escape cancels, keyboard changes
  apply immediately.
- `BeatCard` owns card presentation. Scene layout, routes, selection and camera
  state stay with the map. `material` owns restrained card/search shading;
  other reading panels, buttons and selection plates use flat fills.

Feature modules own validation and persistence. A reusable control must not
acquire project state or duplicate compiler, schema, catalogue or runtime rules.

## Type, layout and motion

The minimum window is 900 × 650 logical pixels. Controls must also work at
200% UI scale. Standard actions and fields have a 32-pixel minimum height;
24-pixel clear actions and segment plates sit inside that frame.
The drawer spans 180–360 pixels and the script pane 320–800, constrained by the
remaining map space. Dividers support Left/Right and Home/End. Saved dimensions
and pane side belong to personal `writer.presentation`/`writer.pane_side` settings.

Script and localisation use a reading column. Metadata and passage actions stay
stable while editing; long beats page entries without flattening condition groups.
Map geometry stays independent of excerpt length. A graph action must have an
accessible textual route, and technical IDs remain available through details.

Pointer and keyboard presses share immediate colour feedback. Button transitions
use 100 ms ease-out; press depth settles in 60 ms and returns in 160 ms. Segments
use 200 ms interpolation without moving labels or hit targets. Pickers enter
with a four-pixel translation/fade over 160 ms and dismiss immediately. Dialogs
fade without scaling text. Interrupted animation starts from its visible state.
Reduced motion makes these changes immediate.

## Verification

`mise exec -- just check-writer` includes colour ownership linting, contrast,
component and interaction tests. Use `RECITE_WRITER_CAPTURE_DIR` with the
`design_system` or `scene` integration tests to capture actual rendered controls.
Review light, dark and monochrome, long labels, disabled/pressed/focused states,
and narrow windows. Native frame pacing and assistive-technology evidence remain
separate; see [acceptance](acceptance.md).
