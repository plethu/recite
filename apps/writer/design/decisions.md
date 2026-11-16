# Writer design decisions

Design councils and regression reviewers read this before proposing anything. It holds what the
maintainer has settled, so a new pass builds on it instead of reopening it. Newest entries first.
Each signed-off pass adds one entry with the baseline commit it produced.

## Standing decisions

These predate the gallery and come from #256 and `assets/identity/README.md`.

- **Audience.** Narrative designers and writers who author branching dialogue for games, often in
  long sessions alongside a source editor and an engine. Writer is a professional tool: reading
  and writing comfort comes before novelty.
- **Identity.** Restrained risography: two inks on paper with print typography. Plum, ink, paper,
  riso teal, text-safe teal, mint and overprint navy are the inks; values live in the identity
  README and `recite-writer-session`'s palette.
- **Text colour.** Riso teal is a plate ink at 3.8:1 on paper and is never text there. Text is
  plum, ink or text-safe teal. Mint may be text on the navy status bar and in dark mode. Every new
  colour gets a contrast row and a palette test.
- **Texture.** Grain never sits behind prose, code or the reading surface. Misregistration and the
  print offset appear only in identity assets, hero and empty states, never on controls.
- **Type.** Fraunces for display and identity, Literata for the manuscript, IBM Plex Sans and Mono
  for interface and source. Other scripts use named system fonts.
- **Structure.** A paper sidebar, a tab row that is always present, a one-row toolbar and an ink
  status bar. Script reads as a manuscript: speaker labels in the margin, destinations inline, an
  end-mark and folio. Map has a dot grid, teal return edges and offset cards.
- **Tokens.** `presentation/tokens.rs` owns the spacing scale (4, 8, 12, 16, 24, 32, 48), the
  interface type roles, which scale with the UI, and the sheet roles, which follow the reading size
  with floors. Views compose tokens and helpers from `presentation/` rather than hard-coding
  colours, sizes or gaps.
- **Controls.** One state table (`presentation/theme/states.rs`) styles every control class; wells
  share one look. Every one-of choice is a segment on one tonal track; only Write | Localise are
  tabs.
- **Slips and backdrop.** Menus, pickers and modal panels are slips: floating paper with a 1px
  ink edge and a hard stock offset. Behind a modal slip the backdrop is a static overprint dim
  under a paper-toned print texture; nothing blurs or animates.
- **Modal surfaces.** Pickers open 640px wide 72px from the top over the backdrop, one 32px line
  per result in a virtual list with no paging. The close decision is a slip over the dimmed
  document. Settings is a 360px side sheet over the live workspace, whose manuscript is the
  preview. Each takes focus as it opens, returns it as it closes, and closes on Esc.
- **Tool sheets.** Tool and Localise screens share Script's toolbar. Each sheet opens with its
  title in Display, its own actions and a named Return to writing icon; Open in editor, Save all
  and Close live in the Workspace menu.
- **Galley.** Script is one continuous scene on a sheet grid (`presentation/grid.rs`): pad,
  margin column, measure capped at 68 characters, destination column. Every beat is set at the
  same metrics; only the selected one mounts fields, so choosing a beat never reflows the page. A
  sticky running head and foot carry the place. Below a 480px measure the paper goes full-bleed.
- **Margin marks.** One vocabulary in the galley margin, the scene list and on map cards: a solid
  teal bar for the current beat, a plum halftone bar for unsaved work, an error wedge for a
  problem. They differ in shape as well as ink; only the galley's wedge takes focus.
- **In place, not forms.** A change happens on the object it changes. Rename beat (F2, the
  command or the heading's pen) turns the selected beat's heading into a field; F2 or a
  double-click on a sidebar beat row or a map card does the same to that row or card (in Map the
  command renames on the selected card, and a card's double-click renames rather than opens:
  Enter and Open script open it). Enter reviews the project rename and a slip under the field says
  what it changes ("Renames a to b in 3 references across 2 files") with Rename, Review all
  changes and Cancel, stacked in the narrow sidebar; Esc cancels and focus returns to where it
  was, or to the row or card under its new name when the field took its place. Rename stages one Undo step and the status line offers Undo; only a
  refused rename opens the full page. Insertion affordances ("+ Line", "+ Reply", "+ Beat") sit
  after the beat, never between rows, so choosing a passage never moves another; Line and Reply
  insert after the passage being written when it is of their kind. The passage being written
  offers its choice on the label itself: its margin speaker label and its destination are the
  controls (Label style: borderless, a focus ring inside the edge, the words where the resting
  label's are, at least 24px tall), `expanded` while their combobox slip hangs from them. A slip
  opens on the current value, says "Find speaker" or "Find destination" before anything is
  typed, and drops the kind column when every result is of one kind. A resting reply's
  destination stays the toggle that shows the destination's opening.
- **Filter as you type.** Search fields filter on every keystroke and have no Search button; a
  list states its count under the field ("6 declarations", "1 of 6 declarations") and scrolls as
  one virtual list of 32px rows with a plate selection. A declaration's prose wraps in a named,
  non-focusable detail area that scrolls beside the list rather than paging; a single unbroken
  token ends in an ellipsis and keeps its full text as its accessible name. Paging survives only
  for reports too long to mount at once.
- **Sheet heads and banners.** A sheet head holds the sheet's own actions, one of them Primary,
  and stacks them on a second row below 640px. Changes from outside Writer arrive as a banner at
  the top of the affected sheet (Changed on disk · Compare · Keep mine · Use disk version), which
  settles the open document in one checkout; the merge page opens only when needed.
- **Commands with results.** Build is a command: its progress is the status sentence with Cancel
  build beside it, and its result waits on a slip above the status line; the build page opens
  only for a failure's report or to choose outputs. The status sentence is one line ending in an
  ellipsis; assistive technology reads it whole.
- **Master and detail.** A list of records sits in a 320px column beside the chosen record from
  720px and stacks above it below. A record's fields read as a definition list: names in Plex
  Mono ink, types or values in muted Sans.
- **Toggles.** A state that is on or off is a named toggle holding the pressed sink (Reviewed,
  Pin reference); a disclosure says expanded or collapsed (Translation details). A choice that
  cannot apply is hidden rather than shown disabled when hiding changes neither Tab order nor
  discoverability: while Preview uses the source language only, the language search is not
  mounted.
- **Motion.** Nothing animates differently from the platform; reduced motion is unchanged. Motion
  added later must answer a user action and honour the reduced-motion preference.
- **Vocabulary.** Writer says scene, beat and reply; the docs glossary maps those to block and
  choice.

## Log

No council pass has been signed off yet. The first sign-off creates `baseline/`.
