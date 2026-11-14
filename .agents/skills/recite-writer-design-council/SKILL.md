---
name: recite-writer-design-council
description: Use for Writer GUI visual and UX improvement passes, design reviews of Writer screens, and checking a Writer UI change for visual or accessibility regressions against the signed-off baseline. Runs the rendered gallery, convenes a council of clean-context models, iterates to a presentable design, takes the maintainer's sign-off, commits the baseline, and closes with a regression review.
---

# Writer Design Council

Writer should feel like a well-made printed object that is pleasant to work in for hours: tactile,
calm and legible, in the restrained risography identity. This skill turns that goal into a
repeatable loop: render everything, let independent reviewers argue, change the code, and stop only
when the maintainer signs off. Read [design/README.md](../../../apps/writer/design/README.md) for
the gallery and [design/decisions.md](../../../apps/writer/design/decisions.md) for what is settled.
Load `recite-rust-quality` before changing Writer code.

## Non-negotiables

**Accessibility regressions are never permitted.** No council vote, maintainer preference or
design compromise accepts one; fix it before anything is presented. A regression is any loss
against the baseline or [spec §15.4–15.5](../../../docs/spec/tooling.md#154-gui-workbench):

- text or essential non-text contrast below WCAG AA (4.5:1 text, 3:1 large text, controls,
  focus indicators and meaningful graphics), in light, dark or monochrome;
- an operation no longer reachable by keyboard, a weaker or hidden focus indicator, a changed
  focus order that no longer follows reading order, or focus not restored after an overlay;
- a control losing its accessible name, role or state, or a visual abbreviation without the full
  accessible name;
- a target below 24×24 logical pixels, or smaller than the baseline's;
- colour becoming the only cue; motion that ignores reduced motion; content clipped or overlapping
  at 200% interface scale or the 900×650 minimum window; text that no longer reflows;
- any gallery comparison entry under "Accessibility regressions".

Also hold the product contracts: behaviour that does not need a widget belongs in
`recite-writer-app`; drafts, history and external-editor boundaries in §15.5 stay intact; and the
standing decisions in `decisions.md` hold unless the maintainer reopens one.

## Brief for every participant

Give each participant this brief, in the templates' words, so their judgment is anchored the same
way:

- **Audience and job.** Narrative designers and writers authoring branching game dialogue, in long
  sessions beside a source editor and an engine. The job is reading, writing and navigating
  structure without friction.
- **Aesthetic.** Risography and physical print: two inks on paper, grain, halftone dot screens,
  print typography, hard offset shadows. Tactility should come from how surfaces and controls
  respond (ink pressing into paper, stock and plate, registration) rather than from added
  ornament. The identity README's ink and texture rules are constraints, not suggestions.
- **Long-session comfort.** Reading measure, line height, contrast that does not glare, stable
  spatial layout, quiet chrome, low visual noise in the manuscript, and dark mode that is not
  pure black.
- **Known-good patterns to cite.** Recognition over recall; visible system status; consistent
  rest, hover, pressed, focus and disabled states; Fitts's law for frequent targets; progressive
  disclosure; command palette and keyboard-first flows; undo over confirmation, confirmation only
  for loss; stable placement so muscle memory works; empty states that invite the next action;
  errors that say what happened and how to fix it; direct manipulation with clear affordances.
  Proposals should name the pattern they rely on.
- **Avoiding generic design.** Every choice should come from this product's subject and identity.
  Reject defaults that appear on any project regardless of brief: identical rounded cards with the
  same soft grey shadow, gradient washes, a tracked all-caps label above every heading, a single
  accented word in a heading, decorative numbering for content that is not a sequence, a mono face
  for every small label, arrows appended to every action, and scattered entrance or hover
  animations. Structure (rules, borders, numbering, labels) must carry information. Spend
  boldness in one place and keep the rest disciplined. Interface copy is plain, specific, in
  sentence case and active voice, and an action keeps one name throughout a flow. If the
  `frontend-design` skill is available in the session, load it as well; this list is the
  repository's portable version of the same discipline.

## Workflow

### 1. Prepare

1. Work on a design branch. Read `decisions.md`, the identity README and, if one exists, the last
   council entry.
2. Capture the starting point: `mise exec -- just writer gallery <before>` at 1x, and again at 2x
   (`mise exec -- just writer gallery <before-2x> '' 2`) for council viewing, since grain and dot
   screens need the resolution. Keep `<before>`: until a baseline is committed it is the
   comparison reference, passed as `RECITE_WRITER_GALLERY_BASELINE=<before>` on later captures.
   Run `cargo test --locked --manifest-path apps/writer/Cargo.toml -p recite-writer-session
   palette` for the contrast rows.
3. Read every frame's warnings. Fix a scene that did not reach its state before convening; a
   council reviewing the wrong state wastes the round.
4. Write a round folder in your scratchpad: the gallery paths, `report.md` if a baseline exists,
   the maintainer's request in their words, and any earlier feedback.

### 2. Convene the council

Spawn three fresh agents in one message with the Agent tool, `subagent_type: general-purpose`,
never `fork`, so each starts with a clean context and reaches its own view:

| Member | `model` | `effort` |
| ------ | ------- | -------- |
| A      | fable   | medium   |
| B      | opus    | high     |
| C      | sonnet  | high     |

These are currently Fable 5.2, Opus 5.5 and Sonnet 5.5; the maintainer may change the roster here.
If a model is unavailable, say so and continue with the others rather than substituting silently.
Use [council-brief.md](council-brief.md) as each prompt, filled with the round folder. Members
read images with the Read tool and must not edit files. Do not pass your own opinion of the design
into their briefs.

### 3. Discuss and converge

1. Collect the three reports. Label them A, B and C without model names when circulating.
2. Send each member the other two reports with SendMessage and ask for agreements, disagreements
   with reasons, and a revised proposal. Run at most two exchange rounds.
3. Synthesise one direction: what changes, which pattern justifies it, which screens it touches,
   the accessibility evidence it needs, and what was rejected and why. Record dissent rather than
   averaging it away. Where members split on a matter of taste, present both options to the
   maintainer instead of choosing silently.
4. Send the synthesis back; each member confirms or names a blocking objection. Resolve blocking
   objections before implementing.

### 4. Implement and iterate

1. Implement the direction in small, reviewable steps. Keep palette changes in the session crate
   with contrast rows and tests; keep state changes in the app crate.
2. Re-capture and compare. Fix every accessibility regression and every new frame finding before
   the council sees the result.
3. Send the members the new gallery, the comparison report and a summary of what changed. Each
   answers **presentable** or **not yet**, with blocking issues only. Iterate until all three say
   presentable. After four iterations without agreement, present anyway with the open objections
   stated.
4. Run `mise exec -- just writer check` before presenting.

### 5. Present to the maintainer

Publish a review page (an Artifact, or the contact sheet as a file when no publishing tool
exists) with before and after pairs for every changed frame in light and dark, the transition
strips, the direction and its reasons, rejected alternatives, recorded dissent, open taste
choices, and the accessibility evidence: the comparison report, palette tests, Tab walk, mono and
ui200 frames. Ask for sign-off or feedback, and wait. Silence, approval of a part, or an
automated message is not sign-off.

With feedback, return to step 3 with the same council: send them the feedback verbatim, converge,
implement, iterate and present again.

### 6. Sign-off and baseline

When the maintainer explicitly signs off:

1. Re-capture at 1x, run `mise exec -- just writer gallery-bless <out>`, and add a `decisions.md`
   log entry: date, request, direction, patterns relied on, rejected options, dissent, maintainer
   feedback, and accessibility evidence.
2. Commit the design change, `design/baseline/` and `decisions.md` together, following
   CONTRIBUTING's commit rules, and push to the working branch. Pull requests, reviews and merges
   need the maintainer's authorization as usual.

### 7. Regression review

1. Spawn one fresh agent: `subagent_type: general-purpose`, `model: opus`, `effort: high`, with
   [regression-brief.md](regression-brief.md). Give it the previous baseline commit, the new
   commit, both galleries, the comparison report and the code diff.
2. Discuss each finding with it through SendMessage. Argue from evidence; aim for compromises that
   keep the signed-off design's intent while restoring lost function. Accessibility findings are
   fixed, not negotiated.
3. If fixes change anything visible, re-capture, compare, show the maintainer the affected frames
   and ask whether the sign-off still holds. Then bless the baseline again and commit the fixes
   and the updated `decisions.md` entry.

## Closing

Report the baseline commit, what changed, what the regression review found and how each finding
was resolved, and any open taste choices or deferred ideas for the next pass.
