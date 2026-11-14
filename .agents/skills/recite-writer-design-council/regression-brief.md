# Regression reviewer brief

Fill the bracketed parts and send the whole brief as the agent's prompt.

---

The maintainer has signed off a new Recite Writer design. Your job is to find what it made worse:
UI and UX regressions against the previous design, with accessibility regressions above all else.
You are not re-litigating the new direction; you are protecting what worked. Do not edit files.

**Read first:** `.agents/skills/recite-writer-design-council/SKILL.md` ("Non-negotiables"),
`apps/writer/design/decisions.md` (including the new log entry explaining the intent), and
`apps/writer/design/README.md`.

**Evidence:**

- Previous baseline: commit [base SHA], `apps/writer/design/baseline/` (read files with
  `git show [base SHA]:path` or a worktree).
- New design: commit [new SHA], with its gallery at [1x folder] and [2x folder].
- Comparison: [report.md path] with highlighted pixel diffs under `diff/`.
- Code diff: `git diff [base SHA] [new SHA] -- apps/writer`.

**Look for:**

1. Accessibility: every item in Non-negotiables, checked in light, dark, mono, min and ui200, plus
   the Tab walk and control-state strips. Read `a11y/*.json` on both sides for names, roles,
   focusability, order and target sizes.
2. Function: anything harder to find, slower to reach, less visible in its state, or removed;
   lost keyboard paths; overlays that trap or lose focus; information no longer shown.
3. UX: weaker feedback for hover, pressed, focus, disabled, busy or error; inconsistent naming of
   actions; empty and error states that stopped directing the user; reading comfort lost in the
   manuscript; layout instability between workspaces.
4. Identity: breaches of the ink, texture and type rules.
5. Code: behaviour moved into the frontend that belongs in `recite-writer-app`, broken draft or
   history guarantees, palette colours without contrast rows or tests.

**Return** a list ordered by severity. For each finding: category (accessibility, function, UX,
identity, code), frames or files, what got worse compared with the baseline, why it matters to a
writer, and the smallest fix that keeps the new design's intent. Mark accessibility findings
**must fix**. End with what the new design improved, so fixes do not undo it. Expect follow-up
questions: the orchestrator will discuss each finding with you to agree on a compromise.
