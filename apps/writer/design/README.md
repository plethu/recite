# Writer design gallery and baseline

The gallery renders every Writer surface the way the native window would and records what
assistive technology would see. Design reviews read it, and comparisons against the signed-off
baseline here catch visual drift and accessibility regressions.

```sh
mise exec -- just writer gallery                    # capture, then compare with design/baseline
mise exec -- just writer gallery target/g 'map.,overlay.' 2   # only matching entries, at 2x
mise exec -- just writer gallery-bless target/g     # after maintainer sign-off only
```

## What is captured

Each scene mounts the production `App` view with the window's theme, Writer's bundled fonts and
live workers, then rasterises it with Vello CPU. No compositor or GPU is involved, so the same
source produces the same pixels on any host. Fixtures live in
`crates/xilem/src/tests/gallery/scenes.rs` and the scenes in its `stills` and `transitions`
modules:

- **Stills**: welcome and project creation; Script with a new starter project, an empty beat, a
  scene with no beats, a representative branching scene, an unapplied draft and focus mode; Map
  and Split; source needing repair; Preview; Localise with a catalogue and with none; the
  translations queue; Build, Rename, Declarations, disk changes and source updates; preferences;
  the command, go-to, find and passage pickers; and the unsaved-close decision.
- **Transitions**: welcome to examples, switching workspaces, running a command, closing with
  unsaved work, control states (rest, hover, pressed, keyboard focus), a Tab walk, and a preview
  reply. Each step is a frame; `strips/` joins them into one image per sequence.
- **Specimen**: every production control and type style, with hover, pressed and focus states.

Every scene renders in `light` and `dark`. Layout-sensitive scenes also render in `mono`
(monochrome), `min` (the 900×650 minimum window) and `ui200` (200% interface scale).

Writer has no transition animation today, so a frame per step shows each transition completely.
To review motion once it exists, pass millisecond offsets in `RECITE_WRITER_GALLERY_MOTION` (for
example `16,120,300`): each transition step then also records frames at those times.

## Outputs

| Path            | Contents                                                                  |
| --------------- | ------------------------------------------------------------------------- |
| `index.html`    | Contact sheet grouped by surface, with captions, findings and links       |
| `manifest.json` | Entries, frames, variants, intents, warnings and accessibility findings   |
| `frames/*.webp` | One lossless image per frame, named `<scene>--<variant>--<step>.webp`     |
| `a11y/*.json`   | The accessibility tree in reading order: role, name, value, focus, bounds |
| `strips/*.webp` | Each transition's frames side by side at half size                        |
| `report.md`     | Comparison with the baseline, when one exists                             |
| `diff/*.webp`   | Changed pixels in magenta over a faded current frame                      |

Frame findings are automatic: an interactive node with no accessible name, or an enabled target
smaller than 24×24 logical pixels (WCAG 2.2 §2.5.8). Warnings mean a scene could not reach the
state it asked for, so its frame may not show what its caption says.

The comparison reports, per frame, visual changes for review and accessibility changes in two
tiers. **Regressions** block sign-off: a control that loses keyboard focusability, a target that
shrinks below the minimum, or a new automatic finding. **Changes to justify** need a stated reason:
a control that disappears or is renamed, appears fewer times, or becomes disabled. Set
`RECITE_WRITER_GALLERY_STRICT=1` to fail the run on any regression.

## Limits

- Fonts are hermetic: only the bundled faces are registered. A glyph drawn as a box is missing from
  them; desktops substitute a system font, but treat it as a portability finding.
- Contrast is not measured from pixels. Palette contrast is enforced by the session crate's
  palette tests; text that renders in an unthemed colour still needs a human or reviewer eye.
- Window chrome, native dialogs, IME composition and screen-reader speech are outside the frame.
  `just writer probe` covers native AT-SPI behaviour on a headless Wayland host.

## Baseline

`baseline/` holds the last design the maintainer signed off: `manifest.json`, `frames/` and
`a11y/`, captured at 1x. `decisions.md` records why it looks the way it does. Replace the baseline
only through `just writer gallery-bless` after explicit maintainer sign-off, in the same commit as
the design change and its `decisions.md` entry. The design council workflow that produces those
sign-offs is `.agents/skills/recite-writer-design-council/SKILL.md`.
