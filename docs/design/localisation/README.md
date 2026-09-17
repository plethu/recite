# Localisation interaction study · 03

Open `index.html` directly in a browser. No build, dependencies, server or network
requests. Edit the HTML, CSS tokens or sample JavaScript and reload. All edits
are temporary; reloading resets the sample. Freya is unchanged.

This is a disposable wireframe to settle narrative context and task transitions.
French text and translator notes are illustrative, unreviewed design fixtures.

## Audience and direction

Writers need to revise passages while reading the surrounding conversation.
Translators need speaker, intention, incoming prompt and destinations. People
using external PO editors should retain that choice and still benefit from the
story context. The bilingual beat manuscript keeps these needs together.

One scene/beat navigator stays visible. The main area shows the complete beat:
speech followed by replies and their destinations. Source and French align by
passage. The incoming prompt is above the beat, with preceding speech available
through disclosure. A separate translation queue supports catalogue-wide work;
it lists actual passage text and opens the selected passage within its full beat.

## Try these paths

1. Start in **Initial draft**: Station History contains Mara's speech and both
   replies. Edit in place. No translation administration appears in the script.
2. Choose **Localised project**, then **Localisation**. Source and translation
   appear together. Fields grow with content up to a scrolling maximum.
3. Focus Reply 1, then switch to **Write**. The complete beat remains visible and
   the same reply has focus. Return to Localisation: translation drafts survive.
4. Enter a translation. **Save** appears within that passage. **Reviewed** sets
   review intent; the state explicitly says **Review pending save** until saved.
   Further text edits clear review. Ctrl/Cmd+S saves the focused translation.
5. Open **Translation queue**, filter or search, and select a passage. The queue
   closes and that passage opens in context. Saving does not advance the queue
   or remove the current passage from the manuscript.
6. Open the PO path beside French. It identifies the shared working document.
   Native external-editor/reveal actions are described, not fake launch controls.
7. **Simulate external edit** from the prototype strip. Compare versions, dismiss
   without resolving, or retain your draft/file version. Draft retention still
   requires a subsequent save. No real file is touched.
8. Compare light/dark, keyboard navigation and narrower windows. Selection uses
   an inset underline and fill; hover does not impersonate selection, and focus
   has its own outer outline.

The top strip belongs to the study, not the proposed app. Six sample passages
span three beats. Other scenes are outside the sample. Source drafts appear in
the bilingual view but do not regenerate PO fixtures: affected passages explicitly
say catalogue refresh is pending. Full refresh/merge semantics remain out of scope.
The external-change fixture affects Station History's opening speech only.

## What changed after critique

Revision 01 used two repeated navigation columns and a save/review/next button
row. Revision 02 attached actions to their objects but still treated each
catalogue record as the entire script. Revision 03 replaces that model with the
full beat in both workspaces. Replies no longer acquire invented replies of their
own. Destination controls refer to beats, not synthetic dialogue.

Language and catalogue identity now sit together. Short replies receive short
fields. Source revision details are adjacent to the affected translation. Review
and persistence state are distinct. Actual passage text replaces anonymous
“Reply 1” entries in the optional queue. No reflow animation is used.

## Decisions still to test

Does a bilingual manuscript provide enough context without a separate inspector?
Does opening the queue as a temporary surface suit batch translation, or should
it become a separate full workspace? How much review metadata should remain
visible on inactive passages? These are interaction decisions, not native
implementation commitments.

Follow-up studies: plurals and variants, multiple incoming routes, catalogue
creation, source-update review, fallback preview, undo/redo and large queues.
The small fixture proves neither virtualisation nor catalogue performance.
Production must reuse shared kernel PO/validation authority and benchmark loading,
editing, source refresh and external reload independently.

## Design guidance

Used local skills: `humanist-software`, `prototyping`, `web-frontend-quality`,
`css-quality`, `accessibility`, and the existing
[writer design system](../../../apps/writer/design-system.md).

- [Refactoring UI](https://refactoringui.com/): public hierarchy, spacing and
  fewer-borders guidance, not a claim of access to the paid book.
- [NN/G: Progressive disclosure](https://www.nngroup.com/articles/progressive-disclosure/):
  put routine work first and expose secondary detail in context.
- [NN/G: Usability heuristics](https://www.nngroup.com/articles/ten-usability-heuristics/):
  recognise context, preserve control, distinguish state and support recovery.

Your preferences take precedence over cookbook tactics: compact writing space,
scene ownership, meaningful context, consistent pressable surfaces, exclusive
choices, no decorative left-border containers, no repeated action rows, and no
animated text reflow. Freya remains the native frontend.

## Verification

Revision 03 checked in local Chromium: full-beat writing and translation,
passage focus across workspaces, retained translation drafts, pending/saved review,
queue search and navigation, and external-conflict draft retention. No horizontal
page overflow at 1440, 1000, 640 and 375 pixels. Visually inspected wide light and
1000-pixel dark layouts. JavaScript syntax and whitespace checks passed.
Native performance and assistive-technology acceptance remain untested.
