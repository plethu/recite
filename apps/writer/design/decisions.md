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
- **Motion.** Nothing animates differently from the platform; reduced motion is unchanged. Motion
  added later must answer a user action and honour the reduced-motion preference.
- **Vocabulary.** Writer says scene, beat and reply; the docs glossary maps those to block and
  choice.

## Log

No council pass has been signed off yet. The first sign-off creates `baseline/`.
