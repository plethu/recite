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
- **UI scale zooms the sheet.** The Reading size setting is the sheet's size at 100%. The UI scale
  zooms the whole sheet as a browser zooms a page: lines, replies, numbers, labels, destinations,
  the scene title (Display, 1.6R), beat headings (Beat, 1.1R) and the folio, with their 11 and
  13px floors, and the grid's measure and columns, which are set in ems. Where the pane is
  narrower than the zoomed measure the text reflows. Controls on the sheet zoom with it and never
  fall below the interface's own size, so no text or target is smaller at 200% than at 100%.
- **One display line per sheet.** The scene title is the sheet's only Fraunces display line. Beat
  headings are Literata semibold at 1.1R (22px at R20), a face change and a 1.45 size ratio below
  the title, so an empty beat named like its scene no longer reads as a second title, and the
  same metrics on every beat, so selecting one never reflows. This is the one deliberate shrink
  against the baseline (headings were 32px). A beat that ends the conversation ends with the
  end mark flush left on the text column and a faint dotted rule running right; an empty beat
  keeps it, because it still says the conversation ends there. The insertion affordances' plus
  hangs in the margin column with its end on the margin labels' right edge.
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
- **Printer's marks, not margin bars.** One family from a press sheet, painted around the work
  and never on its words, the same in the galley, the lists and on map cards
  (`presentation/marks.rs`). The current beat has crop marks: at each corner a pair of 2×8 ticks
  4px off the corner, pointing outward, in the sheet pad and the space between beats (doubled at
  200%, capped 4px inside the pad; 6px ticks 2px off on a full-bleed page); a map card has them
  outside its stock offset, and in the scene list the current beat is the selected row. The
  current line has no mark: its field shows it. Unsaved work is an 8px plum halftone dot, in the
  sheet pad beside the changed line or heading, in a trailing slot every scene row keeps, and in a
  card's top-trailing corner. A selected list row keeps its plate and has a 2px ink underscore
  under its words. Banners are errata slips (floating paper, a closed 1px boundary edge, a 3px
  stock offset and a plum caps lead-in). A problem keeps its error wedge. In monochrome the ticks
  and the dot are both ink and differ by shape alone. Nothing takes layout room, input or a name;
  only the galley's wedge takes focus. These replace the leading-edge bar, which marked the
  current beat, unsaved work, selected rows and banners alike: the maintainer found it out of
  keeping with the print identity and a lazy substitute for design, and it hid unsaved work on a
  line under the beat's own bar. The toolkit layer's bar runs along the bottom only, so the
  leading bar cannot return.
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

### 2026-11-17 · v1 polish pass (council rounds 1–9)

- **Request.** A final GUI cleanup and polish for v1:
  - better use of space and a stronger type hierarchy in Script;
  - inline mutations instead of forms;
  - modern controls;
  - one spacing scale that is neither cramped nor loose;
  - a dimmed backdrop with print texture.

  After round 8, the maintainer rejected the leading-edge bar used for the current beat,
  unsaved work, selected rows and banners. They called it out of style with the product and a
  lazy substitute for better design.
- **Direction.**
  - Script is one continuous galley. The 32px Fraunces scene title is the only display heading;
    beats are 22px Literata semibold.
  - Only the line being written is a field. Read-only lines keep identical metrics, so nothing
    moves on selection, and ids sit in the margin.
  - Rename, insertion, speaker and destination choice happen in place. Build, preview
    configuration and disk changes settle in slips and banners rather than pages.
  - Settings is a modal side sheet that the toolbar's Workspace control toggles.
  - Interface scale zooms the manuscript.
  - The leading-edge bar is gone. The replacement is one family of printer's marks:
    - crop-mark ticks outside the current beat and the selected map card;
    - an 8px plum dot for unsaved work, in the sheet pad and in a slot reserved on every scene
      row;
    - an ink underscore under a selected row's words;
    - errata-slip banners with a plum caps lead-in.
  - `material::Bar` draws only along the bottom.
- **Patterns relied on.**
  - Stable placement, direct manipulation, and progressive disclosure.
  - Undo over confirmation, and visible system status in the status line.
  - Recognition over recall in pickers.
  - Print conventions: galley proofs, crop marks, editor's margin marks, errata slips, tint plates.
- **Rejected.**
  - Scaling only headings and labels at 200%.
  - A hatch backdrop.
  - Reserving an id line under every row.
  - A disclosure for raw effect syntax.
  - 32px Accept / Keep chips, which stop centring their labels.
  - 20px beat headings.
  - The band banner, and the slip without its stock offset.
  - Keeping "+ Line" words on the text edge with a wider gap.
- **Dissent recorded.** One member preferred a 10px outlined square for unsaved work and no ": END
  :" in empty beats. Another preferred the banner without its offset. The maintainer saw each
  alternative rendered side by side and chose the built option.
- **Maintainer feedback.**
  - Round 4: poor use of space, forms, dated controls, and spacing both cramped and loose.
  - Round 9: replace the left-hand border everywhere.
  - Sign-off: the recommended option on all fifteen decisions, including keeping
    `PoDocument::refreshed_keeping` and merging as one pull request.
- **Accessibility evidence.**
  - Gallery comparisons against the pre-pass capture show no accessibility regressions at 1x or
    2x.
  - New palette contrast rows cover the crop ticks, unsaved dot, underscore, banner edge and
    lead-in in light, dark and mono.
  - Marks tests check geometry, no overlap, no hit-testing, identical row bounds when selected,
    and that mono luminance tells current, unsaved and plain apart.
  - A Tab walk was run. Mono, minimum-window and 200% frames were checked.
  - The beat heading is the only text made smaller on purpose, from 32px to 22px; it stays above
    the 20px reading size.
  - The AT-SPI probes in `tests/writer-native` were updated but not run here, because the
    container has no Wayland host.
