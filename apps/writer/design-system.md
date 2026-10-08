# Writer design system

Writer uses warm paper, ink and plum, with separate roles for canvas, manuscript, inset fields and
floating panels. Selection, keyboard focus and hover must remain visually distinct. Light, dark and
monochrome appearances share the same layout. The [Recite identity](../../assets/identity/README.md)
supplies the wordmark and packaged app icon; the runtime wordmark uses the current text colour.

Inspect the production controls without opening a project:

```sh
mise exec -- just writer run --design-system
```

The specimen's appearance and reduced-motion settings affect only that window. It exercises real
buttons, segments, search, destination pickers and graph cards.

## Ownership

[`crates/freya/src/design`](crates/freya/src/design) owns palette, typography, geometry and shared
controls. Use its tokens rather than local colour, spacing or timing constants. Dialogue typography
is shared across writing, localisation, preview and excerpts; fonts resolve locally with fallbacks.

Use quiet toolbar actions and a filled primary dialog action. Exclusive choices use `Segments`,
searchable bounded lists use `SearchPicker`, and modal work uses `Dialog`. Reuse button and submit
behavior so keyboard and pointer activation agree. Enter in prose inserts a newline; the platform
submit chord invokes the containing action. Pickers must not resize their dialog or lose manual
scroll position during idle updates. Dialog callers supply meaningful focus order and restore the
invoker's focus.

Feature modules own values, validation and persistence. Shared controls must not acquire project
state or duplicate compiler, schema, catalogue or runtime rules. Map layout, routes and camera state
remain separate from card presentation.

## Type, layout and motion

The token module owns window, control and pane dimensions and animation timing. Review the specimen
at the minimum window size and 200% scale. Resizing must remain available by keyboard; pointer
resizing previews a guide and can be cancelled before reflow.

Script and localisation use a reading column. Metadata and passage actions stay stable while
editing; paging preserves condition groups. Map geometry stays independent of excerpt length. Graph
actions need an accessible textual route, with technical IDs available through details.

Pointer and keyboard actions share feedback. Animation must not move labels or hit targets, scale
reading text or jump when interrupted. Reduced motion makes transitions immediate without removing
state information. Reading panels, controls and selection plates use flat fills; restrained depth
belongs to cards and search surfaces.

## Verification

`mise exec -- just writer check` includes colour ownership linting, contrast, component and
interaction tests. Use `RECITE_WRITER_CAPTURE_DIR` with the `design_system` or `scene` integration
tests to capture actual rendered controls. Review light, dark and monochrome, long labels,
disabled/pressed/focused states, and narrow windows. Native frame pacing and assistive-technology
evidence remain separate; see [acceptance](acceptance.md).
